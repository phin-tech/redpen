<script lang="ts">
  import Editor from "./Editor.svelte";
  import MarkdownPreview from "./MarkdownPreview.svelte";
  import DiffEditor from "./DiffEditor.svelte";
  import DiffModeToggle from "./DiffModeToggle.svelte";
  import DiffRefPicker from "./DiffRefPicker.svelte";
  import ReviewPage from "./ReviewPage.svelte";
  import { getEditor, getFileExtension, isMarkdownFile, getShowPreview, togglePreview } from "$lib/stores/editor.svelte";
  import { getDiffState, enterDiff, exitDiff, setDiffMode } from "$lib/stores/diff.svelte";
  import { isReviewPageOpen } from "$lib/stores/reviewPage.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { sortedAnnotations, getBubblesEnabled, toggleBubbles } from "$lib/stores/annotations.svelte";
  import BubbleKindFilter from "./BubbleKindFilter.svelte";
  import { highlightsModeExtensions, buildUnifiedDocument, buildSplitDecorations, scrollSync } from "$lib/codemirror/diff";
  import { onDestroy } from "svelte";

  let {
    onSelectionChange,
    onJumpToFile,
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    onJumpToFile?: (filePath: string, line: number) => void;
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
  // Only track diff.enabled and diff.mode — not the editor refs (avoids loops on destroy)
  $effect(() => {
    // Clean up previous sync
    cleanupScrollSync?.();
    cleanupScrollSync = null;

    if (diff.enabled && diff.mode === "split") {
      // Delay to let DiffEditor instances mount and populate bind:this
      const timer = setTimeout(() => {
        const leftView = leftDiffEditor?.getView();
        const rightView = rightDiffEditor?.getView();
        if (leftView && rightView) {
          cleanupScrollSync = scrollSync(leftView, rightView);
        }
      }, 50);
      return () => clearTimeout(timer);
    }
  });

  onDestroy(() => {
    cleanupScrollSync?.();
  });

  const directory = $derived(workspace.rootFolders[0] ?? "");

  function handleEnterDiff(mode: import("$lib/types").DiffMode) {
    if (editor.currentFilePath && directory) {
      setDiffMode(mode);
      enterDiff(directory, editor.currentFilePath);
    }
  }

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
  <!-- Toolbar: always visible when a file is open -->
  {#if editor.currentFilePath}
    <div class="toggle-bar">
      <DiffModeToggle onEnterDiff={handleEnterDiff} />
      {#if diff.enabled}
        <DiffRefPicker {directory} filePath={editor.currentFilePath ?? ""} />
      {/if}
      <div class="separator"></div>
      <button
        class="toggle-btn"
        class:active={getBubblesEnabled()}
        onclick={() => toggleBubbles()}
        title="Toggle inline annotations"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
        Inline
      </button>
      {#if getBubblesEnabled()}
        <BubbleKindFilter />
      {/if}
      {#if isMarkdownFile() && !diff.enabled}
        <div class="separator"></div>
        <button
          class="toggle-btn"
          class:active={!getShowPreview()}
          onclick={() => { if (getShowPreview()) togglePreview(); }}
        >
          Source
        </button>
        <button
          class="toggle-btn"
          class:active={getShowPreview()}
          onclick={() => { if (!getShowPreview()) togglePreview(); }}
        >
          Preview
        </button>
      {/if}
    </div>
  {/if}

  <!-- Content area -->
  <div class="pane-content">
    {#if isReviewPageOpen()}
      <ReviewPage onJumpToFile={(path, line) => onJumpToFile?.(path, line)} />
    {:else if diff.enabled && diff.loading}
      <div class="diff-status">Computing diff...</div>
    {:else if diff.enabled && diff.error}
      <div class="diff-status">
        <p>Cannot compute diff: {diff.error}</p>
        <button class="exit-btn" onclick={() => exitDiff()}>Exit diff mode</button>
      </div>
    {:else if diff.enabled && diff.diffResult}
      {#if diff.diffResult.hunks.length === 0}
        <div class="diff-status">No changes between {diff.baseRef} and {diff.targetRef}</div>
      {:else if diff.mode === "split"}
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
  .separator {
    width: 1px;
    height: 16px;
    background: var(--border-default);
    margin: 0 4px;
  }
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
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 12px;
  }
</style>
