# Review Page — Design Spec

## Overview

A full-screen feed view that replaces the editor pane, providing a linear, scrollable review experience. Two modes share the same UI:

- **Review Changes:** Shows file diffs with annotations rendered inline — for reviewing a changeset.
- **Agent Feedback:** Shows only files/lines that have agent annotations — for responding to agent questions and choice prompts.

The view is modeled after GitHub's "Files changed" PR review tab but lives entirely inside the Red Pen desktop app.

## Entry & Exit

### Entry
- **Keyboard shortcut:** `Cmd+Shift+R` (macOS) / `Ctrl+Shift+R` (Linux/Windows) opens Review Changes mode.
- **Command bar:** "Review Changes" and "Agent Feedback" as separate actions.
- **Agent-triggered:** `redpen open --wait` can open directly into Agent Feedback mode.
- **Sidebar button:** A "Review" button in the sidebar bottom action bar (where Approve / Request Changes currently live).

### Exit
- **`Esc` key** returns to the normal editor view.
- **Close button** in the top bar does the same.
- Previous editor state (open file, cursor position, selection) is restored on exit.

## Modes

### Review Changes
- **Scope (default):** All files in the current `redpen open` session.
- **Scope (optional):** A git diff range (`base..head`) to control which changes are shown. Selectable via a scope dropdown in the top bar.
- **Content:** Diff hunks for every changed file. Hunks without annotations are still shown so the reviewer can see the full changeset. Annotations appear inline on their relevant lines.

### Agent Feedback
- **Scope:** Annotations from the current agent session (filtered to agent authors).
- **Content:** Only files and lines that have agent annotations. No empty diff hunks. Code context is still shown around each annotation.

## Feed Structure

### Top Bar (sticky)
| Element | Description |
|---------|-------------|
| Mode label | "Review Changes" or "Agent Feedback" |
| Count badge | Total annotations (and unresolved count) |
| Scope selector | Dropdown: "All files (N)" / "Current file" / custom range |
| Progress | Fraction or percentage of resolved/answered items |
| Close button | Exits the review page (`Esc`) |

### Progress Bar
A thin bar below the top bar showing resolved/answered vs total. Fills left-to-right as the reviewer works through annotations.

### File Sections
Each file in the review gets a section:
- **File header:** File path (monospace), annotation count badge, "Jump to file" button that exits review mode and opens the file in the editor at the relevant location.
- **Content:** Diff hunks (Review Changes) or annotated code snippets (Agent Feedback).

### Annotation Cards
Each annotation renders as a card attached to its code context:

#### Code Snippet
- ~3-5 lines of context around the annotated line(s).
- Annotated lines are highlighted (kind-colored background, matching existing bubble colors).
- "Expand above" / "Expand below" buttons to reveal more context on demand.
- Line numbers shown in the gutter.
- Syntax highlighting applied (same language detection as the editor).

#### Card Body
- **Header row:** Author name (with bot icon for agents), kind badge (comment/note/explanation/label, color-coded), relative timestamp.
- **Body text:** The annotation body, full text (no truncation).
- **Labels:** Rendered as small badges below the body.
- **Choices:** If the annotation has choices, render them as radio buttons (single) or checkboxes (multi) inline. Selecting a choice persists immediately (same behavior as inline bubbles).

#### Reply Thread
- Existing replies shown below the root annotation, indented with a `↳` indicator.
- Each reply shows author + body.

#### Actions Row
- **Reply input:** Text field for composing a reply. Submit with `Enter` (or `Cmd+Enter` for multiline).
- **Resolve button:** Marks the annotation as resolved/acknowledged. Resolved cards are visually dimmed but remain in the feed.
- **Jump to file button:** Opens the file in the normal editor, scrolled to this annotation's line.

### Keyboard Navigation
| Key | Action |
|-----|--------|
| `j` / `k` | Move to next / previous card |
| `r` | Focus the reply input on the current card |
| `e` | Toggle resolve on the current card |
| `o` | Jump to file in editor for the current card |
| `Esc` | Close review page (or blur reply input if focused) |

A hint bar at the bottom of the viewport shows these shortcuts.

## Data Flow

### Review Changes Mode
1. On entry, determine the file list from the current session (or specified git range).
2. For each file, compute the diff (via `git diff` through a Tauri command).
3. Load annotations for each file (existing `list_annotations` command).
4. Merge: position annotations onto their diff hunk lines.
5. Render the feed.

### Agent Feedback Mode
1. On entry, load all annotations for the session.
2. Filter to annotations from agent authors.
3. For each annotation, load the code snippet context (file content around the anchor line).
4. Render the feed (no diff hunks, just code context + annotations).

### State Management
- A new `reviewPage` store manages:
  - `mode: "changes" | "feedback" | null` (null = review page closed)
  - `activeCardIndex: number`
  - `resolvedIds: Set<string>`
  - `scope: { type: "session" } | { type: "diff", base: string, head: string }`
- Resolving/replying uses the existing annotation mutation functions (`editAnnotation`, `removeAnnotation`, Tauri commands).
- Choice selections use the existing `updateChoices` flow.

### New Tauri Commands Needed
- `get_file_diff(file_path: String, base: Option<String>, head: Option<String>) -> Vec<DiffHunk>` — returns diff hunks for a file.
- `get_file_snippet(file_path: String, line: u32, context: u32) -> FileSnippet` — returns lines around a given line number with syntax info.
- `resolve_annotation(file_path: String, annotation_id: String)` — marks an annotation as resolved (new field on `Annotation`).

### New Annotation Fields
- `resolved: bool` — whether the annotation has been resolved/acknowledged. Default `false`. Serialized with `#[serde(default, skip_serializing_if = "is_false")]` for backward compatibility with existing sidecar files. Resolving is distinct from deleting — resolved annotations remain in the data and are visible (dimmed) in the feed.

## UI Components

### New Components
- `ReviewPage.svelte` — top-level view component, replaces the editor pane when active.
- `ReviewTopBar.svelte` — sticky top bar with mode, counts, scope, close.
- `ReviewCard.svelte` — single annotation card with code snippet, body, actions.
- `ReviewCodeSnippet.svelte` — code context display with line numbers, highlighting, expand buttons.

### Modified Components
- `EditorPane.svelte` — conditionally renders `ReviewPage` instead of the editor when review mode is active.
- `AnnotationSidebar.svelte` — add "Review" button to the bottom action bar.
- App-level keyboard handler — register `Cmd+Shift+R` shortcut.

## Visual Design
- Matches existing Red Pen dark theme (surface colors, borders, text colors).
- Kind-colored strips on cards (blue=comment, yellow=note, purple=explanation, green=label) — same palette as inline bubbles.
- Active card has accent-colored border + subtle glow.
- Resolved cards dimmed to 50% opacity.
- Code snippets use the same monospace font and syntax highlighting as the editor.
- Responsive: feed is max-width ~720px centered, comfortable reading width.

## Out of Scope (for v1)
- Inline commenting on diff lines that don't have annotations yet (add-new-annotation from review page).
- Drag-to-reorder annotations.
- Filtering/sorting within the review page (beyond the two modes).
- Collaborative real-time review (multiple reviewers).
- Export/share review summary.
