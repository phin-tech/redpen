# Diff View for Red Pen Code Review

## Overview

Add a diff viewing capability to Red Pen's code view, allowing reviewers to see what changed between two git refs. Three view modes (split, unified, highlights) mirror GitHub's approach. Users toggle between modes via a segmented control or command palette. Annotations attach only to the "new" side.

## Diff Computation (Rust Backend)

### New Tauri Commands

**`compute_diff(directory: String, file_path: String, base_ref: String, target_ref: String, algorithm: String) -> DiffResult`**

Computes a line-level diff between two versions of a file. `directory` is used with `Repository::discover` to locate the git repo (consistent with existing `get_git_status`). `file_path` is resolved relative to the repo root. Refs can be branch names, commit SHAs, `"HEAD"`, or `"working-tree"` (current file on disk). `algorithm` is `"patience"` or `"myers"`.

The backend reads file content at each ref via `git2` (for committed refs) or the filesystem (for `"working-tree"`).

Uses the `similar` crate with Patience algorithm by default (produces more readable diffs for structural code changes). Myers algorithm available as a user preference.

```rust
struct DiffResult {
    base_ref: String,
    target_ref: String,
    hunks: Vec<DiffHunk>,
    old_content: String,
    new_content: String,
}

struct DiffHunk {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    changes: Vec<DiffChange>,
}

struct DiffChange {
    kind: ChangeKind,       // Equal, Insert, Delete
    old_line: Option<u32>,  // present for Equal and Delete
    new_line: Option<u32>,  // present for Equal and Insert
    content: String,
}
```

**`list_refs(directory: String) -> RefList`**

Returns available git refs for the ref picker UI. Uses `Repository::discover(directory)` to locate the repo.

```rust
struct RefList {
    branches: Vec<BranchInfo>,    // name, is_current
    tags: Vec<String>,
    recent_commits: Vec<CommitInfo>,  // sha, short_message, ~10 most recent
}
```

### Algorithm Configuration

- **Patience** (default): Anchors on unique lines, produces readable diffs for code review
- **Myers**: Minimizes total edits, more compact output

The algorithm preference is passed per-call from the frontend to `compute_diff`. The frontend reads it from the diff store, which initializes from `AppSettings`. This avoids two sources of truth — the frontend store is authoritative at runtime, settings file is just persistence.

Add to Rust `AppSettings` struct: `diff_algorithm: String` (default `"patience"`, with `#[serde(default)]` for backward compat).
Add to TypeScript `AppSettings` interface: `diff_algorithm: "patience" | "myers"`.

## Frontend State

### `diff.svelte.ts` Store

```typescript
interface DiffState {
    enabled: boolean;
    mode: "split" | "unified" | "highlights";
    baseRef: string;
    targetRef: string;
    diffResult: DiffResult | null;
    algorithm: "patience" | "myers";  // source of truth at runtime, persisted to AppSettings
    loading: boolean;
}
```

Key actions:
- `setDiffMode(mode)` — switches view mode, no re-fetch needed (same diff data drives all three views)
- `setDiffRefs(base, target)` — updates refs, calls `compute_diff`, stores result
- `exitDiff()` — clears state, restores normal editor view
- `swapRefs()` — flips base and target, recomputes

Default when entering diff mode: `HEAD ←→ working tree`.

## View Modes

### Highlights Mode

The existing CodeMirror editor displays `new_content`. A new extension adds:
- Line decorations: green background on inserted lines
- Gutter markers: green indicator on inserted lines
- No deleted lines shown
- Annotations work identically to normal mode (real file, real line numbers)

### Unified Mode

Single CodeMirror instance with a synthetic document interleaving equal, insert, and delete lines:
- Two gutter columns: old line numbers (left), new line numbers (right)
- Delete lines get old line number only; insert lines get new line number only
- Line decorations: green background for inserts, red for deletes
- Delete lines are not selectable for annotation placement
- Syntax highlighting loaded from `new_content` language detection; may be imperfect on delete lines (acceptable, matches GitHub behavior)

**Line number mapping:** The synthetic document has its own line numbers (1-based, sequential) that don't correspond to either the old or new file. A `Map<number, number>` maps each synthetic line to its `new_content` line number (only for insert and equal lines). When the user creates an annotation in unified mode, the annotation system intercepts the CodeMirror line number, looks it up in this map, and stores the `new_content` line number in the anchor. Delete lines have no entry in the map and are blocked from annotation selection. This map is built once when the synthetic document is constructed from `DiffResult.hunks`.

### Split Mode

Two CodeMirror instances side-by-side:
- **Left editor**: `old_content`, read-only, red highlights on deleted lines
- **Right editor**: `new_content`, read-only, green highlights on inserted lines
- **Scroll sync**: bidirectional. When either editor scrolls, find the hunk containing the top visible line, compute the corresponding line in the other editor using the hunk's old/new line mapping, and scroll the other editor to that position. Uses CodeMirror's `EditorView.scrollDOM` scroll events and `EditorView.dispatch({ effects: EditorView.scrollIntoView() })`. Ghost line heights are factored into offset calculation.
- **Ghost lines**: empty padding lines (CodeMirror widget decorations) inserted to keep corresponding hunks vertically aligned. When one side has more lines in a hunk (e.g., 3 inserts vs 1 delete), the other side gets 2 ghost lines.
- **Annotations**: only the right (new) editor has annotation support wired up

**Component architecture:** The current `Editor.svelte` reads from the global `getEditor()` store and cannot be instantiated twice. Split mode uses a new `DiffEditor.svelte` component that accepts `content`, `decorations`, and `enableAnnotations` as props. In split mode, `EditorPane` renders two `DiffEditor` instances. In unified/highlights modes, it renders one. The existing `Editor.svelte` remains unchanged for non-diff use.

All three modes share a `createDiffDecorations` function:

```typescript
function createDiffDecorations(
    diffResult: DiffResult,
    mode: "split" | "unified" | "highlights"
): {
    old?: DecorationSet;        // split mode only: left editor decorations
    new: DecorationSet;         // all modes: right/single editor decorations
    syntheticDoc?: string;      // unified mode only: interleaved content
    lineMap?: Map<number, number>;  // unified mode only: synthetic line → new_content line
}
```

## UI Components

### Segmented Control (in EditorPane header)

Three-option toggle: `Split | Unified | Highlights`

Positioned alongside the existing file name in the editor toolbar. Only visible when diff mode is active.

### `DiffRefPicker.svelte`

Compact ref picker displayed below the segmented control in the EditorPane header:

```
[main ▾]  ←→  [working tree ▾]
```

Each dropdown shows:
- **Branches**: local branches, current branch at top
- **Tags**: if any
- **Recent commits**: last ~10 with short message and SHA
- **"Working tree"**: current file on disk

Swap button (`←→`) flips base and target. Agent can set refs programmatically via `setDiffRefs`.

### Command Palette

Three new entries:
- "Diff: Split View"
- "Diff: Unified View"
- "Diff: Highlights Only"

All call `setDiffMode` on the diff store.

## Layout Behavior

- **Split mode**: file tree auto-collapses to icon rail. Restored when exiting diff or switching to unified/highlights.
- **Unified / Highlights modes**: file tree stays as-is (single editor panel, no space pressure).
- **File navigation**: switching files while in diff mode keeps diff active with same refs — recomputes diff for the new file. Enables PR-like review flow.
- **"Changed only" filter**: existing file tree filter works naturally with diff mode — shows only git-changed files, clicking each loads its diff.

## Annotation Behavior

Annotations attach to the **new side only** across all modes:
- **Highlights**: annotations work identically to normal mode
- **Unified**: only insert and equal lines are annotatable (delete lines are blocked)
- **Split**: only the right editor supports annotation placement

Annotations use `new_content` line numbers, so annotations created in diff mode are identical to those created in normal mode. No special handling needed when exiting diff.

## Error Handling

- **Diff computation failure** (file not in repo, invalid ref, binary file): `compute_diff` returns an error. The frontend shows an inline error message in the editor area ("Cannot compute diff: {reason}") with an option to exit diff mode. No crash, no empty state.
- **Loading state**: while `compute_diff` is in flight, show a spinner overlay on the editor area. `DiffState.loading` tracks this.
- **Untracked files** (git status `"?"`): no old version exists. Diff shows the entire file as additions (every line green). This is the natural output — `old_content` is empty, all changes are inserts.
- **Deleted files** (git status `"D"`): no new version on disk. Diff shows the entire file as deletions (every line red). Annotations are disabled since there is no new content to annotate.
- **File changed on disk while viewing diff**: the existing file watcher fires. Re-invoke `compute_diff` with the same refs to refresh. If `target_ref` is `"working-tree"`, the new content picks up the change automatically.

## Dependencies

### Rust
- `similar` crate (already supports both Patience and Myers algorithms)
- Existing `git2` for ref listing and file content retrieval at specific commits

### Frontend
- No new dependencies — CodeMirror 6 decorations, gutters, and panels are sufficient
- Existing Svelte 5 runes for state management
