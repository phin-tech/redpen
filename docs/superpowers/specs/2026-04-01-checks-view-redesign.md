# Checks View Redesign

**Date:** 2026-04-01
**Status:** Draft

## Problem

The current Checks view is a "Frankenstein" layout. When active, the app shows 4 horizontal columns: file tree (irrelevant to CI debugging), checks rail, a detail pane that wastes the center on a single status badge, and an empty annotation sidebar. The user scans across 4 columns, the most valuable screen real estate shows almost nothing, and there's no way to read CI logs without leaving the app.

## Design

### Layout: 3-Column Checks Workspace

When the Checks tab is active, the **App shell collapses the file tree and annotation sidebar** (same pattern as split-diff mode). ChecksView takes over the full workspace and renders its own 3-column layout:

```
┌──────────────┬──────────────────────────────────┬─────────────────┐
│  Jobs List   │         Log Terminal              │  Error Context  │
│  (240px)     │         (flex: 1, bg: #000)       │  (220px,        │
│              │                                    │   collapsible)  │
│  Summary     │  ┌─ header: job name + badge ───┐ │                 │
│  ✓ 8  ✗ 7   │  │                               │ │  Failing Files  │
│  ● 1         │  │  Monospace log output with    │ │  ─────────────  │
│              │  │  ANSI colors converted to     │ │  github_rev.rs  │
│  Filter tabs │  │  CSS. True black background.  │ │    :1477        │
│  [All][Fail] │  │                               │ │    :1481        │
│  ──────────  │  │  error[E0599]: no variant...  │ │    :1488        │
│  ● Cargo Chk │  │    --> github_review.rs:1477  │ │  error.rs       │
│  ● Clippy    │  │                               │ │    :11          │
│  ● Cargo Tst │  │  error[E0282]: type annot...  │ │                 │
│  ● Typecheck │  │    --> github_review.rs:1493  │ │                 │
│  ● Vitest    │  │                               │ │  ──────────     │
│  ● Rustfmt   │  │                               │ │  Click to open  │
│  ● Build     │  └──────────────────────────────┘ │  mini-diff      │
└──────────────┴──────────────────────────────────┴─────────────────┘
```

### Column 1: Jobs List (Left, 240px)

**Summary stats** anchored at top as a filter header:
- Pass count (green ✓), fail count (red ✗), pending count (pulsing amber dot)
- Filter tabs below: All | Failed | Passed — filters the list below

**Job list** below the stats:
- Each row: status dot + job name + duration (monospace)
- Selected job has highlighted background with left accent border
- **Running jobs**: pulsing amber dot (no "running" text) — consistent with Red Pen's pulse design language
- **Naming convention**: use the job name exactly as GitHub returns it, no deduplication or reformatting. The `name` field from the check runs API is the canonical label.
- Failed jobs sort to top within their workflow group

### Column 2: Log Terminal (Center, flex: 1)

**The hero of the view.** This is where CI output lives.

**Header bar** (dark, not pure black):
- Status dot + job name + duration + conclusion badge
- Link to full logs on GitHub (small, right-aligned)

**Log body**:
- Background: `#000000` (true black)
- Font: monospace, ~11px, line-height 1.6
- ANSI escape codes stripped and converted to inline CSS color spans
- Auto-scrolls to first error on load (if any errors detected)
- Manual scroll unlocks auto-scroll
- Selectable text for copy/paste

**Data source**: new Rust command `get_job_logs` (see Backend section)

**Empty state**: "Select a job to view logs" centered in the terminal area

### Column 3: Error Context (Right, 220px, collapsible)

**Purpose**: jump-to-code links extracted from log output. This is NOT a full annotation sidebar — it's a lightweight navigation aid.

**Failing Files section**:
- Regex extracts `file_path:line_number` patterns from log text
- Groups by file, deduplicates lines
- Shows file basename with line numbers underneath
- Each file:line entry is a clickable link that opens the mini-diff overlay

**File path extraction** — tool-agnostic regex approach:
- Pattern: lines containing `-->`, `error`, `warning`, `FAIL` followed by a path-like string with `:line` suffix
- Specific patterns to match:
  - Rust: `--> path/to/file.rs:123:45` or `--> path/to/file.rs:123`
  - TypeScript: `path/to/file.ts(123,45): error`
  - Generic: `path/to/file.ext:123` where the path contains at least one `/`
- Dedup by file+line, sort files alphabetically, lines numerically

**Collapsing**: if no file:line patterns are found in the log, the right column auto-hides and the terminal gets the full width. User can toggle it with a button in the terminal header.

### Mini-Diff Overlay

Triggered by clicking a file:line link in the error context column.

**Appearance**:
- Positioned modal overlaying the log terminal area (not full-screen)
- ~70% of the terminal width, ~60% height, centered over the terminal
- Semi-transparent backdrop over the terminal (the log is still partially visible underneath)
- Rounded corners, elevated shadow

**Content**:
- File path + line number in the header
- Shows the file content at the PR's `headSha` ref (using existing `get_file_content_at_ref` command)
- Scrolled to the target line with ~15 lines of context above/below
- Target line highlighted
- Line numbers in gutter
- Syntax highlighting via CodeMirror (reuse existing infrastructure)

**Behavior**:
- Close on Escape, click outside, or close button
- Only one overlay at a time
- Does NOT navigate away from Checks view

## Backend Changes

### New Rust command: `get_job_logs`

```rust
#[tauri::command]
pub fn get_job_logs(repo: String, job_id: u64) -> CommandResult<String>
```

**Flow**:
1. Check cache at `~/.redpen/cache/logs/{job_id}.log`
2. If cache hit, return contents
3. If cache miss, call `gh api repos/{repo}/actions/jobs/{job_id}/logs`
4. Write response to cache file
5. Return log text

**Cache rules**:
- Cache key: `{job_id}.log` — job IDs are globally unique in GitHub
- Only cache completed jobs (check `status == "completed"` before caching)
- No TTL needed — completed job logs are immutable
- Cache dir created on first use: `~/.redpen/cache/logs/`
- No cache eviction in v1 (logs are small text files, ~10-50KB each)

### CheckRun struct update

Add `id` field to the CheckRun struct and JQ query:

```rust
pub struct CheckRun {
    pub id: u64,  // NEW — needed for log fetching and cache key
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub details_url: Option<String>,
    pub html_url: Option<String>,
}
```

Update the JQ filter in `get_pr_check_runs` to include `id`:
```
{total_count: .total_count, check_runs: [.check_runs[] | {id: .id, name, status, ...}]}
```

### ANSI stripping

Logs from GitHub Actions contain ANSI escape sequences. Two options:
- **Strip entirely** in Rust before sending to frontend — simplest, loses color info
- **Convert to markup** — replace ANSI codes with `<span style="color:...">` in Rust, send as HTML

**Decision**: Convert to markup. The color information in compiler output (red for errors, blue for file paths, green for suggestions) is valuable for readability. Use a simple state machine in Rust that maps the common ANSI color codes (30-37, 90-97, bold, reset) to inline CSS styles. Unknown codes get stripped. The output is rendered via `{@html}` in Svelte — since the source is GitHub Actions logs (not user input), and we control the ANSI-to-HTML conversion, this is safe. The converter must HTML-escape the log text (`<`, `>`, `&`) before wrapping in spans.

## App Shell Integration

### Panel visibility

In `App.svelte`, when `showChecksView` is true:
- Collapse left panel (file tree) — set width to 0, same as split-diff mode
- Collapse right panel (annotation sidebar) — set width to 0
- `Cmd+B` / `Cmd+Shift+B` still work to manually toggle if desired

When `showChecksView` becomes false (user switches to another tab):
- Restore panels to their previous state

This follows the existing pattern where split-diff mode already auto-collapses the left panel.

### State flow

```
User clicks "Checks" tab
  → EditorPane sets showChecksView = true
  → App shell detects showChecksView, collapses panels
  → WorkspaceContentRouter renders ChecksView
  → ChecksView loads check runs, renders 3-column layout
  → User clicks a job → logs fetched (cache check → API → cache write) → terminal renders
  → User clicks file:line in error context → mini-diff overlay opens
  → User presses Escape → overlay closes
  → User clicks "Code" tab → showChecksView = false → panels restore
```

## Component Structure

```
ChecksView.svelte (3-column layout container)
├── ChecksJobList.svelte (left column)
│   ├── summary stats + filter tabs
│   └── scrollable job list
├── ChecksTerminal.svelte (center column)
│   ├── header bar
│   └── log body (ANSI → HTML rendered)
├── ChecksErrorContext.svelte (right column, collapsible)
│   ├── failing files list
│   └── file:line links
└── ChecksMiniDiff.svelte (overlay)
    ├── file header
    └── code view at target line
```

Breaking ChecksView into sub-components keeps each file focused and testable. The current single-file ChecksView.svelte gets replaced entirely.

## What's NOT in scope

- Log streaming for in-progress jobs (v1 fetches logs only for completed jobs)
- Cache eviction / size management
- Smart error parsing (structured error cards per compiler) — just file:line extraction
- Annotation creation from within Checks view
- Re-running failed jobs from the UI
