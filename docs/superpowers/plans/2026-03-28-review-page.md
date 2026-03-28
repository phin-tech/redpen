# Review Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a full-screen review feed view with two modes — "Review Changes" (diffs + annotations) and "Agent Feedback" (annotations only) — accessible via `Cmd+Shift+R` and the command palette.

**Architecture:** A new `ReviewPage.svelte` component replaces the editor pane when active, driven by a `reviewPage.svelte.ts` store. It reuses existing diff (`compute_diff`) and annotation (`get_annotations`, `get_all_annotations`) Tauri commands. A new `resolved` field on `Annotation` tracks resolve state. New Tauri command `read_file_lines` provides code snippets for the Agent Feedback mode.

**Tech Stack:** Svelte 5 (runes), TypeScript, Tauri commands (Rust), existing `compute_diff` / annotation APIs, CSS custom properties from the existing theme.

---

## File Structure

### New Files
| File | Responsibility |
|------|---------------|
| `src/lib/stores/reviewPage.svelte.ts` | Review page state: mode, active card, resolved IDs, data loading |
| `src/components/ReviewPage.svelte` | Top-level review feed layout (top bar, progress, scrollable feed) |
| `src/components/ReviewCard.svelte` | Single annotation card: code snippet, body, actions, reply |
| `src/components/ReviewCodeSnippet.svelte` | Code context with line numbers, highlighting, expand buttons |

### Modified Files
| File | Change |
|------|--------|
| `crates/redpen-core/src/annotation.rs` | Add `resolved: bool` field to `Annotation` |
| `src-tauri/src/commands/annotations.rs` | Add `resolved` to `update_annotation`, add `read_file_lines` command |
| `src-tauri/src/lib.rs` | Register `read_file_lines` command |
| `src/lib/tauri.ts` | Add `readFileLines`, `resolveAnnotation` wrappers |
| `src/lib/stores/annotations.svelte.ts` | Add `resolveAnnotation` function |
| `src/lib/commands.ts` | Add "Review Changes" and "Agent Feedback" commands |
| `src/components/EditorPane.svelte` | Conditionally render `ReviewPage` when review mode active |
| `src/App.svelte` | Add `Cmd+Shift+R` shortcut, wire command context |

---

### Task 1: Add `resolved` field to Annotation (Rust)

**Files:**
- Modify: `crates/redpen-core/src/annotation.rs:100-133`

- [ ] **Step 1: Add the `resolved` field to Annotation struct**

In `crates/redpen-core/src/annotation.rs`, add the field after `selection_mode`:

```rust
// After the selection_mode field, add:
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resolved: bool,
```

The full `Annotation` struct field list becomes:
```rust
pub struct Annotation {
    pub id: String,
    pub kind: AnnotationKind,
    pub body: String,
    pub labels: Vec<String>,
    pub author: String,
    pub is_orphaned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(
        default,
        deserialize_with = "flexible_datetime_deserialize",
        serialize_with = "flexible_datetime_serialize",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "string | null")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(
        default,
        deserialize_with = "flexible_datetime_deserialize",
        serialize_with = "flexible_datetime_serialize",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "string | null")]
    pub updated_at: Option<DateTime<Utc>>,
    pub anchor: Anchor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<Choice>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_mode: Option<SelectionMode>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resolved: bool,
}
```

- [ ] **Step 2: Update constructors to set `resolved: false`**

In `Annotation::new()` (line ~154) and `Annotation::new_reply()` (line ~178), add `resolved: false` to both `Self { ... }` blocks.

```rust
// In Annotation::new():
Self {
    id: Uuid::new_v4().to_string().to_uppercase(),
    kind,
    body,
    labels,
    author,
    is_orphaned: false,
    reply_to: None,
    created_at: Some(now),
    updated_at: Some(now),
    anchor,
    choices: None,
    selection_mode: None,
    resolved: false,
}

// In Annotation::new_reply():
Self {
    id: Uuid::new_v4().to_string().to_uppercase(),
    kind: AnnotationKind::Comment,
    body,
    labels: vec![],
    author,
    is_orphaned: false,
    reply_to: Some(reply_to),
    created_at: Some(now),
    updated_at: Some(now),
    anchor,
    choices: None,
    selection_mode: None,
    resolved: false,
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && cargo check -p redpen-core`
Expected: Compiles with no errors.

- [ ] **Step 4: Regenerate TypeScript bindings**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && cargo test -p redpen-core export_bindings_ -- --ignored 2>/dev/null; task generate:bindings 2>/dev/null || cargo test -p redpen-core 2>&1 | tail -5`

Check that `src/lib/bindings/Annotation.ts` now includes `resolved: boolean`.

- [ ] **Step 5: Run existing tests**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && cargo test -p redpen-core`
Expected: All tests pass. The `#[serde(default)]` attribute ensures existing JSON without `resolved` deserializes to `false`.

- [ ] **Step 6: Commit**

```bash
git add crates/redpen-core/src/annotation.rs src/lib/bindings/
git commit -m "feat: add resolved field to Annotation struct"
```

---

### Task 2: Add `read_file_lines` Tauri command and update `update_annotation` for resolve

**Files:**
- Modify: `src-tauri/src/commands/annotations.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/stores/annotations.svelte.ts`

- [ ] **Step 1: Add `read_file_lines` Tauri command**

In `src-tauri/src/commands/annotations.rs`, add at the bottom of the file:

```rust
#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FileSnippet {
    pub lines: Vec<String>,
    pub start_line: u32,
    pub total_lines: u32,
}

#[tauri::command]
pub fn read_file_lines(
    file_path: String,
    center_line: u32,
    context: u32,
) -> Result<FileSnippet, String> {
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len() as u32;

    let start = if center_line <= context + 1 {
        0u32
    } else {
        center_line - context - 1
    };
    let end = ((center_line + context) as usize).min(all_lines.len());

    let lines: Vec<String> = all_lines[start as usize..end]
        .iter()
        .map(|l| l.to_string())
        .collect();

    Ok(FileSnippet {
        lines,
        start_line: start + 1,
        total_lines,
    })
}
```

- [ ] **Step 2: Add `resolved` parameter to `update_annotation` command**

In `src-tauri/src/commands/annotations.rs`, find the `update_annotation` Tauri command. Add `resolved: Option<bool>` to its parameters and apply it when present:

```rust
// Add resolved parameter to update_annotation function signature:
#[tauri::command]
pub async fn update_annotation(
    state: tauri::State<'_, ...>,
    file_path: String,
    annotation_id: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
    choices: Option<Vec<Choice>>,
    resolved: Option<bool>,
) -> CommandResult<Annotation> {
```

After the existing field updates (body, labels, choices), add:
```rust
if let Some(r) = resolved {
    ann.resolved = r;
}
```

- [ ] **Step 3: Register `read_file_lines` in `src-tauri/src/lib.rs`**

Add `commands::annotations::read_file_lines` to the `tauri::generate_handler![]` macro.

- [ ] **Step 4: Add TypeScript wrappers in `src/lib/tauri.ts`**

```typescript
export interface FileSnippet {
  lines: string[];
  startLine: number;
  totalLines: number;
}

export async function readFileLines(
  filePath: string,
  centerLine: number,
  context: number,
): Promise<FileSnippet> {
  return invoke("read_file_lines", { filePath, centerLine, context });
}
```

Update `updateAnnotation` to accept `resolved`:
```typescript
export async function updateAnnotation(
  filePath: string, annotationId: string, body?: string, labels?: string[], choices?: Choice[], resolved?: boolean
): Promise<Annotation> {
  return invoke("update_annotation", { filePath, annotationId, body, labels, choices, resolved });
}
```

- [ ] **Step 5: Add `resolveAnnotation` to `src/lib/stores/annotations.svelte.ts`**

```typescript
export async function resolveAnnotation(filePath: string, annotationId: string, resolved: boolean) {
  const updated = await updateAnnotation(filePath, annotationId, undefined, undefined, undefined, resolved);
  if (state.sidecar) {
    state.sidecar.annotations = state.sidecar.annotations.map((a) =>
      a.id === annotationId ? updated : a
    );
  }
}
```

- [ ] **Step 6: Verify compilation**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && cargo check`
Expected: Compiles with no errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/annotations.rs src-tauri/src/lib.rs src/lib/tauri.ts src/lib/stores/annotations.svelte.ts src/lib/bindings/
git commit -m "feat: add read_file_lines command and resolve support to update_annotation"
```

---

### Task 3: Create review page store

**Files:**
- Create: `src/lib/stores/reviewPage.svelte.ts`

- [ ] **Step 1: Create the store file**

Create `src/lib/stores/reviewPage.svelte.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import { getAllAnnotations, getAnnotations, readFileLines } from "$lib/tauri";
import { getDiffState } from "$lib/stores/diff.svelte";
import { getWorkspace } from "$lib/stores/workspace.svelte";
import { getEditor } from "$lib/stores/editor.svelte";
import { getReviewSession } from "$lib/stores/review.svelte";
import type { Annotation, DiffResult, FileAnnotations } from "$lib/types";
import type { FileSnippet } from "$lib/tauri";

type ReviewMode = "changes" | "feedback";

const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

export interface ReviewFileData {
  filePath: string;
  fileName: string;
  annotations: Annotation[];
  diff: DiffResult | null;
  snippets: Map<string, FileSnippet>; // annotationId -> snippet
}

interface ReviewPageState {
  mode: ReviewMode | null; // null = closed
  activeCardIndex: number;
  files: ReviewFileData[];
  loading: boolean;
  error: string | null;
}

let state = $state<ReviewPageState>({
  mode: null,
  activeCardIndex: 0,
  files: [],
  loading: false,
  error: null,
});

export function getReviewPageState() {
  return state;
}

export function isReviewPageOpen(): boolean {
  return state.mode !== null;
}

export async function openReviewPage(mode: ReviewMode) {
  state.mode = mode;
  state.loading = true;
  state.error = null;
  state.activeCardIndex = 0;
  state.files = [];

  try {
    if (mode === "changes") {
      await loadReviewChanges();
    } else {
      await loadAgentFeedback();
    }
  } catch (e) {
    state.error = e instanceof Error ? e.message : String(e);
  } finally {
    state.loading = false;
  }
}

export function closeReviewPage() {
  state.mode = null;
  state.files = [];
  state.activeCardIndex = 0;
  state.error = null;
}

async function loadReviewChanges() {
  const workspace = getWorkspace();
  const directory = workspace.rootFolders[0];
  if (!directory) {
    state.error = "No workspace open";
    return;
  }

  const session = getReviewSession();
  const editor = getEditor();

  // Determine file list: session files, or current file
  let filePaths: string[] = [];
  if (session.active && session.files.length > 0) {
    filePaths = [...session.files];
  } else if (editor.currentFilePath) {
    filePaths = [editor.currentFilePath];
  } else {
    state.error = "No files to review";
    return;
  }

  const diffState = getDiffState();
  const baseRef = diffState.baseRef || "HEAD";
  const targetRef = diffState.targetRef || "working-tree";

  const results: ReviewFileData[] = [];

  for (const filePath of filePaths) {
    const fileName = filePath.split("/").pop() ?? filePath;

    // Load diff
    let diff: DiffResult | null = null;
    try {
      diff = await invoke<DiffResult>("compute_diff", {
        directory,
        filePath,
        baseRef,
        targetRef,
        algorithm: "patience",
      });
    } catch {
      // File may not have changes — that's ok
    }

    // Load annotations
    let annotations: Annotation[] = [];
    try {
      const sidecar = await getAnnotations(filePath);
      annotations = sidecar.annotations;
    } catch {
      // No annotations for this file
    }

    // Load snippets for annotations (for lines not covered by diff)
    const snippets = new Map<string, FileSnippet>();
    for (const ann of annotations) {
      try {
        const snippet = await readFileLines(filePath, ann.anchor.range.startLine, 3);
        snippets.set(ann.id, snippet);
      } catch {
        // Skip if file not readable
      }
    }

    results.push({ filePath, fileName, annotations, diff, snippets });
  }

  state.files = results;
}

async function loadAgentFeedback() {
  const workspace = getWorkspace();
  const directory = workspace.rootFolders[0];
  if (!directory) {
    state.error = "No workspace open";
    return;
  }

  // Load all annotations across the project
  let allFiles: FileAnnotations[] = [];
  try {
    allFiles = await getAllAnnotations(directory);
  } catch {
    state.error = "Failed to load annotations";
    return;
  }

  // Filter to files with agent annotations
  const results: ReviewFileData[] = [];

  for (const fileGroup of allFiles) {
    const agentAnnotations = fileGroup.annotations.filter((a) =>
      AGENT_AUTHORS.has(a.author.toLowerCase())
    );
    if (agentAnnotations.length === 0) continue;

    const fileName = fileGroup.filePath.split("/").pop() ?? fileGroup.filePath;

    // Load snippets for each annotation
    const snippets = new Map<string, FileSnippet>();
    for (const ann of agentAnnotations) {
      try {
        const snippet = await readFileLines(fileGroup.filePath, ann.anchor.range.startLine, 3);
        snippets.set(ann.id, snippet);
      } catch {
        // Skip
      }
    }

    results.push({
      filePath: fileGroup.filePath,
      fileName,
      annotations: agentAnnotations,
      diff: null,
      snippets,
    });
  }

  state.files = results;
}

// Navigation
export function setActiveCard(index: number) {
  const total = totalCards();
  if (index >= 0 && index < total) {
    state.activeCardIndex = index;
  }
}

export function nextCard() {
  setActiveCard(state.activeCardIndex + 1);
}

export function prevCard() {
  setActiveCard(state.activeCardIndex - 1);
}

function totalCards(): number {
  return state.files.reduce((sum, f) => sum + f.annotations.length, 0);
}

export function getTotalCards(): number {
  return totalCards();
}

export function getResolvedCount(): number {
  return state.files.reduce(
    (sum, f) => sum + f.annotations.filter((a) => a.resolved).length,
    0
  );
}

// Find which file and annotation corresponds to a flat card index
export function getCardAtIndex(index: number): { filePath: string; annotation: Annotation } | null {
  let cursor = 0;
  for (const file of state.files) {
    for (const ann of file.annotations) {
      if (cursor === index) {
        return { filePath: file.filePath, annotation: ann };
      }
      cursor++;
    }
  }
  return null;
}
```

- [ ] **Step 2: Verify no TypeScript errors**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && npx tsc --noEmit 2>&1 | head -20`
Expected: No errors related to the new store.

- [ ] **Step 3: Commit**

```bash
git add src/lib/stores/reviewPage.svelte.ts
git commit -m "feat: add review page store with changes and feedback modes"
```

---

### Task 4: Create `ReviewCodeSnippet.svelte`

**Files:**
- Create: `src/components/ReviewCodeSnippet.svelte`

- [ ] **Step 1: Create the component**

Create `src/components/ReviewCodeSnippet.svelte`:

```svelte
<script lang="ts">
  import type { FileSnippet } from "$lib/tauri";
  import type { DiffChange, DiffHunk } from "$lib/types";
  import { readFileLines } from "$lib/tauri";

  let {
    filePath,
    snippet,
    highlightLine,
    highlightEndLine,
    diffHunk,
    kindColor = "var(--accent)",
  }: {
    filePath: string;
    snippet: FileSnippet | null;
    highlightLine: number;
    highlightEndLine?: number;
    diffHunk?: DiffHunk | null;
    kindColor?: string;
  } = $props();

  let expandedAbove = $state(0);
  let expandedBelow = $state(0);
  let extraLinesAbove = $state<string[]>([]);
  let extraLinesBelow = $state<string[]>([]);

  const endLine = $derived(highlightEndLine ?? highlightLine);

  async function expandAbove() {
    if (!snippet) return;
    const newStart = Math.max(1, snippet.startLine - expandedAbove - 5);
    try {
      const extra = await readFileLines(filePath, newStart + 2, 2);
      const needed = snippet.startLine - expandedAbove - newStart;
      extraLinesAbove = [...extra.lines.slice(0, needed), ...extraLinesAbove];
      expandedAbove += needed;
    } catch { /* ignore */ }
  }

  async function expandBelow() {
    if (!snippet) return;
    const currentEnd = snippet.startLine + snippet.lines.length - 1 + expandedBelow;
    if (currentEnd >= snippet.totalLines) return;
    try {
      const extra = await readFileLines(filePath, currentEnd + 3, 2);
      const needed = Math.min(5, snippet.totalLines - currentEnd);
      extraLinesBelow = [...extraLinesBelow, ...extra.lines.slice(0, needed)];
      expandedBelow += needed;
    } catch { /* ignore */ }
  }

  // Build display lines from snippet or diff hunk
  interface DisplayLine {
    lineNum: number | null;
    content: string;
    highlighted: boolean;
    changeKind?: "insert" | "delete" | "equal";
  }

  const displayLines = $derived.by((): DisplayLine[] => {
    if (diffHunk) {
      return diffHunk.changes.map((change) => ({
        lineNum: change.newLine ?? change.oldLine ?? null,
        content: change.content.replace(/\n$/, ""),
        highlighted:
          change.newLine !== null &&
          change.newLine >= highlightLine &&
          change.newLine <= endLine,
        changeKind: change.kind as "insert" | "delete" | "equal",
      }));
    }

    if (!snippet) return [];

    const allLines = [...extraLinesAbove, ...snippet.lines, ...extraLinesBelow];
    const startNum = snippet.startLine - expandedAbove;

    return allLines.map((content, i) => {
      const lineNum = startNum + i;
      return {
        lineNum,
        content,
        highlighted: lineNum >= highlightLine && lineNum <= endLine,
      };
    });
  });

  const canExpandAbove = $derived(
    snippet !== null && snippet.startLine - expandedAbove > 1
  );
  const canExpandBelow = $derived(
    snippet !== null &&
    snippet.startLine + snippet.lines.length - 1 + expandedBelow < snippet.totalLines
  );
</script>

<div class="review-snippet">
  {#if canExpandAbove && !diffHunk}
    <button class="snippet-expand" onclick={expandAbove}>
      ··· expand above ···
    </button>
  {/if}

  {#each displayLines as line}
    <div
      class="snippet-line"
      class:snippet-highlighted={line.highlighted}
      class:snippet-insert={line.changeKind === "insert"}
      class:snippet-delete={line.changeKind === "delete"}
      style:--kind-highlight={kindColor}
    >
      <span class="snippet-linenum">
        {line.lineNum ?? ""}
      </span>
      <span class="snippet-content">{line.content}</span>
    </div>
  {/each}

  {#if canExpandBelow && !diffHunk}
    <button class="snippet-expand" onclick={expandBelow}>
      ··· expand below ···
    </button>
  {/if}
</div>

<style>
  .review-snippet {
    background: color-mix(in srgb, var(--surface-base) 70%, black);
    font-family: "SF Mono", "Fira Code", Consolas, monospace;
    font-size: 12px;
    line-height: 1.6;
    overflow-x: auto;
    border-bottom: 1px solid var(--border-default);
  }
  .snippet-line {
    display: flex;
    padding: 0 12px;
  }
  .snippet-highlighted {
    background: color-mix(in srgb, var(--kind-highlight) 12%, transparent);
  }
  .snippet-insert {
    background: color-mix(in srgb, var(--success) 10%, transparent);
  }
  .snippet-delete {
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    opacity: 0.7;
  }
  .snippet-linenum {
    color: var(--text-muted);
    min-width: 36px;
    text-align: right;
    padding-right: 12px;
    user-select: none;
    flex-shrink: 0;
  }
  .snippet-content {
    white-space: pre;
    color: var(--text-secondary);
  }
  .snippet-highlighted .snippet-content {
    color: var(--text-primary);
  }
  .snippet-expand {
    display: block;
    width: 100%;
    text-align: center;
    padding: 4px;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    background: color-mix(in srgb, var(--surface-base) 50%, transparent);
    border: none;
    font-family: inherit;
  }
  .snippet-expand:hover {
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--surface-raised) 50%, transparent);
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ReviewCodeSnippet.svelte
git commit -m "feat: add ReviewCodeSnippet component for review page"
```

---

### Task 5: Create `ReviewCard.svelte`

**Files:**
- Create: `src/components/ReviewCard.svelte`

- [ ] **Step 1: Create the component**

Create `src/components/ReviewCard.svelte`:

```svelte
<script lang="ts">
  import type { Annotation, AnnotationKind, DiffHunk } from "$lib/types";
  import type { FileSnippet } from "$lib/tauri";
  import ReviewCodeSnippet from "./ReviewCodeSnippet.svelte";
  import { Bot } from "lucide-svelte";

  const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

  const KIND_COLORS: Record<AnnotationKind, string> = {
    comment: "var(--kind-comment-border, #89b4fa)",
    lineNote: "var(--kind-linenote-border, #f9e2af)",
    explanation: "var(--kind-explanation-border, #cba6f7)",
    label: "var(--kind-label-border, #a6e3a1)",
  };

  const KIND_LABELS: Record<AnnotationKind, string> = {
    comment: "comment",
    lineNote: "note",
    explanation: "explanation",
    label: "label",
  };

  let {
    annotation,
    replies = [],
    filePath,
    snippet,
    diffHunk,
    isActive = false,
    onReply,
    onResolve,
    onJumpToFile,
    onChoiceToggle,
  }: {
    annotation: Annotation;
    replies?: Annotation[];
    filePath: string;
    snippet: FileSnippet | null;
    diffHunk?: DiffHunk | null;
    isActive?: boolean;
    onReply: (annotationId: string, body: string) => void;
    onResolve: (annotationId: string, resolved: boolean) => void;
    onJumpToFile: (filePath: string, line: number) => void;
    onChoiceToggle: (annotationId: string, choiceIndex: number) => void;
  } = $props();

  let replyText = $state("");
  let replyInputRef: HTMLInputElement | undefined = $state(undefined);

  const kindColor = $derived(KIND_COLORS[annotation.kind]);
  const isAgent = $derived(AGENT_AUTHORS.has(annotation.author.toLowerCase()));

  function submitReply() {
    if (!replyText.trim()) return;
    onReply(annotation.id, replyText.trim());
    replyText = "";
  }

  function handleReplyKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submitReply();
    }
    // Stop j/k navigation when typing
    e.stopPropagation();
  }

  export function focusReply() {
    replyInputRef?.focus();
  }

  function relativeTime(dateStr: string | null | undefined): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }
</script>

<div
  class="review-card"
  class:review-card-active={isActive}
  class:review-card-resolved={annotation.resolved}
>
  <div class="review-card-kind-strip" style:background={kindColor}></div>

  <ReviewCodeSnippet
    {filePath}
    {snippet}
    highlightLine={annotation.anchor.range.startLine}
    highlightEndLine={annotation.anchor.range.endLine}
    {diffHunk}
    kindColor={kindColor}
  />

  <div class="review-card-body">
    <div class="review-card-header">
      <span class="review-card-author">
        {#if isAgent}<Bot size={14} class="review-card-agent-icon" />{/if}
        {annotation.author}
      </span>
      <span class="review-card-badge" style:background="color-mix(in srgb, {kindColor} 15%, transparent)" style:color={kindColor}>
        {KIND_LABELS[annotation.kind]}
      </span>
      <span class="review-card-time">{relativeTime(annotation.createdAt)}</span>
    </div>

    <div class="review-card-text">{annotation.body}</div>

    {#if annotation.choices && annotation.choices.length > 0}
      <div class="review-card-choices">
        {#each annotation.choices as choice, i}
          <label
            class="review-card-choice"
            class:review-card-choice-selected={choice.selected}
          >
            <input
              type={annotation.selectionMode === "multi" ? "checkbox" : "radio"}
              name="review-choice-{annotation.id}"
              checked={choice.selected}
              onchange={() => onChoiceToggle(annotation.id, i)}
            />
            <span>{choice.label}</span>
          </label>
        {/each}
      </div>
    {/if}

    {#if annotation.labels.length > 0}
      <div class="review-card-labels">
        {#each annotation.labels as label}
          <span class="review-card-label">{label}</span>
        {/each}
      </div>
    {/if}
  </div>

  {#each replies as reply}
    <div class="review-card-reply">
      <span class="review-card-reply-indicator">↳</span>
      <span class="review-card-author" style="font-size: 12px">
        {#if AGENT_AUTHORS.has(reply.author.toLowerCase())}<Bot size={12} class="review-card-agent-icon" />{/if}
        {reply.author}
      </span>
      <span class="review-card-reply-text">{reply.body}</span>
    </div>
  {/each}

  <div class="review-card-actions">
    <input
      bind:this={replyInputRef}
      class="review-card-reply-input"
      placeholder="Reply..."
      bind:value={replyText}
      onkeydown={handleReplyKeydown}
    />
    <button
      class="review-card-action-btn"
      class:review-card-action-resolved={annotation.resolved}
      onclick={() => onResolve(annotation.id, !annotation.resolved)}
    >
      {annotation.resolved ? "✓ Resolved" : "✓ Resolve"}
    </button>
    <button
      class="review-card-action-btn review-card-action-jump"
      onclick={() => onJumpToFile(filePath, annotation.anchor.range.startLine)}
      title="Open in editor"
    >
      Open file →
    </button>
  </div>
</div>

<style>
  .review-card {
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    margin: 8px 0;
    overflow: hidden;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .review-card:hover {
    border-color: color-mix(in srgb, var(--border-default) 80%, white);
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.2);
  }
  .review-card-active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent), 0 2px 12px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .review-card-resolved {
    opacity: 0.5;
  }
  .review-card-kind-strip {
    height: 3px;
  }
  .review-card-body {
    padding: 12px 16px;
  }
  .review-card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .review-card-author {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.review-card-agent-icon) {
    color: var(--accent);
  }
  .review-card-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .review-card-time {
    font-size: 11px;
    color: var(--text-muted);
    margin-left: auto;
  }
  .review-card-text {
    color: var(--text-primary);
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .review-card-choices {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .review-card-choice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .review-card-choice:hover {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }
  .review-card-choice-selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--text-primary);
  }
  .review-card-labels {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }
  .review-card-label {
    font-size: 10px;
    background: var(--surface-raised);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 4px;
  }
  .review-card-reply {
    padding: 8px 16px 10px;
    border-top: 1px solid var(--border-default);
    background: color-mix(in srgb, var(--surface-panel) 95%, white);
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: baseline;
    gap: 4px;
  }
  .review-card-reply-indicator {
    color: var(--text-muted);
  }
  .review-card-reply-text {
    flex: 1;
  }
  .review-card-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-top: 1px solid var(--border-default);
    background: color-mix(in srgb, var(--surface-panel) 98%, white);
  }
  .review-card-reply-input {
    flex: 1;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 5px;
    padding: 4px 10px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
  }
  .review-card-reply-input::placeholder {
    color: var(--text-muted);
  }
  .review-card-reply-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .review-card-action-btn {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 5px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .review-card-action-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
    background: var(--surface-raised);
  }
  .review-card-action-resolved {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 12%, transparent);
    border-color: color-mix(in srgb, var(--success) 30%, transparent);
  }
  .review-card-action-jump {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .review-card-action-jump:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ReviewCard.svelte
git commit -m "feat: add ReviewCard component for review page"
```

---

### Task 6: Create `ReviewPage.svelte`

**Files:**
- Create: `src/components/ReviewPage.svelte`

- [ ] **Step 1: Create the component**

Create `src/components/ReviewPage.svelte`:

```svelte
<script lang="ts">
  import {
    getReviewPageState,
    closeReviewPage,
    nextCard,
    prevCard,
    setActiveCard,
    getTotalCards,
    getResolvedCount,
    getCardAtIndex,
  } from "$lib/stores/reviewPage.svelte";
  import { resolveAnnotation } from "$lib/stores/annotations.svelte";
  import { updateChoices } from "$lib/stores/annotations.svelte";
  import { addAnnotation } from "$lib/stores/annotations.svelte";
  import ReviewCard from "./ReviewCard.svelte";
  import type { Annotation, DiffHunk } from "$lib/types";

  let {
    onJumpToFile,
  }: {
    onJumpToFile: (filePath: string, line: number) => void;
  } = $props();

  const state = getReviewPageState();

  const modeLabel = $derived(
    state.mode === "changes" ? "Review Changes" : "Agent Feedback"
  );

  const totalCards = $derived(getTotalCards());
  const resolvedCount = $derived(getResolvedCount());
  const progressPercent = $derived(
    totalCards > 0 ? Math.round((resolvedCount / totalCards) * 100) : 0
  );

  // Build a flat list of cards with their metadata for rendering
  interface CardEntry {
    annotation: Annotation;
    replies: Annotation[];
    filePath: string;
    flatIndex: number;
  }

  const cardEntries = $derived.by((): { filePath: string; fileName: string; cards: CardEntry[]; jumpLine: number }[] => {
    const result: { filePath: string; fileName: string; cards: CardEntry[]; jumpLine: number }[] = [];
    let flatIndex = 0;

    for (const file of state.files) {
      const roots = file.annotations.filter((a) => !a.replyTo);
      const replyMap = new Map<string, Annotation[]>();
      for (const a of file.annotations) {
        if (a.replyTo) {
          const group = replyMap.get(a.replyTo) ?? [];
          group.push(a);
          replyMap.set(a.replyTo, group);
        }
      }

      const cards: CardEntry[] = [];
      for (const root of roots) {
        cards.push({
          annotation: root,
          replies: replyMap.get(root.id) ?? [],
          filePath: file.filePath,
          flatIndex,
        });
        flatIndex++;
      }

      if (cards.length > 0) {
        const jumpLine = roots[0]?.anchor.range.startLine ?? 1;
        result.push({ filePath: file.filePath, fileName: file.fileName, cards, jumpLine });
      }
    }

    return result;
  });

  // Keyboard navigation
  let cardRefs: Map<number, { focusReply: () => void }> = new Map();

  function handleKeydown(e: KeyboardEvent) {
    // Don't intercept when typing in an input
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      if (e.key === "Escape") {
        (e.target as HTMLElement).blur();
        e.preventDefault();
      }
      return;
    }

    if (e.key === "j") {
      e.preventDefault();
      nextCard();
      scrollToActive();
    } else if (e.key === "k") {
      e.preventDefault();
      prevCard();
      scrollToActive();
    } else if (e.key === "r") {
      e.preventDefault();
      cardRefs.get(state.activeCardIndex)?.focusReply();
    } else if (e.key === "e") {
      e.preventDefault();
      const card = getCardAtIndex(state.activeCardIndex);
      if (card) {
        resolveAnnotation(card.filePath, card.annotation.id, !card.annotation.resolved);
      }
    } else if (e.key === "o") {
      e.preventDefault();
      const card = getCardAtIndex(state.activeCardIndex);
      if (card) {
        handleJumpToFile(card.filePath, card.annotation.anchor.range.startLine);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeReviewPage();
    }
  }

  function scrollToActive() {
    const el = document.querySelector(`[data-review-card="${state.activeCardIndex}"]`);
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  function handleJumpToFile(filePath: string, line: number) {
    closeReviewPage();
    onJumpToFile(filePath, line);
  }

  async function handleReply(annotationId: string, body: string) {
    // Find the file path for this annotation
    for (const file of state.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann) {
        await addAnnotation(
          file.filePath,
          body,
          [],
          ann.anchor.range.startLine,
          ann.anchor.range.startColumn,
          ann.anchor.range.endLine,
          ann.anchor.range.endColumn,
        );
        break;
      }
    }
  }

  async function handleResolve(annotationId: string, resolved: boolean) {
    for (const file of state.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann) {
        await resolveAnnotation(file.filePath, annotationId, resolved);
        // Update local state
        ann.resolved = resolved;
        break;
      }
    }
  }

  async function handleChoiceToggle(annotationId: string, choiceIndex: number) {
    for (const file of state.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann && ann.choices) {
        const newChoices = ann.choices.map((c, i) => {
          if (ann.selectionMode === "single") {
            return { ...c, selected: i === choiceIndex };
          }
          return i === choiceIndex ? { ...c, selected: !c.selected } : c;
        });
        await updateChoices(file.filePath, annotationId, newChoices);
        ann.choices = newChoices;
        break;
      }
    }
  }

  // Find the diff hunk that covers a given line
  function findHunkForLine(file: typeof state.files[0], line: number): DiffHunk | null {
    if (!file.diff) return null;
    for (const hunk of file.diff.hunks) {
      const hunkEnd = hunk.newStart + hunk.newCount - 1;
      if (line >= hunk.newStart && line <= hunkEnd) {
        return hunk;
      }
    }
    return null;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="review-page">
  <!-- Top bar -->
  <div class="review-topbar">
    <span class="review-topbar-title">{modeLabel}</span>
    <span class="review-topbar-count">{totalCards} annotations</span>
    {#if resolvedCount > 0}
      <span class="review-topbar-resolved">{resolvedCount} resolved</span>
    {/if}
    <span class="review-topbar-spacer"></span>
    <button class="review-topbar-close" onclick={closeReviewPage}>
      Close <kbd>Esc</kbd>
    </button>
  </div>

  <!-- Progress bar -->
  <div class="review-progress">
    <div class="review-progress-fill" style:width="{progressPercent}%"></div>
  </div>

  {#if state.loading}
    <div class="review-status">Loading...</div>
  {:else if state.error}
    <div class="review-status">
      <p>{state.error}</p>
      <button class="review-close-btn" onclick={closeReviewPage}>Close</button>
    </div>
  {:else if cardEntries.length === 0}
    <div class="review-status">
      <p>No annotations to review</p>
      <button class="review-close-btn" onclick={closeReviewPage}>Close</button>
    </div>
  {:else}
    <div class="review-feed">
      {#each cardEntries as fileGroup}
        <div class="review-file-header">
          <span class="review-file-icon">📄</span>
          <span class="review-file-name">{fileGroup.fileName}</span>
          <span class="review-file-count">{fileGroup.cards.length}</span>
          <button
            class="review-file-jump"
            onclick={() => handleJumpToFile(fileGroup.filePath, fileGroup.jumpLine)}
          >
            Open in editor →
          </button>
        </div>

        {#each fileGroup.cards as entry (entry.annotation.id)}
          <div data-review-card={entry.flatIndex}>
            <ReviewCard
              bind:this={cardRefs[entry.flatIndex]}
              annotation={entry.annotation}
              replies={entry.replies}
              filePath={entry.filePath}
              snippet={state.files.find(f => f.filePath === entry.filePath)?.snippets.get(entry.annotation.id) ?? null}
              diffHunk={findHunkForLine(
                state.files.find(f => f.filePath === entry.filePath)!,
                entry.annotation.anchor.range.startLine
              )}
              isActive={state.activeCardIndex === entry.flatIndex}
              onReply={handleReply}
              onResolve={handleResolve}
              onJumpToFile={handleJumpToFile}
              onChoiceToggle={handleChoiceToggle}
            />
          </div>
        {/each}
      {/each}
    </div>
  {/if}

  <!-- Keyboard hint bar -->
  <div class="review-nav-hint">
    <span><kbd>j</kbd> / <kbd>k</kbd> next / prev</span>
    <span><kbd>r</kbd> reply</span>
    <span><kbd>e</kbd> resolve</span>
    <span><kbd>o</kbd> open in editor</span>
    <span><kbd>Esc</kbd> close</span>
  </div>
</div>

<style>
  .review-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
    overflow: hidden;
  }

  /* Top bar */
  .review-topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 20px;
    background: var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    flex-shrink: 0;
    z-index: 2;
  }
  .review-topbar-title {
    font-weight: 600;
    font-size: 15px;
    color: var(--text-primary);
  }
  .review-topbar-count {
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--surface-raised);
    padding: 2px 8px;
    border-radius: 10px;
  }
  .review-topbar-resolved {
    font-size: 12px;
    color: var(--success);
  }
  .review-topbar-spacer {
    flex: 1;
  }
  .review-topbar-close {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    border-radius: 6px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .review-topbar-close:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }
  .review-topbar-close kbd {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 10px;
  }

  /* Progress bar */
  .review-progress {
    height: 3px;
    background: var(--surface-raised);
    flex-shrink: 0;
  }
  .review-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
    border-radius: 0 2px 2px 0;
  }

  /* Feed */
  .review-feed {
    flex: 1;
    overflow-y: auto;
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
    padding: 12px 24px 100px;
  }

  /* File header */
  .review-file-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 0 8px;
    font-size: 12px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-default);
    margin-bottom: 8px;
  }
  .review-file-header:first-child {
    padding-top: 4px;
  }
  .review-file-icon {
    opacity: 0.5;
  }
  .review-file-name {
    color: var(--text-secondary);
    font-weight: 500;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 12px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .review-file-count {
    background: var(--surface-raised);
    padding: 1px 7px;
    border-radius: 8px;
    font-variant-numeric: tabular-nums;
  }
  .review-file-jump {
    color: var(--accent);
    font-size: 11px;
    cursor: pointer;
    border: none;
    background: none;
    font-family: inherit;
  }
  .review-file-jump:hover {
    text-decoration: underline;
  }

  /* Status */
  .review-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 12px;
    flex: 1;
  }
  .review-close-btn {
    padding: 6px 16px;
    border-radius: 6px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
  }

  /* Keyboard hint */
  .review-nav-hint {
    position: sticky;
    bottom: 0;
    background: var(--surface-panel);
    border-top: 1px solid var(--border-default);
    padding: 8px 16px;
    font-size: 12px;
    color: var(--text-muted);
    display: flex;
    justify-content: center;
    gap: 16px;
    flex-shrink: 0;
    z-index: 2;
  }
  .review-nav-hint kbd {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: inherit;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ReviewPage.svelte
git commit -m "feat: add ReviewPage component with feed layout and keyboard nav"
```

---

### Task 7: Wire ReviewPage into EditorPane

**Files:**
- Modify: `src/components/EditorPane.svelte`

- [ ] **Step 1: Import review page state and component**

At the top of `EditorPane.svelte`'s `<script>` block, add:

```typescript
import ReviewPage from "./ReviewPage.svelte";
import { isReviewPageOpen } from "$lib/stores/reviewPage.svelte";
```

- [ ] **Step 2: Add `onJumpToFile` prop**

Add to the props destructuring:

```typescript
let {
  onSelectionChange,
  onJumpToFile,
  ref = $bindable(undefined),
}: {
  onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
  onJumpToFile?: (filePath: string, line: number) => void;
  ref?: { scrollToLine: (line: number) => void; openSearch: () => void; closeSearch: () => void; navigateMatch: (dir: 1 | -1) => void } | undefined;
} = $props();
```

- [ ] **Step 3: Add conditional render in pane-content**

In the `<div class="pane-content">` section, add a check at the very top of the conditional chain:

```svelte
<div class="pane-content">
  {#if isReviewPageOpen()}
    <ReviewPage onJumpToFile={(path, line) => onJumpToFile?.(path, line)} />
  {:else if diff.enabled && diff.loading}
    <!-- ... existing diff/editor conditions unchanged ... -->
```

- [ ] **Step 4: Commit**

```bash
git add src/components/EditorPane.svelte
git commit -m "feat: wire ReviewPage into EditorPane conditional rendering"
```

---

### Task 8: Add commands and keyboard shortcut

**Files:**
- Modify: `src/lib/commands.ts`
- Modify: `src/App.svelte`

- [ ] **Step 1: Add review commands to the registry**

In `src/lib/commands.ts`, add to the `AppCommandContext` interface:

```typescript
export interface AppCommandContext {
  // ... existing methods ...
  openReviewChanges: () => void;
  openAgentFeedback: () => void;
  isReviewPageOpen: () => boolean;
}
```

Add `"Review"` to `COMMAND_SECTIONS`:

```typescript
export const COMMAND_SECTIONS = [
  "Navigation",
  "Workspace",
  "Annotations",
  "View",
  "Diff",
  "Review",
] as const;
```

Add two commands to `createCommandRegistry()` return array:

```typescript
{
  id: "review.changes",
  title: "Review Changes",
  section: "Review",
  keywords: ["review", "changes", "diff", "feed"],
  shortcut: ["Cmd", "Shift", "R"],
  isEnabled: (context) => context.hasRoots() && !context.isReviewPageOpen(),
  run: (context) => context.openReviewChanges(),
},
{
  id: "review.feedback",
  title: "Agent Feedback",
  section: "Review",
  keywords: ["review", "agent", "feedback", "questions"],
  isEnabled: (context) => context.hasRoots() && !context.isReviewPageOpen(),
  run: (context) => context.openAgentFeedback(),
},
```

- [ ] **Step 2: Wire up in App.svelte**

In `src/App.svelte`, add imports:

```typescript
import { openReviewPage, closeReviewPage, isReviewPageOpen } from "$lib/stores/reviewPage.svelte";
```

Add to `commandContext`:

```typescript
openReviewChanges: () => openReviewPage("changes"),
openAgentFeedback: () => openReviewPage("feedback"),
isReviewPageOpen: () => isReviewPageOpen(),
```

Add `onJumpToFile` prop to `EditorPane`:

```svelte
<EditorPane
  bind:ref={editorRef}
  onSelectionChange={handleSelectionChange}
  onJumpToFile={async (path, line) => {
    await handleFileSelect(path);
    setTimeout(() => editorRef?.scrollToLine(line), 100);
  }}
/>
```

- [ ] **Step 3: Add keyboard shortcut**

In `handleKeydown` in `App.svelte`, add before the existing shortcuts:

```typescript
if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "r") {
  e.preventDefault();
  if (isReviewPageOpen()) {
    closeReviewPage();
  } else {
    void runCommand("review.changes");
  }
  return;
}
```

Also update the `Escape` handler to close the review page:

```typescript
if (e.key === "Escape") {
  if (showCommandPalette) {
    e.preventDefault();
    showCommandPalette = false;
    return;
  }
  if (isReviewPageOpen()) {
    e.preventDefault();
    closeReviewPage();
    return;
  }
}
```

- [ ] **Step 4: Verify the app compiles**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && npm run check 2>&1 | tail -10`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/commands.ts src/App.svelte
git commit -m "feat: add review page commands and Cmd+Shift+R shortcut"
```

---

### Task 9: Build and manually test

**Files:** None (testing only)

- [ ] **Step 1: Build the full app**

Run: `cd /Users/sphinizy/src/github.com/phin-tech/redpen && task dev:install`

- [ ] **Step 2: Test Review Changes mode**

1. Open a workspace with annotations in the app
2. Press `Cmd+Shift+R` — review page should appear
3. Verify: file headers, code snippets, annotation cards render
4. Verify: `j`/`k` navigation moves active card highlight
5. Verify: `Esc` closes review page, returns to editor
6. Press `Cmd+K`, type "Review" — both commands should appear

- [ ] **Step 3: Test Agent Feedback mode**

1. Open command palette, select "Agent Feedback"
2. Verify: only agent annotations appear
3. Verify: code snippets show around each annotation

- [ ] **Step 4: Test actions**

1. Click "Resolve" on a card — card should dim
2. Type in reply input, press Enter — reply should appear
3. Click "Open in editor" — should exit review, open file at line
4. If annotations have choices, click a choice — should persist

- [ ] **Step 5: Fix any issues found during testing**

Address any visual or functional issues discovered.

- [ ] **Step 6: Commit any fixes**

```bash
git add -A
git commit -m "fix: review page polish from manual testing"
```

---

### Task 10: Add Review button to sidebar

**Files:**
- Modify: `src/components/AnnotationSidebar.svelte`

- [ ] **Step 1: Add import and button**

In `src/components/AnnotationSidebar.svelte`, add import:

```typescript
import { openReviewPage } from "$lib/stores/reviewPage.svelte";
```

In the bottom action bar section (the `{#if annotationsState.sidebarView === 'file' && editor.currentFilePath}` block), add a "Review" button row above the Approve/Request Changes buttons:

```svelte
{#if annotationsState.sidebarView === 'file' && editor.currentFilePath}
  <div class="px-2.5 py-2 border-t border-border-default/60" style="box-shadow: var(--shadow-xs)">
    <div class="flex gap-2 mb-2">
      <Button variant="secondary" onclick={() => openReviewPage("changes")} class="flex-1 text-xs">
        Review
      </Button>
    </div>
    {#if reviewDone}
      <!-- ... existing review done UI ... -->
```

- [ ] **Step 2: Commit**

```bash
git add src/components/AnnotationSidebar.svelte
git commit -m "feat: add Review button to annotation sidebar"
```
