<script lang="ts">
  import {
    getAnnotationsState,
    sortedAnnotations,
    selectAnnotation,
    removeAnnotation,
    editAnnotation,
  } from "$lib/stores/annotations.svelte";
  import { getEditor } from "$lib/stores/editor.svelte";
  import { Button, Kbd } from "flowbite-svelte";
  import { invoke } from "@tauri-apps/api/core";
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
  let reviewDone = $state(false);

  async function handleDoneReviewing() {
    if (!editor.currentFilePath) return;
    await invoke("signal_review_done", { filePath: editor.currentFilePath });
    reviewDone = true;
    setTimeout(() => (reviewDone = false), 3000);
  }

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

  let annotations = $derived(sortedAnnotations());
</script>

<div class="h-full flex flex-col bg-graphite-950">
  {#if editor.currentFilePath && annotations.length > 0}
    <div class="px-2 py-1.5 border-b border-graphite-700">
      <Button
        size="xs"
        color={reviewDone ? "green" : "primary"}
        class="w-full"
        onclick={handleDoneReviewing}
        disabled={reviewDone}
      >
        {reviewDone ? "Review sent!" : "Done Reviewing"}
      </Button>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto px-1.5 py-1">
    {#each annotations as annotation (annotation.id)}
      {#if editingId === annotation.id}
        <div class="p-2.5 my-1 flex flex-col gap-2 bg-[var(--bg-primary)] rounded-md">
          <textarea
            bind:value={editBody}
            rows="3"
            class="w-full bg-graphite-900 border-graphite-700 text-graphite-50 text-sm rounded-md p-2 focus:border-amber-400 focus:ring-amber-400/20"
          ></textarea>
          <div class="flex justify-end gap-1.5">
            <Button size="xs" color="alternative" onclick={() => (editingId = null)}>Cancel</Button>
            <Button size="xs" color="primary" onclick={saveEdit}>Save</Button>
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
      <div class="flex flex-col items-center justify-center px-5 py-10 gap-2 h-full min-h-[200px]">
        {#if !editor.currentFilePath}
          <svg class="text-graphite-400 opacity-40 mb-1" width="32" height="32" viewBox="0 0 24 24" fill="none">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" stroke-width="1.5"/>
            <path d="M14 2v6h6M12 18v-6M9 15h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <p class="text-sm font-medium text-graphite-200">No file selected</p>
          <p class="text-xs text-graphite-400 text-center">Open a file from the tree to start annotating</p>
        {:else}
          <svg class="text-graphite-400 opacity-40 mb-1" width="32" height="32" viewBox="0 0 24 24" fill="none">
            <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <p class="text-sm font-medium text-graphite-200">No annotations yet</p>
          <p class="text-xs text-graphite-400 text-center">Select text in the editor, then</p>
          <div class="flex items-center gap-1 my-1 text-xs text-graphite-400">
            <Kbd class="!px-1.5 !py-0.5 !text-[11px]">Cmd</Kbd>
            <span>+</span>
            <Kbd class="!px-1.5 !py-0.5 !text-[11px]">Return</Kbd>
          </div>
          <p class="text-xs text-graphite-400 text-center">to add a comment</p>
        {/if}
      </div>
    {/if}
  </div>
</div>
