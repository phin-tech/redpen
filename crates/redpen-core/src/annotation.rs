use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationKind {
    Comment,
    LineNote,
    Label,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Anchor {
    #[serde(rename_all = "camelCase")]
    TextContext {
        line_content: String,
        surrounding_lines: Vec<String>,
        content_hash: String,
        range: Range,
        last_known_line: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: String,
    pub kind: AnnotationKind,
    pub body: String,
    pub labels: Vec<String>,
    pub author: String,
    pub is_orphaned: bool,
    #[serde(with = "chrono::serde::ts_milliseconds_option", default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_milliseconds_option", default)]
    pub updated_at: Option<DateTime<Utc>>,
    pub anchor: Anchor,
}

impl Annotation {
    pub fn new(
        kind: AnnotationKind,
        body: String,
        labels: Vec<String>,
        author: String,
        anchor: Anchor,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            body,
            labels,
            author,
            is_orphaned: false,
            created_at: Some(now),
            updated_at: Some(now),
            anchor,
        }
    }

    pub fn line(&self) -> u32 {
        match &self.anchor {
            Anchor::TextContext { range, .. } => range.start_line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_serializes_to_camel_case() {
        let anchor = Anchor::TextContext {
            line_content: "fn main() {}".to_string(),
            surrounding_lines: vec!["use std;".to_string(), "".to_string(), "fn main() {}".to_string()],
            content_hash: "abc123".to_string(),
            range: Range { start_line: 3, start_column: 0, end_line: 3, end_column: 12 },
            last_known_line: 3,
        };
        let annotation = Annotation::new(
            AnnotationKind::Comment, "Test comment".to_string(),
            vec!["todo".to_string()], "sam".to_string(), anchor,
        );
        let json = serde_json::to_string(&annotation).unwrap();
        assert!(json.contains("\"kind\":\"comment\""));
        assert!(json.contains("\"isOrphaned\":false"));
        assert!(json.contains("\"startLine\":3"));
        assert!(json.contains("\"type\":\"textContext\""));
        assert!(json.contains("\"lineContent\":\"fn main() {}\""));
    }

    #[test]
    fn test_annotation_deserializes_from_sidecar_format() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "kind": "comment",
            "body": "This is complex",
            "labels": ["refactor"],
            "author": "sam",
            "isOrphaned": false,
            "createdAt": 1710945000000,
            "updatedAt": 1710945000000,
            "anchor": {
                "type": "textContext",
                "lineContent": "func processData()",
                "surroundingLines": ["import Foundation", "", "func processData()"],
                "contentHash": "deadbeef",
                "range": { "startLine": 3, "startColumn": 5, "endLine": 3, "endColumn": 48 },
                "lastKnownLine": 3
            }
        }"#;
        let annotation: Annotation = serde_json::from_str(json).unwrap();
        assert_eq!(annotation.kind, AnnotationKind::Comment);
        assert_eq!(annotation.labels, vec!["refactor"]);
        assert_eq!(annotation.line(), 3);
    }

    #[test]
    fn test_line_returns_start_line() {
        let anchor = Anchor::TextContext {
            line_content: "let x = 1;".to_string(),
            surrounding_lines: vec![],
            content_hash: "abc".to_string(),
            range: Range { start_line: 42, start_column: 0, end_line: 42, end_column: 10 },
            last_known_line: 42,
        };
        let a = Annotation::new(AnnotationKind::LineNote, "note".into(), vec![], "sam".into(), anchor);
        assert_eq!(a.line(), 42);
    }
}
