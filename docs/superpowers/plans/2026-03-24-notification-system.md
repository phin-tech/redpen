# Notification System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a general-purpose OS notification system to Red Pen with per-type toggles in Settings.

**Architecture:** Centralized Rust-side `NotificationService` that fires macOS notifications via `tauri-plugin-notification`. Triggers reach it through deep-link URLs (CLI), internal calls (review done), and file-watch events (new annotations). All notification types are toggleable via `NotificationSettings` in the existing settings infrastructure.

**Tech Stack:** Rust (tauri-plugin-notification, url crate), Svelte 5, TypeScript

**Spec:** `docs/superpowers/specs/2026-03-24-notification-system-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src-tauri/src/notification.rs` | `NotificationKind` enum, `NotificationService` struct, `send()` method, `default_title_body()` helper |
| `src-tauri/src/settings.rs` | `NotificationSettings` struct, additions to `AppSettings` and `UpdateSettingsRequest` |
| `src-tauri/src/lib.rs` | Plugin registration, `mod notification`, deep-link URL branching for `notify` |
| `src-tauri/src/state.rs` | No changes — `NotificationService` gets `AppHandle` from setup, not state |
| `src-tauri/src/commands/annotations.rs` | `send_notification` command, `ReviewComplete` trigger in `signal_review_done` |
| `src-tauri/capabilities/default.json` | Add `notification:default` permission |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-notification` and `url` dependencies |
| `package.json` | Add `@tauri-apps/plugin-notification` |
| `src/lib/types.ts` | `NotificationSettings` interface, update `AppSettings` |
| `src/lib/tauri.ts` | `sendNotification()` invoke wrapper |
| `src/components/SettingsDialog.svelte` | Notification toggles section |
| `src/App.svelte` | Deep-link `notify` handling, file-watch new annotation detection |
| `crates/redpen-cli/src/main.rs` | Fire `redpen://notify?...` deep link after `--reply-to` annotations |

---

### Task 1: Add dependencies and plugin registration

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs:19-23`

- [ ] **Step 1: Add Rust dependencies**

In `src-tauri/Cargo.toml`, add after `tauri-plugin-deep-link = "2"` (line 14):

```toml
tauri-plugin-notification = "2"
url = "2"
```

- [ ] **Step 2: Add JS dependency**

Run: `npm install @tauri-apps/plugin-notification`

- [ ] **Step 3: Add notification permission to capabilities**

In `src-tauri/capabilities/default.json`, add `"notification:default"` to the permissions array (after `"deep-link:default"` on line 19):

```json
"deep-link:default",
"notification:default"
```

- [ ] **Step 4: Register notification plugin in lib.rs**

In `src-tauri/src/lib.rs`, add after `.plugin(tauri_plugin_deep_link::init())` (line 23):

```rust
.plugin(tauri_plugin_notification::init())
```

- [ ] **Step 5: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml package.json package-lock.json src-tauri/capabilities/default.json src-tauri/src/lib.rs
git commit -m "feat: add tauri-plugin-notification and url dependencies"
```

---

### Task 2: Add NotificationSettings to settings infrastructure

**Files:**
- Modify: `src-tauri/src/settings.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Write failing test for NotificationSettings defaults**

Add to `src-tauri/src/settings.rs` at the bottom of the `mod tests` block (after line 169):

```rust
#[test]
fn notification_settings_defaults() {
    let settings = AppSettings::default();
    assert!(settings.notifications.annotation_reply);
    assert!(settings.notifications.review_complete);
    assert!(!settings.notifications.new_annotation);
    assert!(settings.notifications.deep_link);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test notification_settings_defaults`
Expected: FAIL — `notifications` field doesn't exist on `AppSettings`

- [ ] **Step 3: Add NotificationSettings struct and field**

In `src-tauri/src/settings.rs`, add before the `AppSettings` struct (before line 8):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    pub annotation_reply: bool,
    pub review_complete: bool,
    pub new_annotation: bool,
    pub deep_link: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            annotation_reply: true,
            review_complete: true,
            new_annotation: false,
            deep_link: true,
        }
    }
}
```

Add to `AppSettings` struct (after `ignored_folder_names` field):

```rust
#[serde(default)]
pub notifications: NotificationSettings,
```

Add to `AppSettings::default()` (after `ignored_folder_names`):

```rust
notifications: NotificationSettings::default(),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test notification_settings_defaults`
Expected: PASS

- [ ] **Step 5: Write failing test for serde round-trip with notifications**

Add to `mod tests`:

```rust
#[test]
fn notification_settings_serde_roundtrip() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("settings.json");

    let settings = AppSettings {
        author: "sam".to_string(),
        default_labels: vec![],
        ignored_folder_names: vec![],
        notifications: NotificationSettings {
            annotation_reply: false,
            review_complete: true,
            new_annotation: true,
            deep_link: false,
        },
    };

    settings.save_to_path(&path).unwrap();
    let reloaded = AppSettings::load_or_default(&path).unwrap();
    assert_eq!(reloaded.notifications, settings.notifications);
}
```

- [ ] **Step 6: Run test to verify it passes** (should pass already since struct derives Serialize/Deserialize)

Run: `cd src-tauri && cargo test notification_settings_serde_roundtrip`
Expected: PASS

- [ ] **Step 7: Write failing test for backward-compatible loading (missing notifications key)**

Add to `mod tests`:

```rust
#[test]
fn loads_settings_without_notifications_key() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("settings.json");

    // Write a settings file without the notifications key (old format)
    let json = r#"{"author":"sam","defaultLabels":[],"ignoredFolderNames":[]}"#;
    std::fs::write(&path, json).unwrap();

    let settings = AppSettings::load_or_default(&path).unwrap();
    // Should get defaults
    assert!(settings.notifications.annotation_reply);
    assert!(!settings.notifications.new_annotation);
}
```

- [ ] **Step 8: Run test**

Run: `cd src-tauri && cargo test loads_settings_without_notifications_key`
Expected: PASS (because of `#[serde(default)]`)

- [ ] **Step 9: Add notifications to UpdateSettingsRequest**

In `src-tauri/src/settings.rs`, add to `UpdateSettingsRequest` struct (after `ignored_folder_names` field):

```rust
pub notifications: Option<NotificationSettings>,
```

In the `apply` method, add after the `ignored_folder_names` block:

```rust
if let Some(notifications) = self.notifications {
    settings.notifications = notifications;
}
```

- [ ] **Step 10: Update TypeScript types**

In `src/lib/types.ts`, add before the `AppSettings` interface:

```typescript
export interface NotificationSettings {
  annotationReply: boolean;
  reviewComplete: boolean;
  newAnnotation: boolean;
  deepLink: boolean;
}
```

Add to the `AppSettings` interface:

```typescript
notifications: NotificationSettings;
```

- [ ] **Step 11: Run all settings tests**

Run: `cd src-tauri && cargo test settings`
Expected: all PASS

- [ ] **Step 12: Commit**

```bash
git add src-tauri/src/settings.rs src/lib/types.ts
git commit -m "feat: add NotificationSettings to settings infrastructure"
```

---

### Task 3: Create NotificationService

**Files:**
- Create: `src-tauri/src/notification.rs`
- Modify: `src-tauri/src/lib.rs:1-4` (add `mod notification`)

- [ ] **Step 1: Create notification.rs with NotificationKind and NotificationService**

Create `src-tauri/src/notification.rs`:

```rust
use crate::settings::{AppSettings, NotificationSettings};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    AnnotationReply,
    ReviewComplete,
    NewAnnotation,
    DeepLink,
}

impl NotificationSettings {
    pub fn is_enabled(&self, kind: NotificationKind) -> bool {
        match kind {
            NotificationKind::AnnotationReply => self.annotation_reply,
            NotificationKind::ReviewComplete => self.review_complete,
            NotificationKind::NewAnnotation => self.new_annotation,
            NotificationKind::DeepLink => self.deep_link,
        }
    }
}

impl NotificationKind {
    /// Generate default notification title and body for a given kind.
    pub fn default_title_body(&self, file_name: &str, line: Option<u32>) -> (String, String) {
        let line_str = line.map(|l| format!(":{}", l)).unwrap_or_default();
        match self {
            NotificationKind::AnnotationReply => (
                "Agent replied".to_string(),
                format!("New reply on {}{}", file_name, line_str),
            ),
            NotificationKind::ReviewComplete => (
                "Review complete".to_string(),
                "Changes applied, ready for re-review".to_string(),
            ),
            NotificationKind::NewAnnotation => (
                "New annotation".to_string(),
                format!("New comment on {}{}", file_name, line_str),
            ),
            NotificationKind::DeepLink => (
                "Opening file".to_string(),
                format!("{}{}", file_name, line_str),
            ),
        }
    }
}

pub struct NotificationService {
    app_handle: AppHandle,
}

impl NotificationService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Send an OS notification if the given kind is enabled in settings.
    /// Note: `NotificationService` is intentionally not stored in managed state —
    /// it's a thin wrapper around `AppHandle` with no internal state, so creating
    /// it fresh per call is fine. The spec's `deep_link` parameter is omitted
    /// because Tauri v2 desktop notifications don't support click actions.
    pub fn send(
        &self,
        kind: NotificationKind,
        title: &str,
        body: &str,
        settings: &AppSettings,
    ) -> Result<(), String> {
        if !settings.notifications.is_enabled(kind) {
            return Ok(());
        }
        self.app_handle
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 2: Add mod declaration**

In `src-tauri/src/lib.rs`, add after `mod workspace_index;` (line 4):

```rust
mod notification;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles (may warn about unused — that's fine)

- [ ] **Step 4: Add unit tests for is_enabled**

Add to the bottom of `src-tauri/src/notification.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::NotificationSettings;

    #[test]
    fn is_enabled_respects_settings() {
        let settings = NotificationSettings {
            annotation_reply: true,
            review_complete: false,
            new_annotation: false,
            deep_link: true,
        };

        assert!(settings.is_enabled(NotificationKind::AnnotationReply));
        assert!(!settings.is_enabled(NotificationKind::ReviewComplete));
        assert!(!settings.is_enabled(NotificationKind::NewAnnotation));
        assert!(settings.is_enabled(NotificationKind::DeepLink));
    }

    #[test]
    fn is_enabled_defaults_match_spec() {
        let settings = NotificationSettings::default();

        assert!(settings.is_enabled(NotificationKind::AnnotationReply));
        assert!(settings.is_enabled(NotificationKind::ReviewComplete));
        assert!(!settings.is_enabled(NotificationKind::NewAnnotation));
        assert!(settings.is_enabled(NotificationKind::DeepLink));
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test notification`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/notification.rs src-tauri/src/lib.rs
git commit -m "feat: add NotificationService and NotificationKind"
```

---

### Task 4: Wire deep-link notify handler in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs:102-108`

This is the key integration: the `on_open_url` handler learns to parse `redpen://notify?...` URLs, fire a notification, and also navigate the app.

- [ ] **Step 1: Add imports to lib.rs**

At the top of `src-tauri/src/lib.rs`, add:

```rust
use notification::{NotificationKind, NotificationService};
use url::Url;
```

- [ ] **Step 2: Create NotificationService in setup and store in state**

In `src-tauri/src/state.rs`, add the `NotificationService` as an optional field — but actually, `NotificationService` needs `AppHandle` which is only available in `setup`. The cleaner approach: create it in `setup` and pass it into the closure.

In `lib.rs`, inside the `.setup(|app| {` block (after the menu setup, before deep link handling around line 102), add:

```rust
let notification_service = NotificationService::new(app.handle().clone());
```

- [ ] **Step 3: Replace the deep-link handler with URL-parsing version**

Replace lines 102-108 in `lib.rs`:

```rust
// Handle deep links received while app is running (warm start)
let handle = app.handle().clone();
app.deep_link().on_open_url(move |event| {
    for url in event.urls() {
        let _ = handle.emit("deep-link-open", url.to_string());
    }
});
```

With:

```rust
// Handle deep links received while app is running (warm start)
let handle = app.handle().clone();
let state_for_links = app.state::<AppState>();
let settings_for_links = state_for_links.settings.clone();
app.deep_link().on_open_url(move |event| {
    for raw_url in event.urls() {
        let url_str = raw_url.to_string();
        if let Ok(parsed) = Url::parse(&url_str) {
            match parsed.host_str() {
                Some("notify") => {
                    let params: std::collections::HashMap<String, String> =
                        parsed.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                    let kind_str = params.get("kind").map(|s| s.as_str()).unwrap_or("");
                    let file = params.get("file");
                    let line = params.get("line");

                    let kind = match kind_str {
                        "annotation_reply" => Some(NotificationKind::AnnotationReply),
                        "review_complete" => Some(NotificationKind::ReviewComplete),
                        "new_annotation" => Some(NotificationKind::NewAnnotation),
                        _ => None,
                    };

                    if let Some(kind) = kind {
                        let settings = settings_for_links.lock().unwrap();
                        let file_name = file
                            .and_then(|f| f.rsplit('/').next().map(|s| s.to_string()))
                            .unwrap_or_else(|| "unknown".to_string());
                        let line_num = line.and_then(|l| l.parse::<u32>().ok());
                        let (title, body) = kind.default_title_body(&file_name, line_num);
                        let _ = notification_service.send(kind, &title, &body, &settings);
                    }

                    // Emit refresh first so annotations reload, then navigate
                    if let Some(file) = file {
                        let refresh_url = format!("redpen://refresh?file={}", file);
                        let _ = handle.emit("deep-link-open", refresh_url);
                        let mut nav_url = format!("redpen://open?file={}", file);
                        if let Some(line) = line {
                            nav_url.push_str(&format!("&line={}", line));
                        }
                        let _ = handle.emit("deep-link-open", nav_url);
                    }
                }
                _ => {
                    // Existing behavior for open, refresh, etc.
                    let _ = handle.emit("deep-link-open", url_str);
                }
            }
        } else {
            let _ = handle.emit("deep-link-open", url_str);
        }
    }
});
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles

- [ ] **Step 5: Add unit test for URL parsing logic**

Add to `src-tauri/src/notification.rs` tests module:

```rust
#[test]
fn parse_notify_url() {
    let url = url::Url::parse("redpen://notify?kind=annotation_reply&file=%2Ftmp%2Fmain.rs&line=42").unwrap();
    assert_eq!(url.host_str(), Some("notify"));
    let params: std::collections::HashMap<String, String> =
        url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    assert_eq!(params.get("kind").map(|s| s.as_str()), Some("annotation_reply"));
    assert_eq!(params.get("file").map(|s| s.as_str()), Some("/tmp/main.rs"));
    assert_eq!(params.get("line").map(|s| s.as_str()), Some("42"));
}
```

- [ ] **Step 6: Run tests**

Run: `cd src-tauri && cargo test parse_notify_url`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/notification.rs
git commit -m "feat: wire deep-link notify handler for OS notifications"
```

---

### Task 5: Add send_notification command and ReviewComplete trigger

**Files:**
- Modify: `src-tauri/src/commands/annotations.rs`
- Modify: `src-tauri/src/lib.rs` (register new command)

- [ ] **Step 1: Add send_notification Tauri command**

In `src-tauri/src/commands/annotations.rs`, add at the top:

```rust
use crate::notification::{NotificationKind, NotificationService};
```

Add a new command after `signal_review_done`:

```rust
#[tauri::command]
pub fn send_notification(
    kind: String,
    file_name: String,
    line: Option<u32>,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let notification_kind = match kind.as_str() {
        "annotation_reply" => NotificationKind::AnnotationReply,
        "review_complete" => NotificationKind::ReviewComplete,
        "new_annotation" => NotificationKind::NewAnnotation,
        "deep_link" => NotificationKind::DeepLink,
        _ => return Err(format!("Unknown notification kind: {}", kind)),
    };

    let (title, body) = notification_kind.default_title_body(&file_name, line);

    let settings = state.settings.lock().unwrap();
    let service = NotificationService::new(app_handle);
    service.send(notification_kind, &title, &body, &settings)
}
```

- [ ] **Step 2: Register the command in lib.rs**

In `src-tauri/src/lib.rs`, add to the `generate_handler!` macro (after `signal_review_done`):

```rust
commands::annotations::send_notification,
```

- [ ] **Step 3: Add ReviewComplete notification to signal_review_done**

In `src-tauri/src/commands/annotations.rs`, modify `signal_review_done` to accept `app_handle`:

Change the function signature from:
```rust
pub fn signal_review_done(file_path: String, verdict: Option<String>) -> Result<(), String> {
```
To:
```rust
pub fn signal_review_done(
    file_path: String,
    verdict: Option<String>,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
```

Add after the `fs::write(&signal_path, signal_content)` line (before the channel POST block):

```rust
// Fire OS notification for review complete
let settings = state.settings.lock().unwrap();
let service = NotificationService::new(app_handle);
let _ = service.send(
    NotificationKind::ReviewComplete,
    "Review complete",
    &format!("Verdict: {}", verdict_str),
    &settings,
);
drop(settings);
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/annotations.rs src-tauri/src/lib.rs
git commit -m "feat: add send_notification command and ReviewComplete trigger"
```

---

### Task 6: Add notification toggles to Settings UI

**Files:**
- Modify: `src/components/SettingsDialog.svelte`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Add sendNotification to tauri.ts**

In `src/lib/tauri.ts`, add after the `getSettings` function:

```typescript
export async function sendNotification(
  kind: string,
  fileName: string,
  line?: number
): Promise<void> {
  return invoke("send_notification", { kind, fileName, line });
}
```

- [ ] **Step 2: Add notification state to SettingsDialog.svelte**

In `src/components/SettingsDialog.svelte`, add state variables after `ignoredFolderNames` (line 10):

```typescript
let notifyAnnotationReply = $state(true);
let notifyReviewComplete = $state(true);
let notifyNewAnnotation = $state(false);
let notifyDeepLink = $state(true);
```

- [ ] **Step 3: Load notification settings in onMount**

In the `onMount` callback, add after `ignoredFolderNames = settings.ignoredFolderNames.join(", ");`:

```typescript
if (settings.notifications) {
  notifyAnnotationReply = settings.notifications.annotationReply;
  notifyReviewComplete = settings.notifications.reviewComplete;
  notifyNewAnnotation = settings.notifications.newAnnotation;
  notifyDeepLink = settings.notifications.deepLink;
}
```

- [ ] **Step 4: Include notifications in save function**

In the `save` function, update the `updateSettings` call to include notifications:

```typescript
await updateSettings({
  author,
  defaultLabels: labels,
  ignoredFolderNames: ignoredFolders,
  notifications: {
    annotationReply: notifyAnnotationReply,
    reviewComplete: notifyReviewComplete,
    newAnnotation: notifyNewAnnotation,
    deepLink: notifyDeepLink,
  },
});
```

- [ ] **Step 5: Add Notifications section to the dialog markup**

In `SettingsDialog.svelte`, add after the "Ignored folders" `<div>` block (after line 88) and before the button row:

```svelte
<div class="flex flex-col gap-2 pt-2 border-t border-border-default/40">
  <span class="text-xs text-text-secondary font-medium">Notifications</span>

  <label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer">
    <input type="checkbox" bind:checked={notifyAnnotationReply}
      class="accent-accent rounded" />
    Agent replied to annotation
  </label>

  <label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer">
    <input type="checkbox" bind:checked={notifyReviewComplete}
      class="accent-accent rounded" />
    Review complete
  </label>

  <label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer">
    <input type="checkbox" bind:checked={notifyNewAnnotation}
      class="accent-accent rounded" />
    New annotation on file
  </label>

  <label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer">
    <input type="checkbox" bind:checked={notifyDeepLink}
      class="accent-accent rounded" />
    Deep link received
  </label>
</div>
```

- [ ] **Step 6: Verify the dev build works**

Run: `npm run dev` and open Settings dialog. Verify checkboxes appear, toggle them, save, reopen — values should persist.

- [ ] **Step 7: Commit**

```bash
git add src/components/SettingsDialog.svelte src/lib/tauri.ts
git commit -m "feat: add notification toggles to Settings dialog"
```

---

### Task 7: Fire notification from CLI on --reply-to

**Files:**
- Modify: `crates/redpen-cli/src/main.rs:151-167`

- [ ] **Step 1: Add notify_app function**

Add a new function after `notify_app_refresh` in `crates/redpen-cli/src/main.rs`:

```rust
/// Send a notification deep link to the desktop app.
/// The app's notify handler will fire the OS notification, refresh annotations,
/// and navigate to the file — so this replaces the need for a separate refresh call.
fn notify_app(kind: &str, file_path: &Path, line: Option<u32>) {
    let mut url = format!(
        "redpen://notify?kind={}&file={}",
        kind,
        urlencoding::encode(&file_path.to_string_lossy())
    );
    if let Some(l) = line {
        url.push_str(&format!("&line={}", l));
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }
}
```

- [ ] **Step 2: Replace notify_app_refresh with notify_app in the reply-to branch**

In `cmd_annotate`, inside the `if let Some(parent_id) = reply_to` block (around line 164), replace:

```rust
notify_app_refresh(&abs_path);
```

With:

```rust
notify_app("annotation_reply", &abs_path, Some(start_line));
```

This sends a single deep link (`redpen://notify?kind=annotation_reply&...`) instead of two separate calls. The app's `notify` handler emits both a `refresh` and an `open` event, avoiding the race condition of two independent deep links.

- [ ] **Step 3: Verify CLI compiles**

Run: `cd crates/redpen-cli && cargo check`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add crates/redpen-cli/src/main.rs
git commit -m "feat: CLI fires notification deep link on --reply-to"
```

---

### Task 8: Detect new annotations on file-watch and notify

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Add annotation change detection to file watcher**

In `src/App.svelte`, modify `handleFileSelect` to track annotation IDs and detect new ones from other authors.

Import `sendNotification` and `getSettings` at the top (with other imports from `$lib/tauri`):

```typescript
import { readDirectory, sendNotification, getSettings } from "$lib/tauri";
```

Replace the file watcher setup in `handleFileSelect` (lines 74-81):

```typescript
// Set up file watcher for source change detection
stopWatcher?.();
let lastAnnotationIds = new Set(
  getAnnotationsState().sidecar?.annotations.map(a => a.id) ?? []
);
const reloadFile = debounce(async () => {
  if (editor.currentFilePath) {
    await openFile(editor.currentFilePath);
    await loadAnnotations(editor.currentFilePath);

    // Detect new annotations from other authors
    const state = getAnnotationsState();
    const currentSettings = await getSettings();
    const currentAuthor = currentSettings.author;
    const newAnnotations = (state.sidecar?.annotations ?? []).filter(
      a => !lastAnnotationIds.has(a.id) && a.author !== currentAuthor
    );
    lastAnnotationIds = new Set(
      state.sidecar?.annotations.map(a => a.id) ?? []
    );

    // No extra debounce needed — reloadFile is already debounced at 500ms,
    // so burst file-watch events are collapsed into a single detection pass.
    if (newAnnotations.length > 0) {
      const fileName = editor.currentFilePath.split("/").pop() ?? "unknown";
      for (const ann of newAnnotations) {
        const kind = ann.replyTo ? "annotation_reply" : "new_annotation";
        const line = ann.anchor?.range?.startLine;
        sendNotification(kind, fileName, line).catch(() => {});
      }
    }
  }
}, 500);
stopWatcher = await watch(path, reloadFile, { recursive: false });
```

- [ ] **Step 2: Verify dev build works**

Run: `npm run dev`, open a file, use CLI to add an annotation — notification should appear.

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat: detect new annotations on file-watch and send notifications"
```

---

### Task 9: Add DeepLink notification for background deep links

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add DeepLink notification for non-notify deep links**

In the `on_open_url` handler in `lib.rs`, in the `_ =>` match arm (the existing behavior branch), add a notification before emitting:

Replace:
```rust
_ => {
    // Existing behavior for open, refresh, etc.
    let _ = handle.emit("deep-link-open", url_str);
}
```

With:
```rust
_ => {
    // Fire DeepLink notification
    if let Some(file) = parsed.query_pairs().find(|(k, _)| k == "file").map(|(_, v)| v.to_string()) {
        let file_name = file.rsplit('/').next().unwrap_or("unknown");
        let line = parsed.query_pairs().find(|(k, _)| k == "line").map(|(_, v)| v.to_string());
        let line_str = line.as_ref().map(|l| format!(":{}", l)).unwrap_or_default();
        let settings = settings_for_links.lock().unwrap();
        let _ = notification_service.send(
            NotificationKind::DeepLink,
            "Opening file",
            &format!("{}{}", file_name, line_str),
            &settings,
        );
    }
    let _ = handle.emit("deep-link-open", url_str);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add DeepLink notification for incoming deep links"
```

---

### Task 10: Run all tests and manual verification

- [ ] **Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: all PASS

- [ ] **Step 2: Run frontend tests**

Run: `npm run test:run`
Expected: all PASS (existing tests should not break)

- [ ] **Step 3: Build the app**

Run: `npm run tauri build -- --debug`
Expected: builds successfully

- [ ] **Step 4: Manual test checklist**

1. Open the app, go to Settings — verify notification checkboxes appear with correct defaults
2. Toggle settings, save, reopen Settings — verify they persisted
3. Use CLI: `redpen annotate <file> --reply-to <id> --body "test reply"` — verify OS notification appears
4. Click "Done Reviewing" in the app — verify ReviewComplete notification appears
5. Toggle "Agent replied to annotation" OFF, repeat step 3 — verify NO notification
6. Toggle "Deep link received" ON, run `redpen open <file>` — verify DeepLink notification

- [ ] **Step 5: Final commit if any fixes needed, otherwise done**

```bash
git add -A
git commit -m "feat: notification system complete (closes #11)"
```
