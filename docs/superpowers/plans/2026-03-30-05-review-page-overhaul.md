# Review Page Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce vertical scrolling in the Review feed by proximity-merging annotations that share nearby code, removing redundant "Open file" per card, hiding reply input until activated, and adding hover-sync between L-tags and code lines.

**Architecture:** Add a proximity grouping step to ReviewPage.svelte's `cardEntries` derivation that merges annotations within 5 lines into a single context block. The shared snippet spans `min(lines)-1` to `max(lines)+1`. Individual threads are stacked below the snippet. ReviewCard is refactored to support both standalone and grouped rendering.

**Tech Stack:** Svelte 5, TypeScript, CSS

---

### Task 1: Add Proximity Grouping to ReviewPage

**Files:**
- Modify: `src/components/ReviewPage.svelte`

- [ ] **Step 1: Define grouped entry types**

Replace or extend the `CardEntry` interface to support groups:

```typescript
interface ThreadEntry {
  annotation: Annotation;
  replies: Annotation[];
  flatIndex: number;
}

interface ContextGroup {
  threads: ThreadEntry[];
  filePath: string;
  /** Snippet covers from min annotated line - 1 to max annotated line + 1 */
  snippetStartLine: number;
  snippetEndLine: number;
  /** First flatIndex in the group — for keyboard nav */
  startFlatIndex: number;
}

// A file group now contains ContextGroups instead of flat CardEntries
interface FileGroup {
  filePath: string;
  fileName: string;
  groups: ContextGroup[];
  jumpLine: number;
  diffOnlyIndex: number | null;
}
```

- [ ] **Step 2: Implement proximity grouping logic**

In the `cardEntries` derivation, after building root annotations with replies, group them by proximity:

```typescript
const PROXIMITY_THRESHOLD = 5;

function groupByProximity(cards: { annotation: Annotation; replies: Annotation[]; flatIndex: number }[], filePath: string): ContextGroup[] {
  if (cards.length === 0) return [];

  // Sort by line
  const sorted = [...cards].sort((a, b) =>
    a.annotation.anchor.range.startLine - b.annotation.anchor.range.startLine
  );

  const groups: ContextGroup[] = [];
  let currentGroup: typeof sorted = [sorted[0]];

  for (let i = 1; i < sorted.length; i++) {
    const prevLine = currentGroup[currentGroup.length - 1].annotation.anchor.range.startLine;
    const currLine = sorted[i].annotation.anchor.range.startLine;

    if (currLine - prevLine <= PROXIMITY_THRESHOLD) {
      currentGroup.push(sorted[i]);
    } else {
      groups.push(buildContextGroup(currentGroup, filePath));
      currentGroup = [sorted[i]];
    }
  }
  groups.push(buildContextGroup(currentGroup, filePath));

  return groups;
}

function buildContextGroup(threads: typeof sorted, filePath: string): ContextGroup {
  const lines = threads.map(t => t.annotation.anchor.range.startLine);
  const minLine = Math.max(1, Math.min(...lines) - 1);
  const maxLine = Math.max(...lines) + 1;
  return {
    threads: threads.map(t => ({
      annotation: t.annotation,
      replies: t.replies,
      flatIndex: t.flatIndex,
    })),
    filePath,
    snippetStartLine: minLine,
    snippetEndLine: maxLine,
    startFlatIndex: threads[0].flatIndex,
  };
}
```

- [ ] **Step 3: Update the template to render grouped context blocks**

Replace the current `{#each fileGroup.cards as entry}` loop with:

```svelte
{#each fileGroup.groups as group}
  <div class="review-context-block" class:review-card-active={activeIndex >= group.startFlatIndex && activeIndex < group.startFlatIndex + group.threads.length}>
    <!-- Shared snippet -->
    <ReviewCodeSnippet
      filePath={fileGroup.filePath}
      snippet={buildGroupSnippet(group)}
      highlightLine={0}
      diffHunk={null}
      annotatedLines={group.threads.map(t => ({
        line: t.annotation.anchor.range.startLine,
        resolved: t.annotation.resolved,
      }))}
    />

    <!-- Stacked threads -->
    {#each group.threads as thread (thread.annotation.id)}
      <div
        data-review-card={thread.flatIndex}
        class="review-thread"
        class:review-thread-active={activeIndex === thread.flatIndex}
        class:review-thread-resolved={thread.annotation.resolved}
      >
        <!-- Thread content: L-tag, author, body, replies, resolve button -->
      </div>
    {/each}
  </div>
{/each}
```

- [ ] **Step 4: Build the group snippet**

Add a function to build the snippet for a group from the file's snippet map:

```typescript
function buildGroupSnippet(group: ContextGroup): FileSnippet | null {
  // Use the first thread's snippet as a base, or request lines
  const file = reviewState.files.find(f => f.filePath === group.filePath);
  if (!file) return null;
  // Find any available snippet and expand to cover the group range
  for (const thread of group.threads) {
    const snippet = file.snippets.get(thread.annotation.id);
    if (snippet) return snippet; // Use existing snippet — ReviewCodeSnippet handles range
  }
  return null;
}
```

- [ ] **Step 5: Build and verify**

Run: `npm run build`

- [ ] **Step 6: Commit**

```bash
git commit -m "feat: proximity-merge annotations within 5 lines into shared context blocks"
```

---

### Task 2: Refactor Thread Rendering (Remove Per-Card Redundancy)

**Files:**
- Modify: `src/components/ReviewPage.svelte` (inline thread rendering, or create ReviewThread component)

- [ ] **Step 1: Render threads inline instead of using ReviewCard**

Each thread within a group renders as a compact section:

```svelte
<div class="review-thread" data-review-card={thread.flatIndex}>
  <div class="review-thread-header">
    <span class="review-thread-line-tag">L{thread.annotation.anchor.range.startLine}</span>
    <span class="review-thread-author">{thread.annotation.author}</span>
    {#if thread.annotation.kind !== "comment"}
      <span class="review-thread-kind">{KIND_LABELS[thread.annotation.kind]}</span>
    {/if}
    <span class="review-thread-time">{relativeTime(thread.annotation.createdAt)}</span>
  </div>
  <div class="review-thread-body">{thread.annotation.body}</div>

  <!-- Replies always visible inline -->
  {#each thread.replies as reply}
    <div class="review-thread-reply">
      <span class="review-thread-reply-indicator">↳</span>
      <span class="review-thread-author">{reply.author}</span>
      <span class="review-thread-time">{relativeTime(reply.createdAt)}</span>
      <div class="review-thread-reply-text">{reply.body}</div>
    </div>
  {/each}

  <!-- Action bar: resolve button, reply appears on click -->
  <div class="review-thread-actions">
    <button class="review-thread-resolve" onclick={() => handleResolve(thread.annotation.id, !thread.annotation.resolved)}>
      {thread.annotation.resolved ? "✓ Resolved" : "✓ Resolve"}
    </button>
  </div>
</div>
```

- [ ] **Step 2: Remove "Open file →" from individual threads**

The "Open in editor →" button is already in the file header. Remove it from each thread/card's action bar.

- [ ] **Step 3: Hide reply input until activated**

Instead of always showing a reply `<input>`, show it only when the thread is clicked or when `r` key is pressed. Add local state:

```svelte
<script>
  let replyingTo: string | null = $state(null);
</script>

<!-- In thread actions: -->
{#if replyingTo === thread.annotation.id}
  <input class="review-thread-reply-input" ... />
{:else}
  <button class="review-thread-reply-btn" onclick={() => { replyingTo = thread.annotation.id; }}>
    Reply
  </button>
{/if}
```

Update the `r` key handler to set `replyingTo` for the active thread.

- [ ] **Step 4: Build and verify**

Run: `npm run build`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: render threads inline — remove per-card snippet, hide reply until activated"
```

---

### Task 3: Add Status Dots and Hover-Sync to Snippet

**Files:**
- Modify: `src/components/ReviewCodeSnippet.svelte`
- Modify: `src/components/ReviewPage.svelte` (pass annotatedLines prop)

- [ ] **Step 1: Add annotatedLines prop to ReviewCodeSnippet**

```typescript
let {
  filePath, snippet, highlightLine, highlightEndLine, diffHunk, kindColor,
  annotatedLines, // New: array of { line: number, resolved: boolean }
}: {
  // ...existing types...
  annotatedLines?: { line: number; resolved: boolean }[];
} = $props();
```

- [ ] **Step 2: Render status dots on annotated lines**

In the line rendering loop, after the line content, add a dot indicator:

```svelte
{#if annotatedLines?.some(a => a.line === line.lineNum)}
  {@const ann = annotatedLines.find(a => a.line === line.lineNum)}
  <span
    class="snippet-status-dot"
    class:snippet-dot-resolved={ann?.resolved}
    title="L{line.lineNum}: {ann?.resolved ? 'resolved' : 'unresolved'}"
  ></span>
{/if}
```

CSS:
```css
.snippet-status-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  margin-left: 6px;
  vertical-align: middle;
  opacity: 0.6;
}
.snippet-dot-resolved {
  background: var(--color-success);
}
```

- [ ] **Step 3: Add hover-sync CSS classes**

Add a `data-snippet-line` attribute to each snippet line and a `data-thread-line` attribute to each thread L-tag. Use CSS `:hover` + sibling selectors or JavaScript to highlight corresponding elements:

Since CSS can't select siblings across different containers, use a lightweight JS approach:
- On hover of an L-tag, add a `data-highlight-line` attribute to the parent context block
- CSS targets `.review-context-block[data-highlight-line="3"] .snippet-line[data-line="3"]`

```svelte
<!-- On the L-tag: -->
<span
  class="review-thread-line-tag"
  data-thread-line={thread.annotation.anchor.range.startLine}
  onmouseenter={(e) => {
    const block = e.currentTarget.closest('.review-context-block');
    block?.setAttribute('data-highlight-line', String(thread.annotation.anchor.range.startLine));
  }}
  onmouseleave={(e) => {
    const block = e.currentTarget.closest('.review-context-block');
    block?.removeAttribute('data-highlight-line');
  }}
>L{thread.annotation.anchor.range.startLine}</span>
```

Add to ReviewCodeSnippet: `data-line={line.lineNum}` attribute on each line div.

CSS in ReviewPage:
```css
.review-context-block[data-highlight-line] .snippet-line {
  transition: background 150ms;
}
/* Dynamic attribute matching via JS — highlight the specific line */
```

Since CSS attribute selectors can't match dynamic values without a class per line, the simplest approach is to add/remove a `.snippet-line-highlighted` class via the JS event handlers above.

- [ ] **Step 4: Build and verify**

Run: `npm run build`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: add status dots and hover-sync between L-tags and snippet lines"
```

---

### Task 4: Style the Review Page with Surface Elevation

**Files:**
- Modify: `src/components/ReviewPage.svelte` (update styles)

- [ ] **Step 1: Update context block and thread styles**

Replace the heavy-bordered card styles with surface elevation:

```css
.review-context-block {
  background: var(--surface-panel);
  border-radius: 8px;
  overflow: hidden;
  margin: 10px 0;
}
.review-thread {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-subtle);
  cursor: pointer;
}
.review-thread:last-child {
  border-bottom: none;
}
.review-thread-active {
  background: rgba(255, 255, 255, 0.015);
}
.review-thread-resolved {
  opacity: 0.5;
}
.review-thread-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}
.review-thread-line-tag {
  color: var(--text-ghost);
  font-size: 9px;
  font-family: var(--font-mono);
  background: rgba(255, 255, 255, 0.04);
  padding: 1px 4px;
  border-radius: 3px;
  cursor: pointer;
}
.review-thread-line-tag:hover {
  color: var(--text-muted);
  background: rgba(255, 255, 255, 0.08);
}
.review-thread-author {
  font-weight: 600;
  color: var(--text-primary);
  font-size: 12px;
}
.review-thread-time {
  margin-left: auto;
  font-size: 10px;
  color: var(--text-ghost);
}
.review-thread-body {
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.5;
  padding-left: 30px;
}
.review-thread-reply {
  padding: 8px 10px 8px 30px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 4px;
  border-left: 2px solid var(--border-subtle);
  margin: 4px 0 4px 30px;
}
.review-thread-actions {
  padding-left: 30px;
  margin-top: 6px;
}
.review-thread-resolve {
  background: transparent;
  border: 1px solid var(--border-default);
  border-radius: 4px;
  padding: 3px 10px;
  color: var(--text-muted);
  font-size: 10px;
  cursor: pointer;
  font-family: inherit;
}
```

- [ ] **Step 2: Update file header to remove redundant "Open file" from cards**

Verify the file header has "Open in editor →" and no individual threads have it.

- [ ] **Step 3: Build and verify**

Run: `npm run build`

- [ ] **Step 4: Commit**

```bash
git commit -m "style: review page surface elevation — no heavy borders, tighter spacing"
```
