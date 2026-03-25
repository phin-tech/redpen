# Event/Service Boundary Design

GitHub issue: #36

## Problem

Tauri command handlers in `src-tauri/src/commands/` directly access `State<AppState>`, call Tauri APIs, and contain business logic. This makes core runtime paths untestable without bootstrapping the full Tauri runtime. The goal is a framework-agnostic service boundary so core logic can be tested in isolation.

## Approach

Introduce a new `crates/redpen-runtime` crate that owns:
- An `EventBus` trait with a single `emit(AppEvent)` method
- An `AppEvent` enum representing outbound events
- An `AnnotationService<E: EventBus>` that encapsulates annotation CRUD logic
- A `RuntimeError` enum for structured error handling
- A `NoOpEventBus` for tests and CLI use

The Tauri app provides a `TauriEventBus` adapter and thin command wrappers that extract state, call the service, and map errors to strings.

This mirrors Atuin's `EventBus` trait / `ChannelEventBus` adapter pattern.

## Scope

**First vertical slice:** Annotation CRUD (get, get_all, create, update, delete, clear).

**Out of scope for this PR:**
- Extracting workspace index, notifications, settings, git, diff, or export commands
- Changing UX or frontend API surface
- Async runtime changes

## Design

### EventBus Trait & AppEvent

In `crates/redpen-runtime/src/event_bus.rs`:

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

The enum includes variants beyond the annotation slice (`SettingsChanged`, `ReviewDone`) so future extractions don't require trait changes.

### RuntimeError

In `crates/redpen-runtime/src/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
}
```

Converts from `redpen_core::sidecar::SidecarError` via a `From` impl.

### AnnotationService

In `crates/redpen-runtime/src/annotations.rs`:

```rust
pub struct AnnotationService<E: EventBus> {
    event_bus: E,
}

impl<E: EventBus> AnnotationService<E> {
    pub fn new(event_bus: E) -> Self;

    pub fn get_annotations(&self, file_path: &Path, project_root: &Path)
        -> Result<SidecarFile, RuntimeError>;

    pub fn get_all_annotations(&self, dir: &Path, project_root: &Path)
        -> Result<Vec<FileAnnotations>, RuntimeError>;

    pub fn create_annotation(
        &self, project_root: &Path, source_path: &Path,
        body: &str, labels: Vec<String>, author: &str,
        anchor: Anchor,
    ) -> Result<Annotation, RuntimeError>;

    pub fn update_annotation(
        &self, project_root: &Path, source_path: &Path,
        id: &str, body: Option<&str>, labels: Option<Vec<String>>,
    ) -> Result<Annotation, RuntimeError>;

    pub fn delete_annotation(
        &self, project_root: &Path, source_path: &Path, id: &str,
        ) -> Result<(), RuntimeError>;

    pub fn clear_annotations(
        &self, project_root: &Path, source_path: &Path,
    ) -> Result<(), RuntimeError>;
}
```

Key decisions:
- Takes `author` as a parameter (not settings) — the Tauri command layer reads settings and passes it in.
- Emits `AppEvent::AnnotationsChanged` after mutations. Read-only methods (`get_annotations`, `get_all_annotations`) do not emit events and could stay as plain functions, but are included in the service for cohesion.
- Project root resolution (`git2::Repository::discover`) stays in the Tauri command layer — keeps `git2` out of `redpen-runtime`.
- `CreateAnnotationRequest` is defined in the Tauri command layer (where `ts_rs::TS` derives live for TypeScript bindings). The service method takes individual parameters instead of the DTO.
- `FileAnnotations` struct moves to `redpen-runtime` (no `ts_rs` dependency needed — the Tauri layer re-derives `TS` on the re-exported type, or keeps a thin wrapper).

### TauriEventBus Adapter

In `src-tauri/src/event_bus.rs`:

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

### Tauri Command Layer

Commands become thin wrappers. Example:

```rust
#[tauri::command]
pub fn create_annotation(
    request: CreateAnnotationRequest,
    state: State<'_, AppState>,
) -> Result<Annotation, String> {
    let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
    let project_root = resolve_project_root(&request.source_path);
    annotations
        .create_annotation(&project_root, &request.source_path, &request.body, request.labels, &settings.author, anchor)
        .map_err(|e| e.to_string())
}
```

### AppState Changes

`AppState::new()` runs before the `AppHandle` exists, so `AnnotationService<TauriEventBus>` cannot be constructed there. Instead, the service is managed as a separate Tauri state, initialized in the `.setup()` closure where the `AppHandle` is available:

```rust
// In lib.rs setup:
.setup(|app| {
    let event_bus = TauriEventBus::new(app.handle().clone());
    let annotation_service = AnnotationService::new(event_bus);
    app.manage(annotation_service);
    // ... existing setup code
    Ok(())
})
```

Commands access it as a separate `State<>` parameter:

```rust
#[tauri::command]
pub fn create_annotation(
    request: CreateAnnotationRequest,
    state: State<'_, AppState>,
    annotations: State<'_, AnnotationService<TauriEventBus>>,
) -> Result<Annotation, String> { ... }
```

### Crate Structure

```
crates/redpen-runtime/
  Cargo.toml          # depends on redpen-core, thiserror
  src/
    lib.rs            # pub mod event_bus, annotations, error
    event_bus.rs      # EventBus trait, AppEvent enum, NoOpEventBus
    annotations.rs    # AnnotationService<E>
    error.rs          # RuntimeError
```

### Testing

A `RecordingEventBus` in tests captures emitted events:

```rust
struct RecordingEventBus {
    events: Arc<Mutex<Vec<AppEvent>>>,
}

impl EventBus for RecordingEventBus {
    fn emit(&self, event: AppEvent) {
        self.events.lock().unwrap().push(event);
    }
}
```

Tests verify:
- CRUD operations work on temp directories with sidecar files
- Mutations emit `AnnotationsChanged` events
- Error cases return structured `RuntimeError` variants
- No Tauri runtime needed

## Files Modified

| File | Change |
|------|--------|
| `crates/redpen-runtime/` (new) | New crate with EventBus, AnnotationService, RuntimeError |
| `src-tauri/Cargo.toml` | Add `redpen-runtime` dependency |
| `src-tauri/src/event_bus.rs` (new) | TauriEventBus adapter |
| `src-tauri/src/state.rs` | No changes (service managed as separate Tauri state) |
| `src-tauri/src/lib.rs` | Construct TauriEventBus + AnnotationService during setup |
| `src-tauri/src/commands/annotations.rs` | Thin wrappers calling AnnotationService |
| `Cargo.toml` (workspace) | Add redpen-runtime to workspace members |

## Verification

1. `cargo check -p redpen-runtime` — new crate compiles
2. `cargo check -p redpen-desktop` — Tauri app compiles with new wiring
3. `cargo test -p redpen-runtime` — annotation service tests pass without Tauri
4. `cargo test` — all existing tests still pass
5. `cargo clippy -- -D warnings` — no new warnings (pre-existing `too_many_arguments` excluded)
6. Manual test: create/edit/delete annotations in the desktop app to verify behavior is unchanged
