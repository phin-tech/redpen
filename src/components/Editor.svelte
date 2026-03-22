<script lang="ts">
  import { onDestroy } from "svelte";
  import { EditorView } from "@codemirror/view";
  import { createEditor } from "$lib/codemirror/setup";
  import { setAnnotationsEffect } from "$lib/codemirror/annotations";
  import { getEditor, getFileExtension } from "$lib/stores/editor.svelte";
  import { sortedAnnotations } from "$lib/stores/annotations.svelte";

  // Svelte 5 runes mode: use $bindable ref pattern instead of `export function`
  let {
    onSelectionChange,
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    ref?: { scrollToLine: (line: number) => void } | undefined;
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;

  const editor = getEditor();

  function scrollToLine(line: number) {
    if (!view) return;
    const lineObj = view.state.doc.line(Math.min(line, view.state.doc.lines));
    view.dispatch({
      selection: { anchor: lineObj.from },
      effects: EditorView.scrollIntoView(lineObj.from, { y: "center" }),
    });
  }

  // Expose scrollToLine to parent via ref
  $effect(() => {
    ref = { scrollToLine };
  });

  onDestroy(() => {
    view?.destroy();
  });

  function createView() {
    view?.destroy();
    view = createEditor({
      content: editor.content,
      extension: getFileExtension(),
      parent: container,
      onSelectionChange: onSelectionChange
        ? (from, to, fromLine, fromCol, toLine, toCol) => {
            onSelectionChange!(fromLine, fromCol, toLine, toCol);
          }
        : undefined,
    });
  }

  // Recreate view when file changes
  $effect(() => {
    if (container && editor.content !== undefined && editor.currentFilePath) {
      createView();
    }
  });

  // Update annotations when they change
  $effect(() => {
    const annotations = sortedAnnotations();
    if (view) {
      view.dispatch({
        effects: setAnnotationsEffect.of(annotations),
      });
    }
  });
</script>

<div class="editor-container">
  {#if editor.loading}
    <div class="loading">Loading...</div>
  {:else if !editor.currentFilePath}
    <div class="empty">Select a file to view</div>
  {/if}
  <div class="cm-wrapper" bind:this={container}></div>
</div>

<style>
  .editor-container {
    height: 100%;
    position: relative;
    overflow: hidden;
  }

  .cm-wrapper {
    height: 100%;
  }

  .cm-wrapper :global(.cm-editor) {
    height: 100%;
  }

  .loading, .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }
</style>
