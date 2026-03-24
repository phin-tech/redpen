<script lang="ts">
  import Editor from "./Editor.svelte";
  import MarkdownPreview from "./MarkdownPreview.svelte";
  import DiffEditor from "./DiffEditor.svelte";
  import DiffModeToggle from "./DiffModeToggle.svelte";
  import DiffRefPicker from "./DiffRefPicker.svelte";
  import { getEditor, getFileExtension, isMarkdownFile, getShowPreview, togglePreview } from "$lib/stores/editor.svelte";
  import { getDiffState, exitDiff } from "$lib/stores/diff.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { sortedAnnotations } from "$lib/stores/annotations.svelte";
  import { highlightsModeExtensions, buildUnifiedDocument, buildSplitDecorations, scrollSync } from "$lib/codemirror/diff";
  import { onDestroy } from "svelte";

  let {
    onSelectionChange,
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    ref?: { scrollToLine: (line: number) => void; openSearch: () => void; closeSearch: () => void; navigateMatch: (dir: 1 | -1) => void } | undefined;
  } = $props();

  const editor = getEditor();
  const diff = getDiffState();
  const workspace = getWorkspace();

  type EditorRef = { scrollToLine: (line: number) => void; openSearch: () => void; closeSearch: () => void; navigateMatch: (dir: 1 | -1) => void };
  let editorRef: EditorRef | undefined = $state(undefined);
  let previewRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);
  let leftDiffEditor: { getView: () => any } | undefined = $state(undefined);
  let rightDiffEditor: { getView: () => any } | undefined = $state(undefined);
  let cleanupScrollSync: (() => void) | null = null;

  // Forward the active ref to parent
  $effect(() => {
    ref = getShowPreview() && isMarkdownFile() ? previewRef as EditorRef | undefined : editorRef;
  });

  // Set up scroll sync for split mode
  $effect(() => {
    cleanupScrollSync?.();
    cleanupScrollSync = null;

    if (diff.enabled && diff.mode === "split" && leftDiffEditor && rightDiffEditor) {
      const leftView = leftDiffEditor.getView();
      const rightView = rightDiffEditor.getView();
      if (leftView && rightView) {
        cleanupScrollSync = scrollSync(leftView, rightView);
      }
    }
  });

  onDestroy(() => {
    cleanupScrollSync?.();
  });

  const directory = $derived(workspace.rootFolders[0] ?? "");

  // For unified mode: filter selections to only allow annotatable (non-delete) lines
  function createUnifiedSelectionHandler(lineMap: Map<number, number>) {
    return (fromLine: number, fromCol: number, toLine: number, toCol: number) => {
      // Check if all selected lines are in the lineMap (not delete lines)
      for (let line = fromLine; line <= toLine; line++) {
        if (!lineMap.has(line)) {
          return; // Selection includes a delete line — block annotation
        }
      }
      // Map synthetic line numbers to real new-content line numbers
      const mappedFromLine = lineMap.get(fromLine)!;
      const mappedToLine = lineMap.get(toLine)!;
      onSelectionChange?.(mappedFromLine, fromCol, mappedToLine, toCol);
    };
  }
</script>

<div class="editor-pane">
  <!-- Toolbar: markdown toggle OR diff controls -->
  {#if diff.enabled}
    <div class="toggle-bar">
      <DiffModeToggle />
      <DiffRefPicker {directory} filePath={editor.currentFilePath ?? ""} />
    </div>
  {:else if isMarkdownFile()}
    <div class="toggle-bar">
      <button
        class="toggle-btn"
        class:active={!getShowPreview()}
        onclick={() => { if (getShowPreview()) togglePreview(); }}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <polyline points="4,2 4,14" /><polyline points="12,2 12,14" />
          <polyline points="1,5 4,2 7,5" /><polyline points="9,11 12,14 15,11" />
        </svg>
        Source
      </button>
      <button
        class="toggle-btn"
        class:active={getShowPreview()}
        onclick={() => { if (!getShowPreview()) togglePreview(); }}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M1 8s3-5 7-5 7 5 7 5-3 5-7 5-7-5-7-5z" />
          <circle cx="8" cy="8" r="2" />
        </svg>
        Preview
      </button>
    </div>
  {/if}

  <!-- Content area -->
  <div class="pane-content">
    {#if diff.enabled && diff.loading}
      <div class="diff-status">Computing diff...</div>
    {:else if diff.enabled && diff.error}
      <div class="diff-status">
        <p>Cannot compute diff: {diff.error}</p>
        <button class="exit-btn" onclick={() => exitDiff()}>Exit diff mode</button>
      </div>
    {:else if diff.enabled && diff.diffResult}
      {#if diff.mode === "split"}
        {@const splitDeco = buildSplitDecorations(diff.diffResult)}
        <div class="split-diff">
          <DiffEditor
            bind:this={leftDiffEditor}
            content={diff.diffResult.oldContent}
            fileExtension={getFileExtension()}
            diffExtensions={splitDeco.oldExtensions}
          />
          <div class="split-divider"></div>
          <DiffEditor
            bind:this={rightDiffEditor}
            content={diff.diffResult.newContent}
            fileExtension={getFileExtension()}
            diffExtensions={splitDeco.newExtensions}
          />
        </div>
      {:else if diff.mode === "unified"}
        {@const unified = buildUnifiedDocument(diff.diffResult)}
        <DiffEditor
          content={unified.syntheticDoc}
          fileExtension={getFileExtension()}
          diffExtensions={unified.extensions}
          showLineNumbers={false}
          onSelectionChange={createUnifiedSelectionHandler(unified.lineMap)}
        />
      {:else}
        <!-- Highlights mode: existing Editor with diff decorations overlaid -->
        <Editor {onSelectionChange} bind:ref={editorRef} />
      {/if}
    {:else if isMarkdownFile() && getShowPreview()}
      <MarkdownPreview
        content={editor.content}
        annotations={sortedAnnotations()}
        {onSelectionChange}
        bind:ref={previewRef}
      />
    {:else}
      <Editor {onSelectionChange} bind:ref={editorRef} />
    {/if}
  </div>
</div>

<style>
  .editor-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .toggle-bar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 8px;
    background: var(--gradient-toolbar), var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 3px 10px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
  }
  .toggle-btn:hover { color: var(--text-secondary); background: var(--surface-raised); }
  .toggle-btn.active { color: var(--accent); background: var(--accent-subtle); }
  .pane-content { flex: 1; overflow: hidden; }
  .split-diff {
    display: flex;
    height: 100%;
  }
  .split-diff > :global(*) { flex: 1; min-width: 0; }
  .split-divider {
    width: 1px;
    background: var(--border-default);
    flex: none;
  }
  .diff-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 8px;
  }
  .exit-btn {
    padding: 4px 12px;
    border-radius: 4px;
    background: var(--surface-secondary);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 12px;
  }
</style>
