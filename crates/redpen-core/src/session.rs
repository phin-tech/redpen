use crate::annotation::{Annotation, FileAnnotations};
use crate::sidecar::{SidecarError, SidecarFile};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub struct GitSnapshot {
    pub head: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub struct SessionFileEntry {
    pub path: String,
    pub source_file_hash: String,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/lib/bindings/")]
pub struct SessionFile {
    pub version: u32,
    pub session_id: String,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<GitSnapshot>,
    #[ts(type = "Record<string, SessionFileEntry>")]
    pub files: HashMap<String, SessionFileEntry>,
}

impl SessionFile {
    pub fn new(session_id: String, git: Option<GitSnapshot>) -> Self {
        Self {
            version: 2,
            session_id,
            created_at: Utc::now(),
            verdict: None,
            git,
            files: HashMap::new(),
        }
    }

    /// MD5 hex hash of a normalized relative path (forward slashes, no `./` prefix).
    pub fn path_key(relative_path: &str) -> String {
        let normalized = relative_path
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string();
        format!("{:x}", md5::compute(normalized.as_bytes()))
    }

    // -- Path helpers --

    pub fn sessions_dir(project_root: &Path) -> PathBuf {
        project_root.join(".redpen").join("sessions")
    }

    pub fn session_path(project_root: &Path, session_id: &str) -> PathBuf {
        Self::sessions_dir(project_root).join(format!("{}.json", session_id))
    }

    pub fn active_session_path(project_root: &Path) -> PathBuf {
        Self::sessions_dir(project_root).join("active")
    }

    // -- Load / save --

    pub fn load(path: &Path) -> Result<Self, SidecarError> {
        let content = fs::read_to_string(path)?;
        let session: SessionFile = serde_json::from_str(&content)?;
        Ok(session)
    }

    pub fn load_by_id(project_root: &Path, session_id: &str) -> Result<Self, SidecarError> {
        Self::load(&Self::session_path(project_root, session_id))
    }

    pub fn save(&self, project_root: &Path) -> Result<(), SidecarError> {
        let path = Self::session_path(project_root, &self.session_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    // -- Active session pointer --

    pub fn read_active_session_id(project_root: &Path) -> Option<String> {
        let path = Self::active_session_path(project_root);
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    pub fn write_active_session_id(
        project_root: &Path,
        session_id: &str,
    ) -> Result<(), SidecarError> {
        let path = Self::active_session_path(project_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, session_id)?;
        Ok(())
    }

    /// Load the active session, or create a new one if none exists.
    /// If legacy `.redpen/comments/` exists and no active session, auto-migrates.
    pub fn load_or_create_active(
        project_root: &Path,
        git: Option<GitSnapshot>,
    ) -> Result<Self, SidecarError> {
        if let Some(id) = Self::read_active_session_id(project_root) {
            let path = Self::session_path(project_root, &id);
            if path.exists() {
                return Self::load(&path);
            }
        }

        // Check for legacy sidecars to migrate
        let legacy_dir = project_root.join(".redpen").join("comments");
        if legacy_dir.exists() {
            let session = Self::migrate_from_sidecars(project_root, git)?;
            session.save(project_root)?;
            Self::write_active_session_id(project_root, &session.session_id)?;
            return Ok(session);
        }

        // Brand new session
        let session = Self::new(Uuid::new_v4().to_string(), git);
        session.save(project_root)?;
        Self::write_active_session_id(project_root, &session.session_id)?;
        Ok(session)
    }

    // -- File entry helpers --

    pub fn get_or_create_entry(
        &mut self,
        relative_path: &str,
        source_file_hash: &str,
    ) -> &mut SessionFileEntry {
        let key = Self::path_key(relative_path);
        self.files.entry(key).or_insert_with(|| SessionFileEntry {
            path: relative_path.to_string(),
            source_file_hash: source_file_hash.to_string(),
            annotations: Vec::new(),
        })
    }

    pub fn get_entry(&self, relative_path: &str) -> Option<&SessionFileEntry> {
        let key = Self::path_key(relative_path);
        self.files.get(&key)
    }

    pub fn get_entry_mut(&mut self, relative_path: &str) -> Option<&mut SessionFileEntry> {
        let key = Self::path_key(relative_path);
        self.files.get_mut(&key)
    }

    // -- Annotation CRUD --

    pub fn add_annotation(&mut self, relative_path: &str, source_file_hash: &str, annotation: Annotation) {
        let entry = self.get_or_create_entry(relative_path, source_file_hash);
        entry.annotations.push(annotation);
    }

    pub fn remove_annotation(&mut self, annotation_id: &str) -> Option<Annotation> {
        for entry in self.files.values_mut() {
            if let Some(pos) = entry.annotations.iter().position(|a| a.id == annotation_id) {
                return Some(entry.annotations.remove(pos));
            }
        }
        None
    }

    pub fn get_annotation(&self, annotation_id: &str) -> Option<&Annotation> {
        for entry in self.files.values() {
            if let Some(a) = entry.annotations.iter().find(|a| a.id == annotation_id) {
                return Some(a);
            }
        }
        None
    }

    pub fn get_annotation_mut(&mut self, annotation_id: &str) -> Option<&mut Annotation> {
        for entry in self.files.values_mut() {
            if let Some(a) = entry.annotations.iter_mut().find(|a| a.id == annotation_id) {
                return Some(a);
            }
        }
        None
    }

    /// Convert a file entry to a SidecarFile for backward-compatible API responses.
    pub fn to_sidecar(&self, relative_path: &str) -> SidecarFile {
        match self.get_entry(relative_path) {
            Some(entry) => SidecarFile {
                version: 1,
                source_file_hash: entry.source_file_hash.clone(),
                annotations: entry.annotations.clone(),
                metadata: HashMap::new(),
            },
            None => SidecarFile::new(String::new()),
        }
    }

    /// Convert all entries to FileAnnotations for cross-file views.
    pub fn to_file_annotations(&self) -> Vec<FileAnnotations> {
        self.files
            .values()
            .filter(|entry| !entry.annotations.is_empty())
            .map(|entry| {
                let file_name = Path::new(&entry.path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                FileAnnotations {
                    file_path: entry.path.clone(),
                    file_name,
                    annotations: entry.annotations.clone(),
                }
            })
            .collect()
    }

    // -- Migration --

    /// Migrate all legacy `.redpen/comments/` sidecars into a new session file.
    pub fn migrate_from_sidecars(
        project_root: &Path,
        git: Option<GitSnapshot>,
    ) -> Result<Self, SidecarError> {
        let comments_dir = project_root.join(".redpen").join("comments");
        let mut session = Self::new(Uuid::new_v4().to_string(), git);

        if !comments_dir.exists() {
            return Ok(session);
        }

        Self::collect_legacy_sidecars(&comments_dir, project_root, &mut session)?;
        Ok(session)
    }

    fn collect_legacy_sidecars(
        dir: &Path,
        project_root: &Path,
        session: &mut SessionFile,
    ) -> Result<(), SidecarError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::collect_legacy_sidecars(&path, project_root, session)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let sidecar = SidecarFile::load(&path)?;
                if sidecar.annotations.is_empty() {
                    continue;
                }
                // Reconstruct relative path from sidecar path:
                // .redpen/comments/src/foo.rs.json -> src/foo.rs
                let comments_dir = project_root.join(".redpen").join("comments");
                if let Ok(rel) = path.strip_prefix(&comments_dir) {
                    let rel_str = rel.to_string_lossy();
                    // Strip the trailing .json to get the original file path
                    let source_rel = rel_str.trim_end_matches(".json");
                    let key = Self::path_key(source_rel);
                    session.files.insert(
                        key,
                        SessionFileEntry {
                            path: source_rel.to_string(),
                            source_file_hash: sidecar.source_file_hash,
                            annotations: sidecar.annotations,
                        },
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::*;

    fn make_test_annotation(line: u32, body: &str) -> Annotation {
        Annotation::new(
            AnnotationKind::Comment,
            body.to_string(),
            vec![],
            "test".to_string(),
            Anchor::TextContext {
                line_content: "test line".to_string(),
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
        )
    }

    #[test]
    fn test_path_key_normalizes() {
        assert_eq!(
            SessionFile::path_key("src/app.swift"),
            SessionFile::path_key("./src/app.swift")
        );
        assert_eq!(
            SessionFile::path_key("src/app.swift"),
            SessionFile::path_key("src\\app.swift")
        );
    }

    #[test]
    fn test_path_key_different_files_differ() {
        assert_ne!(
            SessionFile::path_key("src/a.rs"),
            SessionFile::path_key("src/b.rs")
        );
    }

    #[test]
    fn test_roundtrip_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        let mut session = SessionFile::new("test-session-1".to_string(), None);
        session.add_annotation("src/main.rs", "hash1", make_test_annotation(1, "first"));
        session.add_annotation("src/lib.rs", "hash2", make_test_annotation(5, "second"));
        session.save(project_root).unwrap();

        let loaded = SessionFile::load_by_id(project_root, "test-session-1").unwrap();
        assert_eq!(loaded.version, 2);
        assert_eq!(loaded.session_id, "test-session-1");
        assert_eq!(loaded.files.len(), 2);

        let main_entry = loaded.get_entry("src/main.rs").unwrap();
        assert_eq!(main_entry.annotations.len(), 1);
        assert_eq!(main_entry.annotations[0].body, "first");

        let lib_entry = loaded.get_entry("src/lib.rs").unwrap();
        assert_eq!(lib_entry.annotations.len(), 1);
        assert_eq!(lib_entry.annotations[0].body, "second");
    }

    #[test]
    fn test_active_session_pointer() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        assert!(SessionFile::read_active_session_id(project_root).is_none());

        SessionFile::write_active_session_id(project_root, "abc-123").unwrap();
        assert_eq!(
            SessionFile::read_active_session_id(project_root).unwrap(),
            "abc-123"
        );
    }

    #[test]
    fn test_load_or_create_active_new() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        let session = SessionFile::load_or_create_active(project_root, None).unwrap();
        assert_eq!(session.version, 2);
        assert!(!session.session_id.is_empty());
        assert!(session.files.is_empty());

        // Should persist the active pointer
        let active_id = SessionFile::read_active_session_id(project_root).unwrap();
        assert_eq!(active_id, session.session_id);
    }

    #[test]
    fn test_load_or_create_active_existing() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        let mut session = SessionFile::new("existing-session".to_string(), None);
        session.add_annotation("src/a.rs", "h", make_test_annotation(1, "hello"));
        session.save(project_root).unwrap();
        SessionFile::write_active_session_id(project_root, "existing-session").unwrap();

        let loaded = SessionFile::load_or_create_active(project_root, None).unwrap();
        assert_eq!(loaded.session_id, "existing-session");
        assert_eq!(loaded.files.len(), 1);
    }

    #[test]
    fn test_remove_annotation() {
        let mut session = SessionFile::new("s1".to_string(), None);
        let ann = make_test_annotation(1, "removeme");
        let id = ann.id.clone();
        session.add_annotation("src/a.rs", "h", ann);

        assert!(session.get_annotation(&id).is_some());
        let removed = session.remove_annotation(&id).unwrap();
        assert_eq!(removed.body, "removeme");
        assert!(session.get_annotation(&id).is_none());
    }

    #[test]
    fn test_to_sidecar() {
        let mut session = SessionFile::new("s1".to_string(), None);
        session.add_annotation("src/a.rs", "hash1", make_test_annotation(1, "comment"));

        let sidecar = session.to_sidecar("src/a.rs");
        assert_eq!(sidecar.source_file_hash, "hash1");
        assert_eq!(sidecar.annotations.len(), 1);

        let empty = session.to_sidecar("src/nonexistent.rs");
        assert!(empty.annotations.is_empty());
    }

    #[test]
    fn test_to_file_annotations() {
        let mut session = SessionFile::new("s1".to_string(), None);
        session.add_annotation("src/a.rs", "h1", make_test_annotation(1, "a"));
        session.add_annotation("src/b.rs", "h2", make_test_annotation(2, "b"));

        let fa = session.to_file_annotations();
        assert_eq!(fa.len(), 2);
    }

    #[test]
    fn test_migrate_from_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        // Create legacy sidecar files
        let comments_dir = project_root.join(".redpen").join("comments").join("src");
        fs::create_dir_all(&comments_dir).unwrap();

        let mut sidecar = SidecarFile::new("hash1".to_string());
        sidecar.add_annotation(make_test_annotation(1, "legacy comment"));
        let sidecar_path = comments_dir.join("main.rs.json");
        sidecar.save(&sidecar_path).unwrap();

        let session = SessionFile::migrate_from_sidecars(project_root, None).unwrap();
        assert_eq!(session.files.len(), 1);

        let entry = session.get_entry("src/main.rs").unwrap();
        assert_eq!(entry.source_file_hash, "hash1");
        assert_eq!(entry.annotations.len(), 1);
        assert_eq!(entry.annotations[0].body, "legacy comment");
    }

    #[test]
    fn test_git_snapshot_serialization() {
        let session = SessionFile::new(
            "s1".to_string(),
            Some(GitSnapshot {
                head: "abc123".to_string(),
                dirty: true,
            }),
        );
        let json = serde_json::to_string_pretty(&session).unwrap();
        assert!(json.contains("\"head\": \"abc123\""));
        assert!(json.contains("\"dirty\": true"));

        let loaded: SessionFile = serde_json::from_str(&json).unwrap();
        let git = loaded.git.unwrap();
        assert_eq!(git.head, "abc123");
        assert!(git.dirty);
    }

    #[test]
    fn test_verdict() {
        let mut session = SessionFile::new("s1".to_string(), None);
        assert!(session.verdict.is_none());

        session.verdict = Some("approved".to_string());
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("\"verdict\":\"approved\""));
    }
}
