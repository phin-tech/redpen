use crate::annotation::Annotation;
use crate::sidecar::SidecarFile;
use std::path::Path;

pub fn export_markdown(sidecar: &SidecarFile, source_content: &str, file_name: &str) -> String {
    let source_lines: Vec<&str> = source_content.lines().collect();
    let lang = lang_hint(file_name);

    let (active, orphaned): (Vec<&Annotation>, Vec<&Annotation>) = sidecar
        .annotations_sorted_by_line()
        .into_iter()
        .partition(|a| !a.is_orphaned);

    let mut annotations_by_line: std::collections::BTreeMap<u32, Vec<&Annotation>> =
        std::collections::BTreeMap::new();
    for a in &active {
        annotations_by_line.entry(a.line()).or_default().push(a);
    }

    let mut out = String::new();
    out.push_str(&format!("## {}\n\n", file_name));

    for (idx, line) in source_lines.iter().enumerate() {
        let line_num = (idx + 1) as u32;
        out.push_str(&format!("```{}\n{}\n```\n\n", lang, line));
        if let Some(line_annotations) = annotations_by_line.get(&line_num) {
            for a in line_annotations {
                out.push_str(&format_annotation_blockquote(a));
                out.push('\n');
            }
        }
    }

    if !orphaned.is_empty() {
        out.push_str("## Unresolved Annotations\n\n");
        for a in orphaned {
            out.push_str(&format_annotation_blockquote(a));
            out.push('\n');
        }
    }

    out.trim_end().to_string() + "\n"
}

fn format_annotation_blockquote(a: &Annotation) -> String {
    let labels = if a.labels.is_empty() {
        String::new()
    } else {
        format!("**[{}]** ", a.labels.join(", "))
    };
    format!("> {}{} — *{}*\n", labels, a.body, a.author)
}

fn lang_hint(file_name: &str) -> &str {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => "rust",
        "swift" => "swift",
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rb" => "ruby",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "css" => "css",
        "html" | "htm" => "html",
        "json" => "json",
        "md" => "markdown",
        "sh" | "bash" | "zsh" => "bash",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "sql" => "sql",
        "ex" | "exs" => "elixir",
        _ => ext,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::*;

    fn make_annotation(line: u32, body: &str, labels: Vec<&str>, orphaned: bool) -> Annotation {
        let mut a = Annotation::new(
            AnnotationKind::Comment,
            body.to_string(),
            labels.iter().map(|s| s.to_string()).collect(),
            "sam".to_string(),
            Anchor::TextContext {
                line_content: "test".to_string(),
                surrounding_lines: vec![],
                content_hash: "abc".to_string(),
                range: Range {
                    start_line: line,
                    start_column: 0,
                    end_line: line,
                    end_column: 10,
                },
                last_known_line: line,
            },
        );
        a.is_orphaned = orphaned;
        a
    }

    #[test]
    fn test_basic_export() {
        let mut sidecar = SidecarFile::new("hash".to_string());
        sidecar.add_annotation(make_annotation(1, "Nice function", vec![], false));
        let source = "fn main() {}\nlet x = 1;";
        let result = export_markdown(&sidecar, source, "test.rs");
        assert!(result.contains("## test.rs"));
        assert!(result.contains("```rust\nfn main() {}\n```"));
        assert!(result.contains("> Nice function — *sam*"));
    }

    #[test]
    fn test_export_with_labels() {
        let mut sidecar = SidecarFile::new("hash".to_string());
        sidecar.add_annotation(make_annotation(1, "Fix this", vec!["bug", "urgent"], false));
        let result = export_markdown(&sidecar, "let x = 1;", "test.js");
        assert!(result.contains("> **[bug, urgent]** Fix this — *sam*"));
    }

    #[test]
    fn test_export_orphaned_section() {
        let mut sidecar = SidecarFile::new("hash".to_string());
        sidecar.add_annotation(make_annotation(1, "Active", vec![], false));
        sidecar.add_annotation(make_annotation(99, "Lost", vec!["bug"], true));
        let result = export_markdown(&sidecar, "line one", "test.py");
        assert!(result.contains("## Unresolved Annotations"));
        assert!(result.contains("> **[bug]** Lost — *sam*"));
    }

    #[test]
    fn test_lang_hint() {
        assert_eq!(lang_hint("app.swift"), "swift");
        assert_eq!(lang_hint("main.rs"), "rust");
        assert_eq!(lang_hint("index.ts"), "typescript");
        assert_eq!(lang_hint("unknown.xyz"), "xyz");
    }
}
