# PR Description Annotations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow reviewers to annotate paragraphs in a PR description using the existing annotation system, with annotations bundled into the GitHub review summary on submit.

**Architecture:** Treat the PR body as a virtual file (`{worktreePath}/__redpen__/pr-body.md`), split it into blocks with line numbers, and reuse the existing annotation store/sidecar/bubble infrastructure. A new `PrBodyAnnotatable.svelte` component replaces the raw markdown rendering in `PullRequestView.svelte`, adding hover-to-annotate affordances and collapsed/expanded bubbles. On review submission, PR body annotations are formatted into the review summary message.

**Tech Stack:** Svelte 5 (runes), TypeScript, existing markdown renderer (Marked), existing annotation store

---

### Task 1: Create `splitMarkdownBlocks` Utility

**Files:**
- Create: `src/lib/markdown/blocks.ts`

- [ ] **Step 1: Create the block-splitting utility**

The markdown renderer already adds `data-source-line` attributes via `findSourceLine()` in `src/lib/markdown/render.ts`. We need a complementary function that splits markdown source into annotatable blocks, each with a line number matching the source line.

```typescript
// src/lib/markdown/blocks.ts

export interface MarkdownBlock {
  /** Source line number in the original markdown (1-based) */
  lineNumber: number;
  /** Raw markdown text of the block */
  content: string;
  /** Rendered HTML for this block */
  html: string;
  /** Block type for formatting in submit summary */
  type: "heading" | "paragraph" | "checklist" | "listItem" | "codeBlock" | "blockquote" | "other";
}

/**
 * Split a markdown body into annotatable blocks.
 * Each block gets a line number matching its position in the source.
 * The line numbers correspond to `data-source-line` attributes added
 * by the markdown renderer.
 */
export function splitMarkdownBlocks(body: string): MarkdownBlock[] {
  if (!body.trim()) return [];

  const lines = body.split("\n");
  const blocks: MarkdownBlock[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // Skip blank lines
    if (trimmed === "" || trimmed === "---" || trimmed === "***") {
      i++;
      continue;
    }

    const lineNumber = i + 1; // 1-based

    // Heading
    if (/^#{1,6}\s/.test(trimmed)) {
      blocks.push({
        lineNumber,
        content: trimmed,
        html: "", // filled later
        type: "heading",
      });
      i++;
      continue;
    }

    // Checklist item
    if (/^[-*]\s*\[[ xX]\]\s/.test(trimmed)) {
      blocks.push({
        lineNumber,
        content: trimmed,
        html: "",
        type: "checklist",
      });
      i++;
      continue;
    }

    // List item (non-checklist)
    if (/^[-*+]\s/.test(trimmed) || /^\d+\.\s/.test(trimmed)) {
      blocks.push({
        lineNumber,
        content: trimmed,
        html: "",
        type: "listItem",
      });
      i++;
      continue;
    }

    // Fenced code block
    if (trimmed.startsWith("```")) {
      const startLine = i;
      i++;
      while (i < lines.length && !lines[i].trim().startsWith("```")) {
        i++;
      }
      i++; // skip closing ```
      const content = lines.slice(startLine, i).join("\n");
      blocks.push({
        lineNumber,
        content,
        html: "",
        type: "codeBlock",
      });
      continue;
    }

    // Blockquote
    if (trimmed.startsWith(">")) {
      const startLine = i;
      while (i < lines.length && lines[i].trim().startsWith(">")) {
        i++;
      }
      const content = lines.slice(startLine, i).join("\n");
      blocks.push({
        lineNumber,
        content,
        html: "",
        type: "blockquote",
      });
      continue;
    }

    // Paragraph: consume consecutive non-blank lines
    const startLine = i;
    while (i < lines.length && lines[i].trim() !== "" && !/^#{1,6}\s/.test(lines[i].trim()) && !lines[i].trim().startsWith("```") && !/^[-*+]\s/.test(lines[i].trim()) && !/^\d+\.\s/.test(lines[i].trim()) && !lines[i].trim().startsWith(">")) {
      i++;
    }
    const content = lines.slice(startLine, i).join("\n");
    blocks.push({
      lineNumber,
      content,
      html: "",
      type: "paragraph",
    });
  }

  return blocks;
}

/**
 * Get a short preview of a block for the submit summary.
 * Strips markdown syntax and truncates.
 */
export function blockPreview(block: MarkdownBlock, maxLength = 60): string {
  let text = block.content
    .replace(/^#+\s*/, "")           // strip heading markers
    .replace(/^[-*]\s*\[[ xX]\]\s*/, "") // strip checklist markers
    .replace(/^[-*+]\s*/, "")        // strip list markers
    .replace(/^\d+\.\s*/, "")        // strip ordered list markers
    .replace(/^>\s*/gm, "")          // strip blockquote markers
    .replace(/\*\*/g, "")            // strip bold
    .replace(/`([^`]+)`/g, "$1")     // strip inline code
    .trim();
  if (text.length > maxLength) {
    text = text.slice(0, maxLength).trimEnd() + "…";
  }
  return text;
}
```

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/lib/markdown/blocks.ts
git commit -m "feat: add splitMarkdownBlocks utility for PR body annotation targeting"
```

---

### Task 2: Create `PrBodyAnnotatable.svelte` Component

**Files:**
- Create: `src/components/PrBodyAnnotatable.svelte`

- [ ] **Step 1: Create the annotatable PR body component**

This component replaces raw `{@html}` rendering with block-level annotation support.

```svelte
<!-- src/components/PrBodyAnnotatable.svelte -->
<script lang="ts">
  import { splitMarkdownBlocks, type MarkdownBlock } from "$lib/markdown/blocks";
  import { renderMarkdown } from "$lib/markdown/render";
  import "$lib/markdown/markdown.css";
  import AnnotationBubble from "./AnnotationBubble.svelte";
  import AnnotationPopover from "./AnnotationPopover.svelte";
  import {
    loadAnnotations,
    getAnnotationsState,
    addAnnotation,
    sortedAnnotations,
    selectAnnotation,
  } from "$lib/stores/annotations.svelte";
  import { onMount } from "svelte";

  let {
    body,
    worktreePath,
  }: {
    body: string;
    worktreePath: string;
  } = $props();

  const virtualFilePath = $derived(`${worktreePath}/__redpen__/pr-body.md`);

  // Split body into blocks
  const blocks = $derived(splitMarkdownBlocks(body));

  // Render full body HTML, then extract per-block HTML via data-source-line
  const fullHtml = $derived(renderMarkdown(body || "_No description provided._"));

  // Annotation state
  const annotationsState = getAnnotationsState();
  const annotations = $derived(sortedAnnotations());

  // Group annotations by line number (block lineNumber)
  const annotationsByLine = $derived.by(() => {
    const map = new Map<number, typeof annotations>();
    for (const ann of annotations) {
      if (ann.replyTo) continue; // skip replies, they're grouped under parent
      const line = ann.anchor.range.startLine;
      const group = map.get(line) ?? [];
      group.push(ann);
      // Also add replies
      for (const reply of annotations.filter(a => a.replyTo === ann.id)) {
        group.push(reply);
      }
      map.set(line, group);
    }
    return map;
  });

  // Count annotations for header badge
  const annotationCount = $derived(
    annotations.filter(a => !a.replyTo).length
  );

  // Hover state
  let hoveredBlock: number | null = $state(null);

  // Popover state
  let showPopover = $state(false);
  let popoverBlock: MarkdownBlock | null = $state(null);
  let popoverPosition = $state({ x: 0, y: 0 });

  // Expanded bubble state (single-focus, like code bubbles)
  let focusedLine: number | null = $state(null);

  function handleBlockHover(lineNumber: number) {
    hoveredBlock = lineNumber;
  }

  function handleBlockLeave() {
    hoveredBlock = null;
  }

  function handleAddClick(block: MarkdownBlock, event: MouseEvent) {
    event.stopPropagation();
    popoverBlock = block;
    popoverPosition = { x: event.clientX - 160, y: event.clientY + 10 };
    showPopover = true;
  }

  async function handleAnnotationSubmit(bodyText: string, labels: string[]) {
    if (!popoverBlock) return;
    await addAnnotation(
      virtualFilePath,
      bodyText,
      labels,
      popoverBlock.lineNumber,
      0,
      popoverBlock.lineNumber,
      popoverBlock.content.length,
    );
    showPopover = false;
    popoverBlock = null;
  }

  function handleBubbleToggle(lineNumber: number) {
    focusedLine = focusedLine === lineNumber ? null : lineNumber;
  }

  function handleBubbleSelect(id: string) {
    selectAnnotation(id);
  }

  // Load annotations for virtual file on mount
  onMount(async () => {
    await loadAnnotations(virtualFilePath);
  });

  // Reload when virtual path changes
  $effect(() => {
    void loadAnnotations(virtualFilePath);
  });

  // Annotation numbering: sequential across annotated blocks
  const annotationNumbers = $derived.by(() => {
    const nums = new Map<number, number>();
    let n = 1;
    for (const block of blocks) {
      if (annotationsByLine.has(block.lineNumber)) {
        nums.set(block.lineNumber, n++);
      }
    }
    return nums;
  });
</script>

<div class="pr-body-annotatable markdown-body">
  {#each blocks as block (block.lineNumber)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="pr-body-block"
      class:pr-body-block-hovered={hoveredBlock === block.lineNumber}
      class:pr-body-block-annotated={annotationsByLine.has(block.lineNumber)}
      onmouseenter={() => handleBlockHover(block.lineNumber)}
      onmouseleave={handleBlockLeave}
    >
      <!-- Gutter area -->
      <div class="pr-body-gutter">
        {#if annotationsByLine.has(block.lineNumber)}
          <!-- Numbered dot for annotated blocks — always visible -->
          <button
            class="pr-body-dot"
            class:pr-body-dot-focused={focusedLine === block.lineNumber}
            onclick={() => handleBubbleToggle(block.lineNumber)}
          >
            {annotationNumbers.get(block.lineNumber)}
          </button>
        {:else if hoveredBlock === block.lineNumber}
          <!-- + button on hover for unannotated blocks -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <button
            class="pr-body-add-btn"
            onclick={(e) => handleAddClick(block, e)}
            title="Add annotation"
          >+</button>
        {/if}
      </div>

      <!-- Block content (rendered markdown) -->
      <div class="pr-body-content">
        {@html renderMarkdown(block.content)}
      </div>
    </div>

    <!-- Annotation bubble (collapsed or expanded) -->
    {#if annotationsByLine.has(block.lineNumber)}
      <div class="pr-body-bubble-wrapper">
        <AnnotationBubble
          annotations={annotationsByLine.get(block.lineNumber) ?? []}
          expanded={focusedLine === block.lineNumber}
          focusPosition={focusedLine === block.lineNumber
            ? { current: annotationNumbers.get(block.lineNumber) ?? 0, total: annotationNumbers.size }
            : null}
          onToggle={() => handleBubbleToggle(block.lineNumber)}
          onSelect={handleBubbleSelect}
          onDelete={() => {}}
          onChoiceToggle={() => {}}
        />
      </div>
    {/if}
  {/each}
</div>

{#if showPopover}
  <AnnotationPopover
    position={popoverPosition}
    onSubmit={handleAnnotationSubmit}
    onCancel={() => { showPopover = false; popoverBlock = null; }}
  />
{/if}

<style>
  .pr-body-annotatable {
    padding: 16px;
  }
  .pr-body-block {
    position: relative;
    padding-left: 28px;
    border-radius: 4px;
    transition: background 150ms;
  }
  .pr-body-block-hovered:not(.pr-body-block-annotated) {
    background: rgba(217, 177, 95, 0.03);
  }
  .pr-body-gutter {
    position: absolute;
    left: 0;
    top: 4px;
    width: 24px;
    display: flex;
    align-items: flex-start;
    justify-content: center;
  }
  .pr-body-dot {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(217, 177, 95, 0.2);
    color: var(--accent);
    font-size: 9px;
    font-weight: 700;
    border: none;
    cursor: pointer;
    font-family: inherit;
    transition: background 150ms;
  }
  .pr-body-dot:hover,
  .pr-body-dot-focused {
    background: var(--accent);
    color: var(--surface-base);
  }
  .pr-body-add-btn {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(217, 177, 95, 0.15);
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    font-family: inherit;
  }
  .pr-body-add-btn:hover {
    background: rgba(217, 177, 95, 0.3);
  }
  .pr-body-content {
    min-height: 1.5em;
  }
  .pr-body-content :global(p) {
    margin: 0 0 8px;
  }
  .pr-body-content :global(h1),
  .pr-body-content :global(h2),
  .pr-body-content :global(h3),
  .pr-body-content :global(h4) {
    margin: 0 0 8px;
  }
  .pr-body-content :global(ul),
  .pr-body-content :global(ol) {
    margin: 0 0 8px;
    padding-left: 20px;
  }
  .pr-body-content :global(pre) {
    margin: 0 0 8px;
  }
  .pr-body-bubble-wrapper {
    padding-left: 28px;
    margin-bottom: 8px;
  }
</style>
```

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/components/PrBodyAnnotatable.svelte
git commit -m "feat: create PrBodyAnnotatable component with hover-to-annotate and bubbles"
```

---

### Task 3: Wire Into PullRequestView

**Files:**
- Modify: `src/components/PullRequestView.svelte`

- [ ] **Step 1: Replace raw HTML rendering with PrBodyAnnotatable**

Replace the body rendering in `PullRequestView.svelte`:

```svelte
<script lang="ts">
  import type { GitHubPrSession } from "$lib/types";
  import PrBodyAnnotatable from "./PrBodyAnnotatable.svelte";

  let { session }: { session: GitHubPrSession } = $props();
</script>

<div class="pr-view">
  <div class="pr-view-card">
    <div class="pr-view-card-header">
      <div class="pr-view-card-title">Pull request</div>
      <div class="pr-view-card-meta">{session.repo} #{session.number}</div>
    </div>
    <PrBodyAnnotatable
      body={session.body ?? ""}
      worktreePath={session.worktreePath}
    />
  </div>
</div>
```

Keep the existing `<style>` block unchanged (`.pr-view`, `.pr-view-card`, etc.).

Remove the `renderMarkdown` import and `bodyHtml` derived since they're no longer needed at this level.

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/components/PullRequestView.svelte
git commit -m "feat: wire PrBodyAnnotatable into PullRequestView — replace raw HTML rendering"
```

---

### Task 4: Bundle PR Body Annotations Into Review Summary

**Files:**
- Modify: `src/components/review-header/ReviewSubmitControl.svelte`
- Create: `src/lib/review/prBodySummary.ts`

- [ ] **Step 1: Create the summary formatter**

```typescript
// src/lib/review/prBodySummary.ts

import { getAnnotations } from "$lib/tauri";
import { splitMarkdownBlocks, blockPreview } from "$lib/markdown/blocks";
import type { Annotation } from "$lib/types";

/**
 * Collect PR body annotations and format them as a markdown summary
 * for inclusion in the GitHub review message.
 */
export async function formatPrBodyAnnotations(
  worktreePath: string,
  prBody: string,
): Promise<string> {
  const virtualPath = `${worktreePath}/__redpen__/pr-body.md`;

  let sidecar;
  try {
    sidecar = await getAnnotations(virtualPath);
  } catch {
    return "";
  }

  if (!sidecar?.annotations.length) return "";

  const blocks = splitMarkdownBlocks(prBody);
  const blockMap = new Map(blocks.map(b => [b.lineNumber, b]));

  // Get root annotations (not replies), sorted by line
  const roots = sidecar.annotations
    .filter((a: Annotation) => !a.replyTo)
    .sort((a: Annotation, b: Annotation) =>
      a.anchor.range.startLine - b.anchor.range.startLine
    );

  if (roots.length === 0) return "";

  // Build reply map
  const replyMap = new Map<string, Annotation[]>();
  for (const ann of sidecar.annotations) {
    if (ann.replyTo) {
      const group = replyMap.get(ann.replyTo) ?? [];
      group.push(ann);
      replyMap.set(ann.replyTo, group);
    }
  }

  const lines: string[] = ["**Comments on PR description:**", ""];

  for (const root of roots) {
    const line = root.anchor.range.startLine;
    const block = blockMap.get(line);
    const preview = block ? blockPreview(block) : `line ${line}`;
    const label = block?.type === "checklist"
      ? `checklist item "${preview}"`
      : `¶${line} (${preview})`;

    lines.push(`> **Re: ${label}:**`);
    lines.push(`> ${root.body}`);

    const replies = replyMap.get(root.id) ?? [];
    for (const reply of replies) {
      lines.push(`> > ↳ ${reply.author}: ${reply.body}`);
    }

    lines.push("");
  }

  return lines.join("\n");
}
```

- [ ] **Step 2: Integrate into ReviewSubmitControl**

In `src/components/review-header/ReviewSubmitControl.svelte`, modify `openSubmitModal` to pre-populate the message with PR body annotations:

Import at the top:
```typescript
import { formatPrBodyAnnotations } from "$lib/review/prBodySummary";
import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
```

Modify `openSubmitModal`:
```typescript
async function openSubmitModal(action: "comment" | "approve" | "requestChanges") {
  showSubmitMenu = false;
  submitModalAction = action;
  submitModalStatus = "editing";
  submitModalError = null;
  submitModalResult = null;

  // Pre-populate with PR body annotations
  const githubReview = getGitHubReviewState();
  const session = githubReview.activeSession;
  if (session) {
    submitModalMessage = await formatPrBodyAnnotations(
      session.worktreePath,
      session.body ?? "",
    );
  } else {
    submitModalMessage = "";
  }

  requestAnimationFrame(() => submitTextareaRef?.focus());
}
```

- [ ] **Step 3: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/lib/review/prBodySummary.ts src/components/review-header/ReviewSubmitControl.svelte
git commit -m "feat: bundle PR body annotations into review summary on submit"
```

---

### Task 5: Handle Annotation Deletion and Edge Cases

**Files:**
- Modify: `src/components/PrBodyAnnotatable.svelte`

- [ ] **Step 1: Wire up delete handler**

In `PrBodyAnnotatable.svelte`, add the delete handler. Import `removeAnnotation`:

```typescript
import {
  loadAnnotations,
  getAnnotationsState,
  addAnnotation,
  sortedAnnotations,
  selectAnnotation,
  removeAnnotation,
} from "$lib/stores/annotations.svelte";
```

Add the handler:
```typescript
async function handleDelete(id: string) {
  await removeAnnotation(virtualFilePath, id);
}
```

Update the `AnnotationBubble` usage:
```svelte
<AnnotationBubble
  annotations={annotationsByLine.get(block.lineNumber) ?? []}
  expanded={focusedLine === block.lineNumber}
  focusPosition={...}
  onToggle={() => handleBubbleToggle(block.lineNumber)}
  onSelect={handleBubbleSelect}
  onDelete={handleDelete}
  onChoiceToggle={() => {}}
/>
```

- [ ] **Step 2: Handle empty body gracefully**

Ensure the component renders a clean message when the PR body is empty:

```svelte
{#if blocks.length === 0}
  <div class="pr-body-empty">
    <p>No description provided.</p>
  </div>
{:else}
  {#each blocks as block (block.lineNumber)}
    ...
  {/each}
{/if}
```

Add CSS:
```css
.pr-body-empty {
  padding: 16px 28px;
  color: var(--text-muted);
  font-style: italic;
}
```

- [ ] **Step 3: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/components/PrBodyAnnotatable.svelte
git commit -m "feat: wire delete handler and handle empty PR body in annotatable view"
```
