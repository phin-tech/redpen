<script lang="ts">
  import { tick } from "svelte";
  import { renderMarkdown } from "$lib/markdown/render";
  import type { Annotation } from "$lib/types";
  import "$lib/markdown/markdown.css";
  import mermaid from "mermaid";

  let {
    content,
    annotations = [],
    onSelectionChange,
    ref = $bindable(undefined),
  }: {
    content: string;
    annotations?: Annotation[];
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    ref?: { scrollToLine: (line: number) => void } | undefined;
  } = $props();

  let container: HTMLDivElement;
  let mermaidInitialized = false;

  function scrollToLine(line: number) {
    if (!container) return;
    const el = container.querySelector(`[data-source-line="${line}"]`);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }

  $effect(() => {
    ref = { scrollToLine };
  });

  // Render markdown and run mermaid when content changes
  $effect(() => {
    if (!container || content === undefined) return;
    const html = renderMarkdown(content);
    container.innerHTML = html;

    // Initialize mermaid once
    if (!mermaidInitialized) {
      mermaid.initialize({
        startOnLoad: false,
        theme: "dark",
        darkMode: true,
      });
      mermaidInitialized = true;
    }

    // Run mermaid on any diagram blocks
    tick().then(() => {
      requestAnimationFrame(() => {
        const mermaidBlocks = container?.querySelectorAll("pre.mermaid");
        if (mermaidBlocks?.length) {
          mermaid.run({ nodes: mermaidBlocks as NodeListOf<HTMLElement> });
        }
      });
    });
  });

  // Apply annotation highlights when annotations change
  $effect(() => {
    if (!container) return;
    // Need to reference content to ensure this runs after render
    void content;

    tick().then(() => {
      // Clear existing highlights
      container.querySelectorAll(".rp-annotation-highlight").forEach((el) => {
        el.classList.remove("rp-annotation-highlight");
      });

      // Apply highlights for each annotation's line range
      for (const ann of annotations) {
        const { startLine, endLine } = ann.anchor.range;
        const elements = container.querySelectorAll("[data-source-line]");
        for (const el of elements) {
          const line = parseInt(el.getAttribute("data-source-line")!);
          if (line >= startLine && line <= endLine) {
            el.classList.add("rp-annotation-highlight");
          }
        }
      }
    });
  });

  function handleMouseUp() {
    if (!onSelectionChange || !container) return;

    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || !sel.rangeCount) return;

    const range = sel.getRangeAt(0);

    // Find source lines from the selection endpoints
    const startLine = findSourceLineFromNode(range.startContainer);
    const endLine = findSourceLineFromNode(range.endContainer);

    if (startLine !== null && endLine !== null) {
      const fromLine = Math.min(startLine, endLine);
      const toLine = Math.max(startLine, endLine);
      // In rendered view, use full-line annotations (column 0 to end)
      onSelectionChange(fromLine, 0, toLine, 999);
    }
  }

  function findSourceLineFromNode(node: Node): number | null {
    let current: Node | null = node;
    while (current && current !== container) {
      if (current instanceof HTMLElement) {
        const line = current.getAttribute("data-source-line");
        if (line) return parseInt(line);
      }
      current = current.parentNode;
    }
    return null;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="markdown-body"
  bind:this={container}
  onmouseup={handleMouseUp}
></div>
