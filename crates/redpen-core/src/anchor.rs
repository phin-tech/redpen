use crate::annotation::{Anchor, Annotation, Range};
use crate::hash::hash_string;
use strsim::normalized_levenshtein;

const REANCHOR_THRESHOLD: f64 = 0.25;

#[derive(Debug, PartialEq)]
pub enum AnchorResult {
    Exact { line: u32 },
    Fuzzy { line: u32, score: f64 },
    Orphaned,
}

pub fn resolve_anchor(anchor: &Anchor, source_lines: &[&str]) -> AnchorResult {
    let Anchor::TextContext {
        line_content, surrounding_lines, content_hash, last_known_line, ..
    } = anchor;

    let last_idx = (*last_known_line as usize).saturating_sub(1);
    if last_idx < source_lines.len() {
        let current_line = source_lines[last_idx];
        if hash_string(current_line) == *content_hash {
            return AnchorResult::Exact { line: *last_known_line };
        }
    }

    for (idx, line) in source_lines.iter().enumerate() {
        if hash_string(line) == *content_hash {
            return AnchorResult::Exact { line: (idx + 1) as u32 };
        }
    }

    let mut best_score: f64 = 0.0;
    let mut best_line: u32 = 0;

    for (idx, line) in source_lines.iter().enumerate() {
        let line_score = normalized_levenshtein(line_content.as_str(), line);
        let context_score = if !surrounding_lines.is_empty() {
            score_context(surrounding_lines, source_lines, idx)
        } else {
            0.0
        };
        let combined = line_score * 0.7 + context_score * 0.3;
        if combined > best_score {
            best_score = combined;
            best_line = (idx + 1) as u32;
        }
    }

    if best_score >= REANCHOR_THRESHOLD {
        AnchorResult::Fuzzy { line: best_line, score: best_score }
    } else {
        AnchorResult::Orphaned
    }
}

fn score_context(surrounding: &[String], source_lines: &[&str], center_idx: usize) -> f64 {
    if surrounding.is_empty() { return 0.0; }
    let center_in_surrounding = surrounding.len() / 2;
    let mut total_score = 0.0;
    let mut count = 0;
    for (i, ctx_line) in surrounding.iter().enumerate() {
        let offset = i as isize - center_in_surrounding as isize;
        let source_idx = center_idx as isize + offset;
        if source_idx >= 0 && (source_idx as usize) < source_lines.len() {
            total_score += normalized_levenshtein(ctx_line.as_str(), source_lines[source_idx as usize]);
            count += 1;
        }
    }
    if count > 0 { total_score / count as f64 } else { 0.0 }
}

pub fn reanchor_annotations(annotations: &mut [Annotation], source_content: &str) {
    let source_lines: Vec<&str> = source_content.lines().collect();
    for annotation in annotations.iter_mut() {
        let result = resolve_anchor(&annotation.anchor, &source_lines);
        match result {
            AnchorResult::Exact { line } | AnchorResult::Fuzzy { line, .. } => {
                annotation.is_orphaned = false;
                if let Anchor::TextContext { ref mut last_known_line, ref mut range, .. } = annotation.anchor {
                    let line_delta = line as i64 - range.start_line as i64;
                    range.start_line = line;
                    range.end_line = (range.end_line as i64 + line_delta) as u32;
                    *last_known_line = line;
                }
            }
            AnchorResult::Orphaned => {
                annotation.is_orphaned = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::*;

    fn make_anchor(line_content: &str, surrounding: Vec<&str>, line: u32) -> Anchor {
        Anchor::TextContext {
            line_content: line_content.to_string(),
            surrounding_lines: surrounding.iter().map(|s| s.to_string()).collect(),
            content_hash: hash_string(line_content),
            range: Range { start_line: line, start_column: 0, end_line: line, end_column: 10 },
            last_known_line: line,
        }
    }

    #[test]
    fn test_exact_match_at_same_position() {
        let anchor = make_anchor("fn main() {}", vec![], 1);
        let source = vec!["fn main() {}"];
        assert_eq!(resolve_anchor(&anchor, &source), AnchorResult::Exact { line: 1 });
    }

    #[test]
    fn test_exact_match_at_different_position() {
        let anchor = make_anchor("fn main() {}", vec![], 1);
        let source = vec!["use std;", "", "fn main() {}"];
        assert_eq!(resolve_anchor(&anchor, &source), AnchorResult::Exact { line: 3 });
    }

    #[test]
    fn test_fuzzy_match_after_rename() {
        let anchor = make_anchor("fn process_data(input: Vec<String>) {", vec![], 5);
        let source = vec!["use std;", "", "fn process_data(input: Vec<String>) -> Result<(), Error> {"];
        let result = resolve_anchor(&anchor, &source);
        match result {
            AnchorResult::Fuzzy { line, score } => {
                assert_eq!(line, 3);
                assert!(score >= REANCHOR_THRESHOLD);
            }
            other => panic!("Expected Fuzzy, got {:?}", other),
        }
    }

    #[test]
    fn test_orphaned_when_no_match() {
        let anchor = make_anchor("completely unique line xyz123", vec![], 1);
        let source = vec!["something totally different", "another line"];
        assert_eq!(resolve_anchor(&anchor, &source), AnchorResult::Orphaned);
    }

    #[test]
    fn test_context_improves_matching() {
        let anchor = make_anchor(
            "let x = 1;",
            vec!["fn foo() {", "    // setup", "let x = 1;", "    let y = 2;", "}"],
            3,
        );
        let source = vec!["fn foo() {", "    // setup", "let x = 1;", "    let y = 2;", "}"];
        let result = resolve_anchor(&anchor, &source);
        match result {
            AnchorResult::Exact { line } => assert_eq!(line, 3),
            other => panic!("Expected Exact, got {:?}", other),
        }
    }

    #[test]
    fn test_reanchor_annotations_updates_in_place() {
        let anchor = make_anchor("fn main() {}", vec![], 1);
        let mut annotations = vec![Annotation::new(
            AnnotationKind::Comment, "test".to_string(), vec![], "sam".to_string(), anchor,
        )];
        let source = "use std;\n\nfn main() {}";
        reanchor_annotations(&mut annotations, source);
        assert!(!annotations[0].is_orphaned);
        assert_eq!(annotations[0].line(), 3);
    }

    #[test]
    fn test_reanchor_marks_orphaned() {
        let anchor = make_anchor("this line was deleted entirely xyz", vec![], 1);
        let mut annotations = vec![Annotation::new(
            AnnotationKind::Comment, "test".to_string(), vec![], "sam".to_string(), anchor,
        )];
        reanchor_annotations(&mut annotations, "completely different file\nnothing matches");
        assert!(annotations[0].is_orphaned);
    }
}
