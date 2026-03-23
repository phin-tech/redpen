<script lang="ts">
  import Editor from "./Editor.svelte";
  import MarkdownPreview from "./MarkdownPreview.svelte";
  import { getEditor, getFileExtension, isMarkdownFile, getShowPreview, togglePreview } from "$lib/stores/editor.svelte";
  import { sortedAnnotations } from "$lib/stores/annotations.svelte";

  let {
    onSelectionChange,
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    ref?: { scrollToLine: (line: number) => void } | undefined;
  } = $props();

  const editor = getEditor();

  let editorRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);
  let previewRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);

  // Forward the active ref to parent
  $effect(() => {
    ref = getShowPreview() && isMarkdownFile() ? previewRef : editorRef;
  });
</script>

<div class="editor-pane">
  {#if isMarkdownFile()}
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

  <div class="pane-content">
    {#if isMarkdownFile() && getShowPreview()}
      <MarkdownPreview
        content={editor.content}
        annotations={sortedAnnotations()}
        {onSelectionChange}
        bind:ref={previewRef}
      />
    {:else}
      <Editor
        {onSelectionChange}
        bind:ref={editorRef}
      />
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
    gap: 2px;
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

  .toggle-btn:hover {
    color: var(--text-secondary);
    background: var(--surface-raised);
  }

  .toggle-btn.active {
    color: var(--accent);
    background: var(--accent-subtle);
  }

  .pane-content {
    flex: 1;
    overflow: hidden;
  }
</style>
