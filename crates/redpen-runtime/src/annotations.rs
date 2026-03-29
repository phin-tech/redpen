use std::fs;
use std::path::Path;

use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind, Choice, FileAnnotations};
use redpen_core::hash::hash_file;
use redpen_core::sidecar::SidecarFile;

use crate::error::RuntimeError;
use crate::event_bus::{AppEvent, EventBus};

pub struct AnnotationService<E: EventBus> {
    event_bus: E,
}

impl<E: EventBus> AnnotationService<E> {
    pub fn new(event_bus: E) -> Self {
        Self { event_bus }
    }

    pub fn get_annotations(
        &self,
        source_path: &Path,
        project_root: &Path,
    ) -> Result<SidecarFile, RuntimeError> {
        load_sidecar_for_file(project_root, source_path)
    }

    pub fn get_all_annotations(
        &self,
        dir: &Path,
        project_root: &Path,
    ) -> Result<Vec<FileAnnotations>, RuntimeError> {
        let mut results = Vec::new();
        let comments_dir = dir.join(".redpen").join("comments");
        if comments_dir.exists() {
            collect_sidecar_files(&comments_dir, project_root, &mut results)?;
        }
        Ok(results)
    }

    pub fn create_annotation(
        &self,
        project_root: &Path,
        source_path: &Path,
        body: &str,
        labels: Vec<String>,
        author: &str,
        anchor: Anchor,
    ) -> Result<Annotation, RuntimeError> {
        self.create_annotation_full(
            project_root,
            source_path,
            AnnotationKind::Comment,
            body,
            labels,
            author,
            anchor,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_annotation_full(
        &self,
        project_root: &Path,
        source_path: &Path,
        kind: AnnotationKind,
        body: &str,
        labels: Vec<String>,
        author: &str,
        anchor: Anchor,
        reply_to: Option<String>,
    ) -> Result<Annotation, RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        let annotation = if let Some(parent_id) = reply_to {
            Annotation::new_reply(body.to_string(), author.to_string(), parent_id, anchor)
        } else {
            Annotation::new(kind, body.to_string(), labels, author.to_string(), anchor)
        };
        let result = annotation.clone();
        sidecar.add_annotation(annotation);
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(result)
    }

    pub fn update_annotation(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
        body: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> Result<Annotation, RuntimeError> {
        self.update_annotation_full(project_root, source_path, id, body, labels, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_annotation_full(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
        body: Option<&str>,
        labels: Option<Vec<String>>,
        choices: Option<Vec<Choice>>,
        resolved: Option<bool>,
    ) -> Result<Annotation, RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        let annotation = sidecar
            .get_annotation_mut(id)
            .ok_or_else(|| RuntimeError::NotFound(format!("annotation {}", id)))?;
        if let Some(body) = body {
            annotation.body = body.to_string();
        }
        if let Some(labels) = labels {
            annotation.labels = labels;
        }
        if let Some(choices) = choices {
            annotation.choices = Some(choices);
        }
        if let Some(r) = resolved {
            annotation.resolved = r;
        }
        annotation.updated_at = Some(chrono::Utc::now());
        let result = annotation.clone();
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(result)
    }

    pub fn delete_annotation(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
    ) -> Result<(), RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar
            .remove_annotation(id)
            .ok_or_else(|| RuntimeError::NotFound(format!("annotation {}", id)))?;
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(())
    }

    pub fn clear_annotations(
        &self,
        project_root: &Path,
        source_path: &Path,
    ) -> Result<(), RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar.annotations.clear();
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(())
    }

    // -- Session-based methods --

    pub fn get_annotations_from_session(
        &self,
        source_path: &Path,
        project_root: &Path,
    ) -> Result<SidecarFile, RuntimeError> {
        load_sidecar_for_file(project_root, source_path)
    }

    pub fn get_all_annotations_from_session(
        &self,
        project_root: &Path,
    ) -> Result<Vec<FileAnnotations>, RuntimeError> {
        self.get_all_annotations(project_root, project_root)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_annotation_in_session(
        &self,
        project_root: &Path,
        source_path: &Path,
        kind: AnnotationKind,
        body: &str,
        labels: Vec<String>,
        author: &str,
        anchor: Anchor,
        reply_to: Option<String>,
    ) -> Result<Annotation, RuntimeError> {
        let annotation = if let Some(parent_id) = reply_to {
            Annotation::new_reply(body.to_string(), author.to_string(), parent_id, anchor)
        } else {
            Annotation::new(kind, body.to_string(), labels, author.to_string(), anchor)
        };
        let result = annotation.clone();
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar.add_annotation(annotation);
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_annotation_in_session(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
        body: Option<&str>,
        labels: Option<Vec<String>>,
        choices: Option<Vec<Choice>>,
        resolved: Option<bool>,
    ) -> Result<Annotation, RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        let annotation = sidecar
            .get_annotation_mut(id)
            .ok_or_else(|| RuntimeError::NotFound(format!("annotation {}", id)))?;
        if let Some(body) = body {
            annotation.body = body.to_string();
        }
        if let Some(labels) = labels {
            annotation.labels = labels;
        }
        if let Some(choices) = choices {
            annotation.choices = Some(choices);
        }
        if let Some(r) = resolved {
            annotation.resolved = r;
        }
        annotation.updated_at = Some(chrono::Utc::now());
        let result = annotation.clone();
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(result)
    }

    pub fn delete_annotation_from_session(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
    ) -> Result<(), RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar
            .remove_annotation(id)
            .ok_or_else(|| RuntimeError::NotFound(format!("annotation {}", id)))?;
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(())
    }

    pub fn clear_annotations_in_session(
        &self,
        project_root: &Path,
        source_path: &Path,
    ) -> Result<(), RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar.annotations.clear();
        save_sidecar(&sidecar, project_root, source_path)?;
        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });
        Ok(())
    }
}

fn load_sidecar_for_file(
    project_root: &Path,
    source_path: &Path,
) -> Result<SidecarFile, RuntimeError> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if annotation_path.exists() {
        let mut sidecar = SidecarFile::load(&annotation_path)?;
        let current_hash = hash_file(source_path)?;
        if sidecar.source_file_hash != current_hash {
            let content = fs::read_to_string(source_path)?;
            reanchor_annotations(&mut sidecar.annotations, &content);
            sidecar.source_file_hash = current_hash;
        }
        Ok(sidecar)
    } else {
        let hash = hash_file(source_path)?;
        Ok(SidecarFile::new(hash))
    }
}

fn save_sidecar(
    sidecar: &SidecarFile,
    project_root: &Path,
    source_path: &Path,
) -> Result<(), RuntimeError> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if sidecar.annotations.is_empty() {
        if annotation_path.exists() {
            fs::remove_file(&annotation_path)?;
        }
        Ok(())
    } else {
        sidecar.save(&annotation_path)?;
        Ok(())
    }
}

fn collect_sidecar_files(
    dir: &Path,
    project_root: &Path,
    results: &mut Vec<FileAnnotations>,
) -> Result<(), RuntimeError> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_sidecar_files(&path, project_root, results)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            if let Ok(sidecar) = SidecarFile::load(&path) {
                if !sidecar.annotations.is_empty() {
                    let relative = path
                        .strip_prefix(project_root.join(".redpen").join("comments"))
                        .unwrap_or(&path);
                    let source_relative = relative.with_file_name(
                        relative
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .trim_end_matches(".json"),
                    );
                    let source_path = project_root.join(&source_relative);
                    let file_name = source_relative
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    results.push(FileAnnotations {
                        file_path: source_path.to_string_lossy().to_string(),
                        file_name,
                        annotations: sidecar.annotations,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::{AppEvent, EventBus};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    struct RecordingEventBus {
        events: Arc<Mutex<Vec<AppEvent>>>,
    }

    impl RecordingEventBus {
        fn new() -> (Self, Arc<Mutex<Vec<AppEvent>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    events: events.clone(),
                },
                events,
            )
        }
    }

    impl EventBus for RecordingEventBus {
        fn emit(&self, event: AppEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn setup_project(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.rs");
        std::fs::write(&source, content).unwrap();
        (dir, source)
    }

    fn make_anchor(line_content: &str) -> Anchor {
        Anchor::TextContext {
            line_content: line_content.to_string(),
            surrounding_lines: vec![line_content.to_string()],
            content_hash: redpen_core::hash::hash_string(line_content),
            range: redpen_core::annotation::Range {
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: line_content.len() as u32,
            },
            last_known_line: 1,
        }
    }

    #[test]
    fn get_annotations_returns_empty_sidecar_for_new_file() {
        let (bus, _events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);
        let (dir, source) = setup_project("fn main() {}");

        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert!(sidecar.annotations.is_empty());
        assert_eq!(sidecar.version, 1);
    }

    #[test]
    fn create_annotation_saves_and_emits_event() {
        let (bus, events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);
        let (dir, source) = setup_project("fn main() {}");
        let anchor = make_anchor("fn main() {}");

        let annotation = service
            .create_annotation(
                dir.path(),
                &source,
                "looks good",
                vec!["review".to_string()],
                "alice",
                anchor,
            )
            .unwrap();

        assert_eq!(annotation.body, "looks good");
        assert_eq!(annotation.labels, vec!["review"]);
        assert_eq!(annotation.author, "alice");

        // Verify persisted
        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert_eq!(sidecar.annotations.len(), 1);
        assert_eq!(sidecar.annotations[0].id, annotation.id);

        // Verify event emitted
        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        match &recorded[0] {
            AppEvent::AnnotationsChanged { file_path } => {
                assert!(file_path.contains("test.rs"));
            }
            other => panic!("expected AnnotationsChanged, got {:?}", other),
        }
    }

    #[test]
    fn update_annotation_modifies_body_and_labels() {
        let (bus, _events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);
        let (dir, source) = setup_project("fn main() {}");
        let anchor = make_anchor("fn main() {}");

        let created = service
            .create_annotation(dir.path(), &source, "original", vec![], "alice", anchor)
            .unwrap();

        let updated = service
            .update_annotation(
                dir.path(),
                &source,
                &created.id,
                Some("revised"),
                Some(vec!["important".to_string()]),
            )
            .unwrap();

        assert_eq!(updated.body, "revised");
        assert_eq!(updated.labels, vec!["important"]);
        assert!(updated.updated_at > created.updated_at);
    }

    #[test]
    fn delete_annotation_removes_and_emits_event() {
        let (bus, events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);
        let (dir, source) = setup_project("fn main() {}");
        let anchor = make_anchor("fn main() {}");

        let created = service
            .create_annotation(dir.path(), &source, "to delete", vec![], "alice", anchor)
            .unwrap();

        service
            .delete_annotation(dir.path(), &source, &created.id)
            .unwrap();

        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert!(sidecar.annotations.is_empty());

        // 2 events: create + delete
        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
    }

    #[test]
    fn delete_nonexistent_annotation_returns_not_found() {
        let (bus, _events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);
        let (dir, source) = setup_project("fn main() {}");

        let result = service.delete_annotation(dir.path(), &source, "DOES-NOT-EXIST");
        assert!(result.is_err());
        match result.unwrap_err() {
            RuntimeError::NotFound(msg) => {
                assert!(msg.contains("DOES-NOT-EXIST"));
            }
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn clear_annotations_removes_all() {
        let (bus, events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);
        let (dir, source) = setup_project("fn main() {}");
        let anchor1 = make_anchor("fn main() {}");
        let anchor2 = make_anchor("fn main() {}");

        service
            .create_annotation(dir.path(), &source, "first", vec![], "alice", anchor1)
            .unwrap();
        service
            .create_annotation(dir.path(), &source, "second", vec![], "bob", anchor2)
            .unwrap();

        service.clear_annotations(dir.path(), &source).unwrap();

        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert!(sidecar.annotations.is_empty());

        // 3 events: 2 creates + 1 clear
        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 3);
    }
}
