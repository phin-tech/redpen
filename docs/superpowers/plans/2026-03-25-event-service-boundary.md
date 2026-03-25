# Event/Service Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract annotation CRUD logic from Tauri command handlers into a framework-agnostic `redpen-runtime` crate behind an `EventBus` trait, so core runtime paths are testable without Tauri.

**Architecture:** New `crates/redpen-runtime` crate owns `EventBus` trait, `AppEvent` enum, `AnnotationService<E>`, and `RuntimeError`. Tauri app provides `TauriEventBus` adapter. Commands become thin wrappers that extract state, call the service, and map errors.

**Tech Stack:** Rust, redpen-core (sidecar/annotation types), thiserror, tempfile (dev)

**Spec:** `docs/superpowers/specs/2026-03-24-event-service-boundary-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `crates/redpen-runtime/Cargo.toml` (new) | Crate manifest — depends on redpen-core, thiserror, chrono |
| `crates/redpen-runtime/src/lib.rs` (new) | Re-exports: event_bus, annotations, error modules |
| `crates/redpen-runtime/src/event_bus.rs` (new) | `EventBus` trait, `AppEvent` enum, `NoOpEventBus` |
| `crates/redpen-runtime/src/error.rs` (new) | `RuntimeError` enum with `From<SidecarError>` |
| `crates/redpen-runtime/src/annotations.rs` (new) | `AnnotationService<E>`, `FileAnnotations` struct |
| `Cargo.toml` | Add `crates/redpen-runtime` to workspace members |
| `src-tauri/Cargo.toml` | Add `redpen-runtime` dependency |
| `src-tauri/src/event_bus.rs` (new) | `TauriEventBus` adapter |
| `src-tauri/src/commands/annotations.rs` | Thin wrappers delegating to `AnnotationService` |
| `src-tauri/src/lib.rs` | Construct `TauriEventBus` + `AnnotationService` in `.setup()`, add `mod event_bus` |

---

### Task 1: Scaffold `redpen-runtime` crate with EventBus trait

**Files:**
- Create: `crates/redpen-runtime/Cargo.toml`
- Create: `crates/redpen-runtime/src/lib.rs`
- Create: `crates/redpen-runtime/src/event_bus.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create `crates/redpen-runtime/Cargo.toml`**

```toml
[package]
name = "redpen-runtime"
version = "0.1.6"
edition = "2021"

[dependencies]
redpen-core = { path = "../redpen-core" }
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create `crates/redpen-runtime/src/event_bus.rs`**

```rust
#[derive(Debug, Clone)]
pub enum AppEvent {
    AnnotationsChanged { file_path: String },
    SettingsChanged,
    ReviewDone { file_path: String, verdict: Option<String> },
}

pub trait EventBus: Send + Sync {
    fn emit(&self, event: AppEvent);
}

pub struct NoOpEventBus;

impl EventBus for NoOpEventBus {
    fn emit(&self, _event: AppEvent) {}
}
```

- [ ] **Step 3: Create `crates/redpen-runtime/src/lib.rs`**

```rust
pub mod event_bus;
```

- [ ] **Step 4: Add to workspace members in root `Cargo.toml`**

Change:
```toml
members = ["src-tauri", "crates/redpen-core", "crates/redpen-cli"]
```
To:
```toml
members = ["src-tauri", "crates/redpen-core", "crates/redpen-cli", "crates/redpen-runtime"]
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p redpen-runtime`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add crates/redpen-runtime/ Cargo.toml Cargo.lock
git commit -m "feat(runtime): scaffold redpen-runtime crate with EventBus trait (#36)"
```

---

### Task 2: Add RuntimeError with From<SidecarError>

**Files:**
- Create: `crates/redpen-runtime/src/error.rs`
- Modify: `crates/redpen-runtime/src/lib.rs`

- [ ] **Step 1: Create `crates/redpen-runtime/src/error.rs`**

```rust
use redpen_core::sidecar::SidecarError;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<SidecarError> for RuntimeError {
    fn from(err: SidecarError) -> Self {
        match err {
            SidecarError::Io(e) => RuntimeError::Io(e),
            SidecarError::Json(e) => RuntimeError::Json(e),
        }
    }
}
```

- [ ] **Step 2: Add module to `lib.rs`**

```rust
pub mod error;
pub mod event_bus;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p redpen-runtime`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/redpen-runtime/src/error.rs crates/redpen-runtime/src/lib.rs
git commit -m "feat(runtime): add RuntimeError with From<SidecarError> (#36)"
```

---

### Task 3: Implement AnnotationService with tests (TDD)

This is the core task. Extract the business logic from `src-tauri/src/commands/annotations.rs` into `AnnotationService<E: EventBus>`.

**Files:**
- Create: `crates/redpen-runtime/src/annotations.rs`
- Modify: `crates/redpen-runtime/src/lib.rs`

**Reference for existing logic:**
- `src-tauri/src/commands/annotations.rs:36-63` — `load_sidecar_for_file` and `save_sidecar` helpers
- `src-tauri/src/commands/annotations.rs:65-70` — `get_annotations`
- `src-tauri/src/commands/annotations.rs:72-140` — `get_all_annotations` + `collect_sidecar_files` + `FileAnnotations`
- `src-tauri/src/commands/annotations.rs:142-195` — `create_annotation`
- `src-tauri/src/commands/annotations.rs:197-220` — `update_annotation`
- `src-tauri/src/commands/annotations.rs:222-242` — `delete_annotation`, `clear_annotations`

**Key types from redpen-core used here:**
- `redpen_core::sidecar::SidecarFile` — load/save annotation JSON
- `redpen_core::annotation::{Annotation, AnnotationKind, Anchor, Range}` — annotation data
- `redpen_core::anchor::reanchor_annotations` — re-anchoring on file changes
- `redpen_core::hash::{hash_file, hash_string}` — content hashing

- [ ] **Step 1: Write test helpers (RecordingEventBus + factory)**

In `crates/redpen-runtime/src/annotations.rs`, add at the bottom:

```rust
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
            (Self { events: events.clone() }, events)
        }
    }

    impl EventBus for RecordingEventBus {
        fn emit(&self, event: AppEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    /// Create a temp project with a source file and return (project_root, source_path)
    fn setup_project(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let source = dir.path().join("test.rs");
        std::fs::write(&source, content).unwrap();
        (dir, source)
    }
}
```

- [ ] **Step 2: Write failing test for `get_annotations`**

Add test to the `tests` module:

```rust
    #[test]
    fn get_annotations_returns_empty_sidecar_for_new_file() {
        let (dir, source) = setup_project("fn main() {}");
        let (bus, _events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);

        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert!(sidecar.annotations.is_empty());
    }
```

Run: `cargo test -p redpen-runtime -- get_annotations_returns_empty`
Expected: FAIL — `AnnotationService` not defined yet

- [ ] **Step 3: Write the AnnotationService struct and `get_annotations`**

At the top of `crates/redpen-runtime/src/annotations.rs`:

```rust
use crate::error::RuntimeError;
use crate::event_bus::{AppEvent, EventBus};
use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind};
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAnnotations {
    pub file_path: String,
    pub file_name: String,
    pub annotations: Vec<Annotation>,
}

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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p redpen-runtime -- get_annotations_returns_empty`
Expected: PASS

- [ ] **Step 5: Write failing test for `create_annotation` + event emission**

```rust
    #[test]
    fn create_annotation_saves_and_emits_event() {
        let (dir, source) = setup_project("line 1\nline 2\nline 3\n");
        let (bus, events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);

        let anchor = redpen_core::annotation::Anchor::TextContext {
            line_content: "line 2".to_string(),
            surrounding_lines: vec!["line 1".to_string(), "line 2".to_string(), "line 3".to_string()],
            content_hash: redpen_core::hash::hash_string("line 2"),
            range: redpen_core::annotation::Range {
                start_line: 2, start_column: 0, end_line: 2, end_column: 6,
            },
            last_known_line: 2,
        };

        let annotation = service
            .create_annotation(dir.path(), &source, "test comment", vec![], "tester", anchor)
            .unwrap();

        assert_eq!(annotation.body, "test comment");
        assert_eq!(annotation.author, "tester");

        // Verify event was emitted
        let emitted = events.lock().unwrap();
        assert_eq!(emitted.len(), 1);
        assert!(matches!(&emitted[0], AppEvent::AnnotationsChanged { .. }));

        // Verify annotation was persisted
        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert_eq!(sidecar.annotations.len(), 1);
    }
```

Run: `cargo test -p redpen-runtime -- create_annotation_saves`
Expected: FAIL — `create_annotation` not defined

- [ ] **Step 6: Implement `create_annotation`**

Add to `impl<E: EventBus> AnnotationService<E>`:

```rust
    pub fn create_annotation(
        &self,
        project_root: &Path,
        source_path: &Path,
        body: &str,
        labels: Vec<String>,
        author: &str,
        anchor: Anchor,
    ) -> Result<Annotation, RuntimeError> {
        let annotation = Annotation::new(
            AnnotationKind::Comment,
            body.to_string(),
            labels,
            author.to_string(),
            anchor,
        );

        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar.add_annotation(annotation.clone());
        save_sidecar(&sidecar, project_root, source_path)?;

        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });

        Ok(annotation)
    }
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cargo test -p redpen-runtime -- create_annotation_saves`
Expected: PASS

- [ ] **Step 8: Write failing test for `update_annotation`**

```rust
    #[test]
    fn update_annotation_modifies_body_and_labels() {
        let (dir, source) = setup_project("fn main() {}");
        let (bus, events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);

        let anchor = redpen_core::annotation::Anchor::TextContext {
            line_content: "fn main() {}".to_string(),
            surrounding_lines: vec!["fn main() {}".to_string()],
            content_hash: redpen_core::hash::hash_string("fn main() {}"),
            range: redpen_core::annotation::Range {
                start_line: 1, start_column: 0, end_line: 1, end_column: 13,
            },
            last_known_line: 1,
        };

        let created = service
            .create_annotation(dir.path(), &source, "original", vec![], "tester", anchor)
            .unwrap();

        let updated = service
            .update_annotation(dir.path(), &source, &created.id, Some("revised"), Some(vec!["bug".to_string()]))
            .unwrap();

        assert_eq!(updated.body, "revised");
        assert_eq!(updated.labels, vec!["bug"]);
        assert!(updated.updated_at.is_some());

        // 2 events: one for create, one for update
        assert_eq!(events.lock().unwrap().len(), 2);
    }
```

Run: `cargo test -p redpen-runtime -- update_annotation_modifies`
Expected: FAIL — `update_annotation` not defined

- [ ] **Step 9: Implement `update_annotation`**

```rust
    pub fn update_annotation(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
        body: Option<&str>,
        labels: Option<Vec<String>>,
    ) -> Result<Annotation, RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        let annotation = sidecar
            .get_annotation_mut(id)
            .ok_or_else(|| RuntimeError::NotFound(format!("annotation {id}")))?;

        if let Some(b) = body {
            annotation.body = b.to_string();
        }
        if let Some(l) = labels {
            annotation.labels = l;
        }
        annotation.updated_at = Some(chrono::Utc::now());
        let result = annotation.clone();
        save_sidecar(&sidecar, project_root, source_path)?;

        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });

        Ok(result)
    }
```

- [ ] **Step 10: Run test to verify it passes**

Run: `cargo test -p redpen-runtime -- update_annotation_modifies`
Expected: PASS

- [ ] **Step 11: Write failing test for `delete_annotation`**

```rust
    #[test]
    fn delete_annotation_removes_and_emits_event() {
        let (dir, source) = setup_project("fn main() {}");
        let (bus, events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);

        let anchor = redpen_core::annotation::Anchor::TextContext {
            line_content: "fn main() {}".to_string(),
            surrounding_lines: vec!["fn main() {}".to_string()],
            content_hash: redpen_core::hash::hash_string("fn main() {}"),
            range: redpen_core::annotation::Range {
                start_line: 1, start_column: 0, end_line: 1, end_column: 13,
            },
            last_known_line: 1,
        };

        let created = service
            .create_annotation(dir.path(), &source, "to delete", vec![], "tester", anchor)
            .unwrap();

        service.delete_annotation(dir.path(), &source, &created.id).unwrap();

        let sidecar = service.get_annotations(&source, dir.path()).unwrap();
        assert!(sidecar.annotations.is_empty());
        assert_eq!(events.lock().unwrap().len(), 2); // create + delete
    }

    #[test]
    fn delete_nonexistent_annotation_returns_not_found() {
        let (dir, source) = setup_project("fn main() {}");
        let (bus, _events) = RecordingEventBus::new();
        let service = AnnotationService::new(bus);

        let result = service.delete_annotation(dir.path(), &source, "nonexistent");
        assert!(matches!(result, Err(RuntimeError::NotFound(_))));
    }
```

Run: `cargo test -p redpen-runtime -- delete_annotation`
Expected: FAIL

- [ ] **Step 12: Implement `delete_annotation`**

```rust
    pub fn delete_annotation(
        &self,
        project_root: &Path,
        source_path: &Path,
        id: &str,
    ) -> Result<(), RuntimeError> {
        let mut sidecar = load_sidecar_for_file(project_root, source_path)?;
        sidecar
            .remove_annotation(id)
            .ok_or_else(|| RuntimeError::NotFound(format!("annotation {id}")))?;
        save_sidecar(&sidecar, project_root, source_path)?;

        self.event_bus.emit(AppEvent::AnnotationsChanged {
            file_path: source_path.to_string_lossy().to_string(),
        });

        Ok(())
    }
```

- [ ] **Step 13: Run tests to verify they pass**

Run: `cargo test -p redpen-runtime -- delete_annotation`
Expected: PASS (both tests)

- [ ] **Step 14: Implement `clear_annotations` and `get_all_annotations`**

```rust
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

    pub fn get_all_annotations(
        &self,
        dir: &Path,
        project_root: &Path,
    ) -> Result<Vec<FileAnnotations>, RuntimeError> {
        let comments_dir = project_root.join(".redpen").join("comments");
        if !comments_dir.exists() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();
        collect_sidecar_files(&comments_dir, project_root, &mut results)?;
        results.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        Ok(results)
    }
```

Also add the `collect_sidecar_files` helper as a module-level function (copy from `src-tauri/src/commands/annotations.rs:97-140`, changing `String` errors to `RuntimeError`):

```rust
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
        } else if path.extension().map_or(false, |e| e == "json") {
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
```

- [ ] **Step 15: Add module to `lib.rs`**

```rust
pub mod annotations;
pub mod error;
pub mod event_bus;
```

- [ ] **Step 16: Run full test suite**

Run: `cargo test -p redpen-runtime`
Expected: All tests PASS

- [ ] **Step 17: Commit**

```bash
git add crates/redpen-runtime/
git commit -m "feat(runtime): implement AnnotationService with TDD tests (#36)"
```

---

### Task 4: Create TauriEventBus adapter

**Files:**
- Create: `src-tauri/src/event_bus.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs` (add `mod event_bus`)

- [ ] **Step 1: Add `redpen-runtime` dependency to `src-tauri/Cargo.toml`**

Add to `[dependencies]`:
```toml
redpen-runtime = { path = "../crates/redpen-runtime" }
```

- [ ] **Step 2: Create `src-tauri/src/event_bus.rs`**

```rust
use redpen_runtime::event_bus::{AppEvent, EventBus};
use tauri::{AppHandle, Emitter};

pub struct TauriEventBus {
    app_handle: AppHandle,
}

impl TauriEventBus {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl EventBus for TauriEventBus {
    fn emit(&self, event: AppEvent) {
        match &event {
            AppEvent::AnnotationsChanged { file_path } => {
                let _ = self.app_handle.emit("annotations-changed", file_path);
            }
            AppEvent::SettingsChanged => {
                let _ = self.app_handle.emit("settings-changed", ());
            }
            AppEvent::ReviewDone { file_path, verdict } => {
                let _ = self.app_handle.emit("review-done", (file_path, verdict));
            }
        }
    }
}
```

- [ ] **Step 3: Add `mod event_bus;` to `src-tauri/src/lib.rs`**

Add after the existing mod declarations at the top of `lib.rs`:

```rust
mod event_bus;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p red-pen-tauri`
Expected: compiles (event_bus module is declared but not yet used — that's fine)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/event_bus.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add TauriEventBus adapter (#36)"
```

---

### Task 5: Wire AnnotationService into Tauri app and refactor commands

**Files:**
- Modify: `src-tauri/src/lib.rs` — construct service in `.setup()`
- Modify: `src-tauri/src/commands/annotations.rs` — refactor to thin wrappers

This is the integration task. The annotation commands must keep their exact same Tauri command signatures and return types so the frontend is unaffected.

- [ ] **Step 1: Wire `AnnotationService` in `lib.rs` setup**

In `src-tauri/src/lib.rs`, inside the `.setup(|app| { ... })` closure, add after the existing `AppState` management:

```rust
use event_bus::TauriEventBus;
use redpen_runtime::annotations::AnnotationService;

// Inside .setup():
let event_bus = TauriEventBus::new(app.handle().clone());
let annotation_service = AnnotationService::new(event_bus);
app.manage(annotation_service);
```

- [ ] **Step 2: Refactor `get_annotations` command**

In `src-tauri/src/commands/annotations.rs`, change `get_annotations` to delegate:

```rust
#[tauri::command]
pub fn get_annotations(
    file_path: String,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<SidecarFile, String> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    svc.get_annotations(source_path, &project_root)
        .map_err(|e| e.to_string())
}
```

Add necessary imports at the top of the file:
```rust
use crate::event_bus::TauriEventBus;
use redpen_runtime::annotations::AnnotationService;
```

- [ ] **Step 3: Refactor `get_all_annotations` command**

```rust
#[tauri::command]
pub fn get_all_annotations(
    root_folder: String,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<Vec<redpen_runtime::annotations::FileAnnotations>, String> {
    let root = Path::new(&root_folder);
    let project_root = resolve_project_root(root);
    svc.get_all_annotations(root, &project_root)
        .map_err(|e| e.to_string())
}
```

Note: `FileAnnotations` now comes from `redpen_runtime::annotations`. Remove the local `FileAnnotations` struct definition and the `collect_sidecar_files` helper. The `ts_rs::TS` derive for `FileAnnotations` needs to stay in the Tauri layer — add a type alias or wrapper with the derive:

```rust
// Keep the TS binding generation in the Tauri layer
pub use redpen_runtime::annotations::FileAnnotations;

// If ts-rs doesn't work with re-exports, create a thin wrapper instead.
// Test compilation to determine which approach works.
```

- [ ] **Step 4: Refactor `create_annotation` command**

```rust
#[tauri::command]
pub fn create_annotation(
    request: CreateAnnotationRequest,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<Annotation, String> {
    let source_path = Path::new(&request.file_path);
    let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
    let source_lines: Vec<&str> = content.lines().collect();

    let line_idx = (request.start_line as usize).saturating_sub(1);
    let line_content = source_lines.get(line_idx).unwrap_or(&"").to_string();

    let start = line_idx.saturating_sub(2);
    let end = (line_idx + 3).min(source_lines.len());
    let surrounding_lines: Vec<String> = source_lines[start..end]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let range = Range {
        start_line: request.start_line,
        start_column: request.start_column,
        end_line: request.end_line,
        end_column: request.end_column,
    };

    let anchor = Anchor::TextContext {
        line_content: line_content.clone(),
        surrounding_lines,
        content_hash: hash_string(&line_content),
        range,
        last_known_line: request.start_line,
    };

    let author = state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .author
        .clone();

    let project_root = resolve_project_root(source_path);
    svc.create_annotation(&project_root, source_path, &request.body, request.labels, &author, anchor)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 5: Refactor `update_annotation` command**

```rust
#[tauri::command]
pub fn update_annotation(
    file_path: String,
    annotation_id: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<Annotation, String> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    svc.update_annotation(&project_root, source_path, &annotation_id, body.as_deref(), labels)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 6: Refactor `delete_annotation` command**

```rust
#[tauri::command]
pub fn delete_annotation(
    file_path: String,
    annotation_id: String,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<(), String> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    svc.delete_annotation(&project_root, source_path, &annotation_id)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 7: Refactor `clear_annotations` command**

```rust
#[tauri::command]
pub fn clear_annotations(
    file_path: String,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<(), String> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    svc.clear_annotations(&project_root, source_path)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 8: Remove dead code from annotations.rs**

Remove the now-unused private functions and structs:
- `load_sidecar_for_file` (moved to redpen-runtime)
- `save_sidecar` (moved to redpen-runtime)
- `collect_sidecar_files` (moved to redpen-runtime)
- `FileAnnotations` struct (moved to redpen-runtime)

Keep:
- `resolve_project_root` (stays in Tauri layer)
- `CreateAnnotationRequest` (has `ts_rs::TS` derive)
- `update_settings`, `get_settings`, `signal_review_done`, `send_notification` (out of scope)

- [ ] **Step 9: Verify compilation**

Run: `cargo check -p red-pen-tauri`
Expected: compiles with no errors

If `FileAnnotations` re-export doesn't work with `ts-rs`, create a Tauri-side newtype wrapper with the derive.

- [ ] **Step 10: Run full test suite**

Run: `cargo test`
Expected: All existing tests still pass + new runtime tests pass

- [ ] **Step 11: Commit**

```bash
git add src-tauri/ Cargo.lock
git commit -m "refactor: wire AnnotationService into Tauri commands (#36)"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run all checks**

```bash
cargo check -p redpen-runtime
cargo check -p red-pen-tauri
cargo test
cargo clippy -- -D warnings   # pre-existing too_many_arguments is expected
```

- [ ] **Step 2: Verify no remaining business logic in annotation commands**

Grep `src-tauri/src/commands/annotations.rs` for `SidecarFile::load`, `SidecarFile::save`, `sidecar.add_annotation`, `sidecar.remove_annotation` — none should remain in the CRUD commands (only in `signal_review_done` which is out of scope).

- [ ] **Step 3: Verify runtime tests run without Tauri**

```bash
cargo test -p redpen-runtime -v
```

Expected: All tests pass. This confirms the acceptance criterion that "core runtime paths can be unit-tested without Tauri runtime bootstrapping."

- [ ] **Step 4: Commit any final fixes if needed**
