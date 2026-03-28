use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;
use uuid::Uuid;

/// Deserializes datetime from either ISO 8601 string or millisecond timestamp.
/// Provides compatibility with the Swift version of Red Pen.
fn flexible_datetime_deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(D::Error::custom),
        Some(serde_json::Value::Number(n)) => {
            if let Some(ms) = n.as_i64() {
                DateTime::from_timestamp_millis(ms)
                    .map(Some)
                    .ok_or_else(|| D::Error::custom("invalid timestamp"))
            } else {
                Err(D::Error::custom("expected integer timestamp"))
            }
        }
        _ => Err(D::Error::custom(
            "expected string, number, or null for datetime",
        )),
    }
}

/// Serializes datetime as ISO 8601 string to match Swift version format.
fn flexible_datetime_serialize<S>(
    value: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(dt) => serializer.serialize_str(&dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub enum AnnotationKind {
    Comment,
    LineNote,
    Label,
    Explanation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub struct Range {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub struct Annotation {
    pub id: String,
    pub kind: AnnotationKind,
    pub body: String,
    pub labels: Vec<String>,
    pub author: String,
    pub is_orphaned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(
        default,
        deserialize_with = "flexible_datetime_deserialize",
        serialize_with = "flexible_datetime_serialize",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "string | null")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(
        default,
        deserialize_with = "flexible_datetime_deserialize",
        serialize_with = "flexible_datetime_serialize",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "string | null")]
    pub updated_at: Option<DateTime<Utc>>,
    pub anchor: Anchor,
}

/// Summary of annotations for a single file, used for cross-file views.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub struct FileAnnotations {
    pub file_path: String,
    pub file_name: String,
    pub annotations: Vec<Annotation>,
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
            id: Uuid::new_v4().to_string().to_uppercase(),
            kind,
            body,
            labels,
            author,
            is_orphaned: false,
            reply_to: None,
            created_at: Some(now),
            updated_at: Some(now),
            anchor,
        }
    }

    pub fn new_reply(body: String, author: String, reply_to: String, anchor: Anchor) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string().to_uppercase(),
            kind: AnnotationKind::Comment,
            body,
            labels: vec![],
            author,
            is_orphaned: false,
            reply_to: Some(reply_to),
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
            surrounding_lines: vec![
                "use std;".to_string(),
                "".to_string(),
                "fn main() {}".to_string(),
            ],
            content_hash: "abc123".to_string(),
            range: Range {
                start_line: 3,
                start_column: 0,
                end_line: 3,
                end_column: 12,
            },
            last_known_line: 3,
        };
        let annotation = Annotation::new(
            AnnotationKind::Comment,
            "Test comment".to_string(),
            vec!["todo".to_string()],
            "sam".to_string(),
            anchor,
        );
        let json = serde_json::to_string(&annotation).unwrap();
        assert!(json.contains("\"kind\":\"comment\""));
        assert!(json.contains("\"isOrphaned\":false"));
        assert!(json.contains("\"startLine\":3"));
        assert!(json.contains("\"type\":\"textContext\""));
        assert!(json.contains("\"lineContent\":\"fn main() {}\""));
        // Dates should be ISO 8601 strings matching Swift format
        assert!(
            json.contains("\"createdAt\":\""),
            "createdAt should be a string"
        );
        assert!(json.contains("T"), "date should contain time separator");
        assert!(json.contains("Z\""), "date should end with Z");
        // ID should be uppercase UUID
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let id = parsed["id"].as_str().unwrap();
        assert_eq!(id, id.to_uppercase(), "ID should be uppercase UUID");
    }

    #[test]
    fn test_annotation_deserializes_from_millisecond_timestamps() {
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
    fn test_annotation_deserializes_from_swift_format() {
        let json = r#"{
            "id": "BBED6208-814E-4C92-B0BB-DA2175249BC0",
            "kind": "comment",
            "body": "test blah\n",
            "labels": [],
            "author": "sphinizy",
            "isOrphaned": false,
            "createdAt": "2026-03-21T00:24:22Z",
            "updatedAt": "2026-03-21T00:24:22Z",
            "anchor": {
                "type": "textContext",
                "lineContent": "requires-python = \">=3.13\"",
                "surroundingLines": ["description", "readme", "requires-python"],
                "contentHash": "b7a8689d",
                "range": { "startLine": 6, "startColumn": 11, "endLine": 8, "endColumn": 17 },
                "lastKnownLine": 6
            }
        }"#;
        let annotation: Annotation = serde_json::from_str(json).unwrap();
        assert_eq!(annotation.id, "BBED6208-814E-4C92-B0BB-DA2175249BC0");
        assert_eq!(annotation.author, "sphinizy");
        assert!(annotation.created_at.is_some());
        assert_eq!(annotation.line(), 6);
    }

    #[test]
    fn test_swift_format_roundtrip() {
        let json = r#"{
            "id": "BBED6208-814E-4C92-B0BB-DA2175249BC0",
            "kind": "comment",
            "body": "test blah\n",
            "labels": [],
            "author": "sphinizy",
            "isOrphaned": false,
            "createdAt": "2026-03-21T00:24:22Z",
            "updatedAt": "2026-03-21T00:24:22Z",
            "anchor": {
                "type": "textContext",
                "lineContent": "requires-python = \">=3.13\"",
                "surroundingLines": ["description", "readme", "requires-python"],
                "contentHash": "b7a8689d",
                "range": { "startLine": 6, "startColumn": 11, "endLine": 8, "endColumn": 17 },
                "lastKnownLine": 6
            }
        }"#;
        let annotation: Annotation = serde_json::from_str(json).unwrap();
        let reserialized = serde_json::to_string_pretty(&annotation).unwrap();
        // Dates should roundtrip as ISO 8601 strings
        assert!(reserialized.contains("\"createdAt\": \"2026-03-21T00:24:22Z\""));
        assert!(reserialized.contains("\"updatedAt\": \"2026-03-21T00:24:22Z\""));
        // ID should preserve original casing
        assert!(reserialized.contains("BBED6208-814E-4C92-B0BB-DA2175249BC0"));
    }

    #[test]
    fn test_line_returns_start_line() {
        let anchor = Anchor::TextContext {
            line_content: "let x = 1;".to_string(),
            surrounding_lines: vec![],
            content_hash: "abc".to_string(),
            range: Range {
                start_line: 42,
                start_column: 0,
                end_line: 42,
                end_column: 10,
            },
            last_known_line: 42,
        };
        let a = Annotation::new(
            AnnotationKind::LineNote,
            "note".into(),
            vec![],
            "sam".into(),
            anchor,
        );
        assert_eq!(a.line(), 42);
    }

    #[test]
    fn test_reply_to_none_by_default() {
        let anchor = Anchor::TextContext {
            line_content: "test".to_string(),
            surrounding_lines: vec![],
            content_hash: "abc".to_string(),
            range: Range {
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 4,
            },
            last_known_line: 1,
        };
        let a = Annotation::new(
            AnnotationKind::Comment,
            "note".into(),
            vec![],
            "sam".into(),
            anchor,
        );
        assert!(a.reply_to.is_none());
        // Should not appear in JSON when None
        let json = serde_json::to_string(&a).unwrap();
        assert!(!json.contains("replyTo"));
    }

    #[test]
    fn test_new_reply_sets_reply_to() {
        let anchor = Anchor::TextContext {
            line_content: "test".to_string(),
            surrounding_lines: vec![],
            content_hash: "abc".to_string(),
            range: Range {
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 4,
            },
            last_known_line: 1,
        };
        let parent_id = "PARENT-1234".to_string();
        let reply = Annotation::new_reply(
            "I fixed this".into(),
            "agent".into(),
            parent_id.clone(),
            anchor,
        );
        assert_eq!(reply.reply_to, Some(parent_id));
        assert_eq!(reply.kind, AnnotationKind::Comment);
        // Should appear in JSON
        let json = serde_json::to_string(&reply).unwrap();
        assert!(json.contains("\"replyTo\":\"PARENT-1234\""));
    }

    #[test]
    fn test_backward_compat_no_reply_to_field() {
        // Old JSON without replyTo should deserialize fine
        let json = r#"{
            "id": "TEST-ID",
            "kind": "comment",
            "body": "old annotation",
            "labels": [],
            "author": "sam",
            "isOrphaned": false,
            "anchor": {
                "type": "textContext",
                "lineContent": "test",
                "surroundingLines": [],
                "contentHash": "abc",
                "range": { "startLine": 1, "startColumn": 0, "endLine": 1, "endColumn": 4 },
                "lastKnownLine": 1
            }
        }"#;
        let a: Annotation = serde_json::from_str(json).unwrap();
        assert!(a.reply_to.is_none());
    }
}
