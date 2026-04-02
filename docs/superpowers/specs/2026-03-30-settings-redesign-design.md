# Settings Redesign Design Spec

## Goal

Replace the current single-panel settings dialog with a sidebar-navigated, pro-density settings panel using pill tag inputs, path pickers, toggle switches, and repo cards.

## Architecture

The settings dialog becomes a wider modal (680px) with a left sidebar rail for category navigation and a right panel for the active settings. Three categories: General, Git & GitHub, Notifications. All existing settings data and the `updateSettings` API remain unchanged — this is a pure frontend redesign.

---

## Categories

### General
- **Identity section**: Author name text input
- **Annotation Defaults section**:
  - Default labels — pill tag input (Enter/comma to add, × to remove)
  - Ignored folders — pill tag input (monospace pills)

### Git & GitHub
- **Checkout section**: Default checkout location — read-only path display + Browse button (opens folder picker)
- **Tracked Repositories section**:
  - List of repo cards: each shows `owner/repo` name + local path + delete button
  - Manual add: `owner/repo` input + path input (with browse button) + Add button
  - "Add from disk…" button: opens folder picker, auto-detects GitHub remote from git config, adds to list

### Notifications
- Toggle switches (right-aligned) with label + description (left-aligned):
  - Agent replied to annotation (default: on)
  - Review complete (default: on)
  - New annotation on file (default: off)
  - Deep link received (default: on)

---

## Components

### Sidebar Rail
- 140px wide, `--surface-panel` background
- Category buttons: text-aligned left, active state has `--accent` left border + slightly lighter background
- Categories: General, Git & GitHub, Notifications

### Pill Tag Input (`TagInput.svelte`)
New reusable component. Props: `tags: string[]`, `onAdd(tag)`, `onRemove(index)`, `placeholder`, `mono?: boolean`.

- Container styled like a text input (`--surface-base` bg, border)
- Existing tags render as pills: `--surface-raised` background, text + × button
- Trailing inline `<input>` for typing new tags
- Enter or comma key adds the current text as a new tag
- Backspace on empty input removes the last tag
- Monospace variant for folder names

### Toggle Switch (`ToggleSwitch.svelte`)
New reusable component. Props: `checked: boolean`, `onToggle()`.

- 36×20px track, 16×16px circle
- On: amber track (`--accent`), white circle pushed right
- Off: dim grey track (`rgba(255,255,255,0.1)`), grey circle left
- CSS transition on circle position

### Repo Card (inline in settings)
- `--surface-panel` background, 6px border-radius
- Two lines: repo name (brighter) + path (monospace, muted)
- Delete button (×) on the right

### "Add from disk" Flow
1. User clicks "Add from disk…" button
2. Folder picker opens (Tauri dialog)
3. On selection, call `invoke("get_git_root", { path })` to find the git root
4. Call `invoke("get_git_remote_url", { path })` — new Rust command that runs `git remote get-url origin`
5. Parse `github.com/owner/repo` from the URL (handle `https://` and `git@` formats)
6. Add to tracked repos list with detected name + selected path
7. If remote detection fails, fall back to manual entry with the path pre-filled

### Path Picker
Read-only path display + "Browse…" ghost button. Opens Tauri folder dialog on click. Updates the path value.

---

## Layout

```
┌─────────────────────────────────────────────────────────┐
│ ┌───────────┐ ┌───────────────────────────────────────┐ │
│ │ General   │ │ IDENTITY                              │ │
│ │ Git & GH  │ │ Author name: [input]                  │ │
│ │ Notifs    │ │                                        │ │
│ │           │ │ ANNOTATION DEFAULTS                    │ │
│ │           │ │ Default labels: [pill] [pill] [+]      │ │
│ │           │ │ Ignored folders: [pill] [pill] [+]     │ │
│ └───────────┘ └───────────────────────────────────────┘ │
│                              [Cancel]  [Save Changes]   │
└─────────────────────────────────────────────────────────┘
```

- Dialog width: 680px (up from 380px)
- Sidebar: 140px fixed
- Content: fills remaining space
- Footer: Cancel (ghost text) + Save Changes (solid amber) — always visible at bottom

---

## Styling

| Element | Treatment |
|---------|-----------|
| Section headers | Small caps, `--text-ghost` color, 1px bottom divider |
| Inputs | `--surface-base` background, 1px `--border-default` border, 6px radius |
| Active focus | 1px amber border (`--accent`), no glow |
| Pill tags | `--surface-raised` background, 4px radius, 11px font |
| Toggle on | `--accent` track, white circle |
| Toggle off | `rgba(255,255,255,0.1)` track, grey circle |
| Repo cards | `--surface-panel` background, 6px radius, no border |
| Save button | Solid amber fill |
| Cancel | Ghost text, no border |

---

## New Rust Command

### `get_git_remote_url`

```rust
#[tauri::command]
fn get_git_remote_url(path: String) -> Result<Option<String>, String> {
    // Open repo at path, get "origin" remote URL
    // Return None if no remote found
}
```

Parse the returned URL to extract `owner/repo`:
- `https://github.com/owner/repo.git` → `owner/repo`
- `git@github.com:owner/repo.git` → `owner/repo`
- Other hosts → return raw URL, let user edit

---

## What's NOT in Scope

- Keyboard navigation between categories (tab through sidebar)
- Search/filter within settings
- Import/export settings
- Per-repo settings overrides
