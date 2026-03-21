<script lang="ts">
  import {
    getAnnotationsState,
    filteredAnnotations,
    selectAnnotation,
    removeAnnotation,
    editAnnotation,
  } from "$lib/stores/annotations.svelte";
  import { getEditor } from "$lib/stores/editor.svelte";
  import FilterBar from "./FilterBar.svelte";
  import AnnotationCard from "./AnnotationCard.svelte";

  let {
    onAnnotationClick,
  }: {
    onAnnotationClick: (line: number) => void;
  } = $props();

  const annotationsState = getAnnotationsState();
  const editor = getEditor();

  let editingId: string | null = $state(null);
  let editBody = $state("");

  function handleClick(id: string, line: number) {
    selectAnnotation(id);
    onAnnotationClick(line);
  }

  function handleDoubleClick(id: string, body: string) {
    editingId = id;
    editBody = body;
  }

  async function handleDelete(id: string) {
    if (!editor.currentFilePath) return;
    await removeAnnotation(editor.currentFilePath, id);
  }

  async function saveEdit() {
    if (!editingId || !editor.currentFilePath) return;
    await editAnnotation(editor.currentFilePath, editingId, editBody);
    editingId = null;
  }

  let annotations = $derived(filteredAnnotations());
</script>

<div class="sidebar">
  <FilterBar />

  <div class="annotations-list">
    {#each annotations as annotation (annotation.id)}
      {#if editingId === annotation.id}
        <div class="edit-card">
          <textarea bind:value={editBody} rows="3"></textarea>
          <div class="edit-actions">
            <button onclick={() => (editingId = null)}>Cancel</button>
            <button class="save-btn" onclick={saveEdit}>Save</button>
          </div>
        </div>
      {:else}
        <AnnotationCard
          {annotation}
          isSelected={annotationsState.selectedAnnotationId === annotation.id}
          onClick={() => handleClick(annotation.id, annotation.anchor.range.startLine)}
          onDoubleClick={() => handleDoubleClick(annotation.id, annotation.body)}
          onDelete={() => handleDelete(annotation.id)}
        />
      {/if}
    {/each}

    {#if annotations.length === 0}
      <div class="empty">
        {#if !editor.currentFilePath}
          Select a file to see annotations
        {:else}
          No annotations yet. Select text and press Cmd+Return.
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .sidebar {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
  }

  .annotations-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px;
  }

  .empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .edit-card {
    padding: 8px;
    margin: 4px 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .edit-actions button {
    padding: 2px 10px;
    font-size: 12px;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .save-btn {
    background: var(--accent-blue) !important;
    color: var(--bg-primary) !important;
  }
</style>
