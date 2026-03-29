<script lang="ts">
  import type { FileSnippet } from "$lib/tauri";
  import type { DiffHunk } from "$lib/types";
  import { readFileLines } from "$lib/tauri";
  import { getLanguageForExtension } from "$lib/codemirror/languages";
  import { highlightTree } from "@lezer/highlight";
  import { classHighlighter } from "@lezer/highlight";

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

  // Syntax highlight using Lezer — returns array of HTML strings per line
  const fileExt = $derived(filePath.split(".").pop() ?? "");

  const highlightedLines = $derived.by((): string[] => {
    if (displayLines.length === 0) return [];

    const lang = getLanguageForExtension(fileExt);
    if (!lang) {
      return displayLines.map(l => escapeHtml(l.content));
    }

    // Separate non-deleted lines for parsing (deleted lines break the parser)
    const keepLines = displayLines.filter(l => l.changeKind !== "delete");
    const keepText = keepLines.map(l => l.content).join("\n");

    const tree = lang.language.parser.parse(keepText);
    const spans: { from: number; to: number; classes: string }[] = [];
    highlightTree(tree, classHighlighter, (from, to, classes) => {
      spans.push({ from, to, classes });
    });

    // Build highlighted HTML for kept lines
    const keepHtmls: string[] = [];
    let pos = 0;
    for (const line of keepLines) {
      const lineStart = pos;
      const lineEnd = pos + line.content.length;
      let html = "";
      let cur = lineStart;

      for (const span of spans) {
        if (span.to <= lineStart) continue;
        if (span.from >= lineEnd) break;
        const from = Math.max(span.from, lineStart);
        const to = Math.min(span.to, lineEnd);
        if (from > cur) {
          html += escapeHtml(keepText.slice(cur, from));
        }
        html += `<span class="${span.classes}">${escapeHtml(keepText.slice(from, to))}</span>`;
        cur = to;
      }
      if (cur < lineEnd) {
        html += escapeHtml(keepText.slice(cur, lineEnd));
      }

      keepHtmls.push(html);
      pos = lineEnd + 1;
    }

    // Map back: kept lines get highlighted HTML, deleted lines get plain escaped
    const result: string[] = [];
    let keepIdx = 0;
    for (const line of displayLines) {
      if (line.changeKind === "delete") {
        result.push(escapeHtml(line.content));
      } else {
        result.push(keepHtmls[keepIdx++]);
      }
    }

    return result;
  });

  function escapeHtml(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
</script>

<div class="review-snippet">
  {#if canExpandAbove && !diffHunk}
    <button class="snippet-expand snippet-expand-above" onclick={expandAbove}>
      ··· expand above ···
    </button>
  {/if}

  {#each displayLines as line, i}
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
      <span class="snippet-content">{@html highlightedLines[i] ?? escapeHtml(line.content)}</span>
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
  /* Lezer highlight classes — match one-dark token colors */
  .snippet-content :global(.tok-keyword) { color: #c678dd; }
  .snippet-content :global(.tok-operator) { color: #56b6c2; }
  .snippet-content :global(.tok-number) { color: #d19a66; }
  .snippet-content :global(.tok-string) { color: #98c379; }
  .snippet-content :global(.tok-string2) { color: #98c379; }
  .snippet-content :global(.tok-comment) { color: #5c6370; font-style: italic; }
  .snippet-content :global(.tok-variableName) { color: #e06c75; }
  .snippet-content :global(.tok-variableName2) { color: #e5c07b; }
  .snippet-content :global(.tok-definition) { color: #61afef; }
  .snippet-content :global(.tok-typeName) { color: #e5c07b; }
  .snippet-content :global(.tok-propertyName) { color: #e06c75; }
  .snippet-content :global(.tok-function) { color: #61afef; }
  .snippet-content :global(.tok-bool) { color: #d19a66; }
  .snippet-content :global(.tok-null) { color: #d19a66; }
  .snippet-content :global(.tok-punctuation) { color: #abb2bf; }
  .snippet-content :global(.tok-meta) { color: #e06c75; }
  .snippet-content :global(.tok-atom) { color: #d19a66; }
  .snippet-content :global(.tok-tagName) { color: #e06c75; }
  .snippet-content :global(.tok-attributeName) { color: #d19a66; }
  .snippet-content :global(.tok-attributeValue) { color: #98c379; }
  .snippet-content :global(.tok-heading) { color: #e06c75; font-weight: bold; }
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
