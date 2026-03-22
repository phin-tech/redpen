use crate::annotation::Annotation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarFile {
    pub version: u32,
    pub source_file_hash: String,
    pub annotations: Vec<Annotation>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SidecarFile {
    pub fn new(source_file_hash: String) -> Self {
        Self { version: 1, source_file_hash, annotations: Vec::new(), metadata: HashMap::new() }
    }

    pub fn sidecar_path(source_path: &Path) -> PathBuf {
        let file_name = source_path.file_name().unwrap().to_string_lossy();
        source_path.with_file_name(format!("{}.redpen.json", file_name))
    }

    pub fn signal_path(source_path: &Path) -> PathBuf {
        let file_name = source_path.file_name().unwrap().to_string_lossy();
        source_path.with_file_name(format!("{}.redpen-signal", file_name))
    }

    pub fn load(sidecar_path: &Path) -> Result<Self, SidecarError> {
        let content = fs::read_to_string(sidecar_path)?;
        let sidecar: SidecarFile = serde_json::from_str(&content)?;
        Ok(sidecar)
    }

    pub fn load_for_source(source_path: &Path) -> Result<Self, SidecarError> {
        let sidecar_path = Self::sidecar_path(source_path);
        Self::load(&sidecar_path)
    }

    pub fn save(&self, sidecar_path: &Path) -> Result<(), SidecarError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(sidecar_path, content)?;
        Ok(())
    }

    pub fn save_for_source(&self, source_path: &Path) -> Result<(), SidecarError> {
        let sidecar_path = Self::sidecar_path(source_path);
        self.save(&sidecar_path)
    }

    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    pub fn remove_annotation(&mut self, id: &str) -> Option<Annotation> {
        if let Some(pos) = self.annotations.iter().position(|a| a.id == id) {
            Some(self.annotations.remove(pos))
        } else {
            None
        }
    }

    pub fn get_annotation(&self, id: &str) -> Option<&Annotation> {
        self.annotations.iter().find(|a| a.id == id)
    }

    pub fn get_annotation_mut(&mut self, id: &str) -> Option<&mut Annotation> {
        self.annotations.iter_mut().find(|a| a.id == id)
    }

    pub fn annotations_sorted_by_line(&self) -> Vec<&Annotation> {
        let mut sorted: Vec<&Annotation> = self.annotations.iter().collect();
        sorted.sort_by_key(|a| a.line());
        sorted
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::*;

    fn make_test_annotation(line: u32, body: &str) -> Annotation {
        Annotation::new(
            AnnotationKind::Comment, body.to_string(), vec![], "test".to_string(),
            Anchor::TextContext {
                line_content: "test line".to_string(), surrounding_lines: vec![],
                content_hash: "abc".to_string(),
                range: Range { start_line: line, start_column: 0, end_line: line, end_column: 10 },
                last_known_line: line,
            },
        )
    }

    #[test]
    fn test_sidecar_path() {
        let source = Path::new("/code/app.swift");
        assert_eq!(SidecarFile::sidecar_path(source), PathBuf::from("/code/app.swift.redpen.json"));
    }

    #[test]
    fn test_roundtrip_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("test.rs");
        fs::write(&source_path, "fn main() {}").unwrap();
        let mut sidecar = SidecarFile::new("hash123".to_string());
        sidecar.add_annotation(make_test_annotation(1, "first comment"));
        sidecar.add_annotation(make_test_annotation(5, "second comment"));
        sidecar.save_for_source(&source_path).unwrap();
        let loaded = SidecarFile::load_for_source(&source_path).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.source_file_hash, "hash123");
        assert_eq!(loaded.annotations.len(), 2);
        assert_eq!(loaded.annotations[0].body, "first comment");
    }

    #[test]
    fn test_remove_annotation() {
        let mut sidecar = SidecarFile::new("hash".to_string());
        let a = make_test_annotation(1, "to remove");
        let id = a.id.clone();
        sidecar.add_annotation(a);
        assert!(sidecar.remove_annotation(&id).is_some());
        assert_eq!(sidecar.annotations.len(), 0);
        assert!(sidecar.remove_annotation(&id).is_none());
    }

    #[test]
    fn test_sorted_by_line() {
        let mut sidecar = SidecarFile::new("hash".to_string());
        sidecar.add_annotation(make_test_annotation(10, "later"));
        sidecar.add_annotation(make_test_annotation(1, "earlier"));
        sidecar.add_annotation(make_test_annotation(5, "middle"));
        let sorted = sidecar.annotations_sorted_by_line();
        assert_eq!(sorted[0].body, "earlier");
        assert_eq!(sorted[1].body, "middle");
        assert_eq!(sorted[2].body, "later");
    }

    #[test]
    fn test_json_matches_sidecar_format() {
        let sidecar = SidecarFile::new("abc123".to_string());
        let json = serde_json::to_string_pretty(&sidecar).unwrap();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"sourceFileHash\": \"abc123\""));
        assert!(json.contains("\"annotations\": []"));
    }

    #[test]
    fn test_load_swift_sidecar_fixture() {
        let fixture = include_str!("../tests/fixtures/swift-sidecar.json");
        let sidecar: SidecarFile = serde_json::from_str(fixture).unwrap();
        assert_eq!(sidecar.version, 1);
        assert_eq!(sidecar.annotations.len(), 2);

        let first = &sidecar.annotations[0];
        assert_eq!(first.id, "BBED6208-814E-4C92-B0BB-DA2175249BC0");
        assert_eq!(first.author, "sphinizy");
        assert_eq!(first.kind, AnnotationKind::Comment);
        assert!(first.created_at.is_some());
        assert_eq!(first.line(), 6);

        let second = &sidecar.annotations[1];
        assert_eq!(second.id, "2AD8E871-F538-45C1-8F58-9C9A457CA706");
        assert_eq!(second.body, "lets switch to click");
        assert_eq!(second.line(), 10);

        // Roundtrip: re-serialize and verify dates stay as ISO 8601
        let reserialized = serde_json::to_string_pretty(&sidecar).unwrap();
        assert!(reserialized.contains("\"createdAt\": \"2026-03-21T00:24:22Z\""));
        assert!(reserialized.contains("\"createdAt\": \"2026-03-21T01:34:21Z\""));
        // Re-parse to confirm full roundtrip
        let reloaded: SidecarFile = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(reloaded.annotations.len(), 2);
        assert_eq!(reloaded.annotations[0].id, first.id);
        assert_eq!(reloaded.annotations[1].id, second.id);
    }
}
