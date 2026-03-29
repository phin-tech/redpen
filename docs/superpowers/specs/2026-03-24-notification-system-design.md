# Notification System Design

## Summary

Add a general-purpose OS notification system to Red Pen using `tauri-plugin-notification`. Rather than hard-coding a single notification for annotation replies, this builds reusable infrastructure that any event in the app can hook into. All notification types are toggleable in the existing Settings dialog.

## Motivation

The reviewer may not be watching Red Pen while the agent works. OS notifications close the feedback loop — the reviewer gets pinged when the agent has something to say, similar to how Slack or GitHub notify on replies. Clicking a notification should deep-link back into Red Pen at the relevant file/line.

Ref: https://github.com/phin-tech/redpen/issues/11

## Notification Types

| Kind | Trigger | Default | Description |
|------|---------|---------|-------------|
| `AnnotationReply` | Agent replies to an annotation via CLI (`redpen annotate --reply-to`) | on | "Agent replied to your comment on main.rs:42" |
| `ReviewComplete` | Agent signals review done or all feedback addressed | on | "Changes applied, ready for re-review" |
| `NewAnnotation` | New annotation created on a file (not by current user) | off | "New comment on main.rs:42" |
| `DeepLink` | Deep link received while app is in background | on | "Opening main.rs:42" |

## Architecture

### Centralized Rust-side notification service

All notification decisions happen in Rust. The backend owns the `tauri-plugin-notification` calls, checks user settings, and fires notifications. The frontend never touches the notification API directly.

This is the right choice because the key trigger — "agent replied to annotation" — happens via the CLI hitting the Rust backend, not through the UI. The frontend might not even be in focus.

### How triggers reach the notification service

There are three paths into the notification service:

1. **Deep link URL** (`redpen://notify?kind=annotation_reply&file=...&line=...`) — The CLI invokes `open` on a deep-link URL. The app's existing `on_open_url` handler in `lib.rs` receives it, parses the `notify` action, and calls `NotificationService::send`. **No MCP channel server required.** This is the primary path for CLI-triggered notifications like `AnnotationReply`.

2. **Channel server POST** — When the MCP channel server is running, `signal_review_done` POSTs to it. The Rust backend can fire a `ReviewComplete` notification at the same time. This path is only active when the channel is up.

3. **Internal events** — Deep link received, file-watch detects new annotations — these are handled entirely within the app process.

```
┌─────────────────────────────────────────────┐
│  NotificationService (Rust, managed state)  │
│                                             │
│  - holds AppHandle (for tauri-plugin calls) │
│  - reads notification settings from state   │
│  - send(kind, title, body, deep_link_url)   │
│    → checks if kind is enabled in settings  │
│    → fires OS notification if yes           │
└─────────────────────────────────────────────┘
        ▲               ▲              ▲
        │               │              │
   deep-link URL    signal_review    file-watch
   (CLI → open)     done (internal)  (sidecar change)
```

### NotificationKind enum

```rust
pub enum NotificationKind {
    AnnotationReply,
    ReviewComplete,
    NewAnnotation,
    DeepLink,
}
```

### NotificationService

A thin wrapper that:
1. Accepts a `NotificationKind`, title, body, and optional deep-link URL
2. Reads the current `AppSettings` to check if this kind is enabled
3. Fires the notification via `tauri-plugin-notification` Rust API

```rust
use tauri_plugin_notification::NotificationExt;

pub struct NotificationService {
    app_handle: AppHandle,
}

impl NotificationService {
    pub fn send(
        &self,
        kind: NotificationKind,
        title: &str,
        body: &str,
        deep_link: Option<&str>,
        settings: &AppSettings,
    ) -> Result<(), String> {
        if !settings.is_notification_enabled(&kind) {
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

## Settings Changes

### Rust: AppSettings

Add a `notifications` field to `AppSettings`:

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
            new_annotation: false,  // off by default — could be noisy
            deep_link: true,
        }
    }
}
```

`AppSettings` gains:
```rust
pub struct AppSettings {
    pub author: String,
    pub default_labels: Vec<String>,
    pub ignored_folder_names: Vec<String>,
    #[serde(default)]
    pub notifications: NotificationSettings,
}
```

The `#[serde(default)]` ensures existing settings files without a `notifications` key load cleanly with defaults.

### Rust: UpdateSettingsRequest

Add notification settings to the existing `UpdateSettingsRequest` and its `apply()` method:

```rust
pub struct UpdateSettingsRequest {
    pub author: Option<String>,
    pub default_labels: Option<Vec<String>>,
    pub ignored_folder_names: Option<Vec<String>>,
    pub notifications: Option<NotificationSettings>,
}

impl UpdateSettingsRequest {
    pub fn apply(self, settings: &mut AppSettings) {
        // ... existing fields ...
        if let Some(notifications) = self.notifications {
            settings.notifications = notifications;
        }
    }
}
```

### TypeScript types

```typescript
interface NotificationSettings {
  annotationReply: boolean;
  reviewComplete: boolean;
  newAnnotation: boolean;
  deepLink: boolean;
}

interface AppSettings {
  author: string;
  defaultLabels: string[];
  ignoredFolderNames: string[];
  notifications: NotificationSettings;
}
```

### Settings UI

Add a "Notifications" section to `SettingsDialog.svelte` below the existing fields. Each notification type gets a labeled toggle/checkbox:

- "Agent replied to annotation" — toggle for `annotationReply`
- "Review complete" — toggle for `reviewComplete`
- "New annotation on file" — toggle for `newAnnotation`
- "Deep link received" — toggle for `deepLink`

No new dialog needed — extends the existing settings modal.

## Trigger Integration Points

### 1. Agent replied to annotation (AnnotationReply)

**Where:** The CLI's `redpen annotate --reply-to` writes to sidecar files.

**Approach:** After writing the sidecar, the CLI invokes a deep-link URL:
```
redpen://notify?kind=annotation_reply&file=<path>&line=<line>
```

The app's `on_open_url` handler parses the URL and branches:
- If the host is `notify`: parse query params, call `NotificationService::send`, then also emit `deep-link-open` with a corresponding `redpen://open?file=<path>&line=<line>` URL so the frontend navigates to the file.
- If the host is `open` (existing behavior): emit `deep-link-open` as before.

```rust
use url::Url;

for raw_url in event.urls() {
    if let Ok(parsed) = Url::parse(&raw_url.to_string()) {
        match parsed.host_str() {
            Some("notify") => {
                // Extract kind, file, line from query params
                let params: HashMap<_, _> = parsed.query_pairs().collect();
                let kind = params.get("kind").map(|v| v.as_ref());
                let file = params.get("file").map(|v| v.as_ref());
                let line = params.get("line").map(|v| v.as_ref());

                // Fire OS notification
                notification_service.send(kind, title, body, settings);

                // Also navigate the app to the file
                if let (Some(file), Some(line)) = (file, line) {
                    let nav_url = format!("redpen://open?file={}&line={}", file, line);
                    let _ = handle.emit("deep-link-open", nav_url);
                }
            }
            _ => {
                // Existing behavior: emit deep-link-open
                let _ = handle.emit("deep-link-open", raw_url.to_string());
            }
        }
    }
}
```

This requires adding the `url` crate to `src-tauri/Cargo.toml`.

### 2. Review complete (ReviewComplete)

**Where:** `signal_review_done` in `commands/annotations.rs` already fires when the agent finishes.

**Change:** After writing the signal file and posting to the channel, call `NotificationService::send` with `ReviewComplete`. This notification is informational — it does not navigate, since the reviewer decides when to re-open the review.

### 3. New annotation on file (NewAnnotation)

**Where:** Same sidecar file-watch mechanism. When a new annotation appears (not a reply, not by the current user), fire notification.

**Change:** Frontend detects new annotation during file-watch reload, calls `send_notification` Tauri command. This notification is informational — clicking it brings the app to foreground, which is already showing the file tree where the annotation appeared.

**Note:** File-watch events can fire in bursts (e.g., CLI writes multiple sidecar files quickly). A simple 500ms debounce on the frontend side before invoking `send_notification` prevents the most obvious spam.

### 4. Deep link received in background (DeepLink)

**Where:** `lib.rs` already handles deep links in `on_open_url`.

**Change:** For non-`notify` deep links (i.e., `redpen://open?...`), call `NotificationService::send` with `DeepLink` if the window is not focused, then emit `deep-link-open` as before. The notification is informational — the app has already navigated to the file.

## Notification Click Behavior

Clicking an OS notification on macOS brings the app to the foreground. The per-kind behavior:

- **AnnotationReply**: The `notify` deep-link handler both fires the notification AND emits `deep-link-open` to navigate to the file/line. When the user clicks the notification, the app is already on the right file.
- **ReviewComplete**: Informational only. Brings the app to foreground; no specific file navigation.
- **NewAnnotation**: Informational only. The app is likely already showing the file tree.
- **DeepLink**: The deep-link handler already navigated the app before the notification was sent.

Tauri v2's notification plugin does not support custom click actions on desktop. This behavior is sufficient — the notification tells you what happened, clicking it brings you to the app.

## Permission Handling

On macOS, the first call to `app.notification().builder().show()` triggers the system notification permission prompt automatically. No app-side permission management needed.

If the user denies permission at the OS level, subsequent `.show()` calls silently fail. The app does not need to check or request permission itself — macOS handles this.

## Dependencies

| Dependency | Where | Purpose |
|------------|-------|---------|
| `tauri-plugin-notification` | `src-tauri/Cargo.toml` | Rust notification API |
| `@tauri-apps/plugin-notification` | `package.json` | JS bindings (for Tauri plugin registration) |
| `url` | `src-tauri/Cargo.toml` | Parse `redpen://notify?...` deep-link URLs |

## Files to Change

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `tauri-plugin-notification` and `url` dependencies |
| `package.json` | Add `@tauri-apps/plugin-notification` |
| `src-tauri/capabilities/default.json` | Add `notification:default` permission |
| `src-tauri/src/lib.rs` | Register notification plugin, add `mod notification`, branch deep-link handler for `notify` URLs |
| `src-tauri/src/settings.rs` | Add `NotificationSettings` struct, add field to `AppSettings` and `UpdateSettingsRequest` |
| `src-tauri/src/notification.rs` | New: `NotificationService` + `NotificationKind` |
| `src-tauri/src/commands/annotations.rs` | Add `send_notification` command, trigger on review done |
| `src/lib/types.ts` | Add `NotificationSettings` interface, update `AppSettings` |
| `src/lib/tauri.ts` | Add `sendNotification` invoke wrapper |
| `src/components/SettingsDialog.svelte` | Add notification toggles section |
| `src/App.svelte` | Trigger notifications on file-watch annotation changes (with debounce) |

## Testing

- Unit tests for `NotificationSettings` default values and serde round-trip
- Unit test for `is_notification_enabled` with each kind
- Unit test for `redpen://notify?...` URL parsing (extract kind, file, line correctly)
- Integration: manually verify each notification type fires on macOS
- Verify existing settings files without `notifications` key load without error
- Verify toggling settings on/off persists and takes effect immediately

## Out of Scope

- Notification sound customization
- Notification grouping/batching (if agent replies to 5 annotations, you get 5 notifications — revisit if spammy)
- Mobile notification actions (Tauri v2 desktop doesn't support them)
- Notification history/log in the UI
