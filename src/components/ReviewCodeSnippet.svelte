<script lang="ts">
  import type { FileSnippet } from "$lib/tauri";
  import type { DiffHunk } from "$lib/types";
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

  function contractAbove() {
    if (expandedAbove <= 0) return;
    const remove = Math.min(5, expandedAbove);
    extraLinesAbove = extraLinesAbove.slice(remove);
    expandedAbove -= remove;
  }

  function contractBelow() {
    if (expandedBelow <= 0) return;
    const remove = Math.min(5, expandedBelow);
    extraLinesBelow = extraLinesBelow.slice(0, extraLinesBelow.length - remove);
    expandedBelow -= remove;
  }

  function reset() {
    expandedAbove = 0;
    expandedBelow = 0;
    extraLinesAbove = [];
    extraLinesBelow = [];
  }

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
    <button class="snippet-expand snippet-expand-above" onclick={expandAbove}>
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
    <button class="snippet-expand snippet-expand-below" onclick={expandBelow}>
      ··· expand below ···
    </button>
  {/if}
  <button class="snippet-contract-above" hidden onclick={contractAbove}></button>
  <button class="snippet-contract-below" hidden onclick={contractBelow}></button>
  <button class="snippet-reset" hidden onclick={reset}></button>
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
