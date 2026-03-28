<script lang="ts">
  import {
    getAnnotationsState,
    sortedAnnotations,
    selectAnnotation,
    removeAnnotation,
    editAnnotation,
    updateChoices,
    setSidebarView,
    loadProjectAnnotations,
  } from "$lib/stores/annotations.svelte";
  import { getEditor } from "$lib/stores/editor.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { clearReviewSession } from "$lib/stores/review.svelte";
  import { openReviewPage } from "$lib/stores/reviewPage.svelte";
  import Kbd from "./ui/Kbd.svelte";
  import Button from "./ui/Button.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import AnnotationCard from "./AnnotationCard.svelte";

  let {
    onAnnotationClick,
    onFileSelect,
  }: {
    onAnnotationClick: (line: number) => void;
    onFileSelect?: (path: string) => void;
  } = $props();

  const annotationsState = getAnnotationsState();
  const editor = getEditor();
  const workspace = getWorkspace();

  let editingId: string | null = $state(null);
  let editBody = $state("");
  let reviewDone: string | null = $state(null);

  async function handleReviewVerdict(verdict: "approved" | "changes_requested") {
    if (!editor.currentFilePath) return;
    await invoke("signal_review_done", { filePath: editor.currentFilePath, verdict });
    clearReviewSession();
    reviewDone = verdict;
    setTimeout(() => (reviewDone = null), 3000);
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

  function switchView(view: "file" | "project") {
    setSidebarView(view);
    if (view === "project" && workspace.rootFolders.length > 0) {
      loadProjectAnnotations(workspace.rootFolders[0]);
    }
  }

  function handleProjectAnnotationClick(filePath: string, line: number) {
    if (onFileSelect && filePath !== editor.currentFilePath) {
      onFileSelect(filePath);
      // Wait for file to load, then scroll
      setTimeout(() => onAnnotationClick(line), 200);
    } else {
      onAnnotationClick(line);
    }
  }

  let annotations = $derived(sortedAnnotations());

  let totalProjectAnnotations = $derived(
    annotationsState.projectAnnotations.reduce((sum, f) => sum + f.annotations.length, 0)
  );
</script>

<div class="h-full flex flex-col">
  <!-- View toggle -->
  <div class="flex border-b border-border-default/60" style="box-shadow: var(--shadow-xs)">
    <button
      class="flex-1 px-3 py-2 text-xs font-medium transition-colors relative
        {annotationsState.sidebarView === 'file'
          ? 'text-text-primary'
          : 'text-text-secondary hover:text-text-primary'}"
      onclick={() => switchView('file')}
    >
      Current File
      {#if annotations.length > 0}
        <span class="ml-1 text-xs font-mono px-1 py-px rounded bg-surface-raised text-text-secondary">{annotations.length}</span>
      {/if}
      {#if annotationsState.sidebarView === 'file'}
        <span class="absolute bottom-0 left-2 right-2 h-0.5 bg-accent rounded-full"></span>
      {/if}
    </button>
    <button
      class="flex-1 px-3 py-2 text-xs font-medium transition-colors relative
        {annotationsState.sidebarView === 'project'
          ? 'text-text-primary'
          : 'text-text-secondary hover:text-text-primary'}"
      onclick={() => switchView('project')}
    >
      All Files
      {#if totalProjectAnnotations > 0}
        <span class="ml-1 text-xs font-mono px-1 py-px rounded bg-surface-raised text-text-secondary">{totalProjectAnnotations}</span>
      {/if}
      {#if annotationsState.sidebarView === 'project'}
        <span class="absolute bottom-0 left-2 right-2 h-0.5 bg-accent rounded-full"></span>
      {/if}
    </button>
  </div>

  <!-- Current file view -->
  {#if annotationsState.sidebarView === 'file'}
    <div class="flex-1 overflow-y-auto px-1.5 py-1">
      {#each annotations as annotation (annotation.id)}
        {#if editingId === annotation.id}
          <div class="p-2.5 my-1 flex flex-col gap-2 rounded-md" style="background: var(--gradient-panel), var(--surface-base); box-shadow: var(--shadow-card)">
            <textarea
              bind:value={editBody}
              rows="3"
              class="w-full bg-surface-panel border-border-default/60 text-text-primary text-sm rounded-md p-2 focus:border-accent focus:ring-accent/20"
              style="box-shadow: var(--shadow-inset)"
            ></textarea>
            <div class="flex justify-end gap-1.5">
              <Button variant="secondary" size="sm" onclick={() => (editingId = null)}>Cancel</Button>
              <Button size="sm" onclick={saveEdit}>Save</Button>
            </div>
          </div>
        {:else}
          <AnnotationCard
            {annotation}
            isReply={!!annotation.replyTo}
            isSelected={annotationsState.selectedAnnotationId === annotation.id}
            onClick={() => handleClick(annotation.id, annotation.anchor.range.startLine)}
            onDoubleClick={() => handleDoubleClick(annotation.id, annotation.body)}
            onDelete={() => handleDelete(annotation.id)}
            onChoiceToggle={(choiceIndex) => {
              if (!editor.currentFilePath || !annotation.choices) return;
              const newChoices = annotation.choices.map((c, i) => {
                if (annotation.selectionMode === "single") {
                  return { ...c, selected: i === choiceIndex };
                }
                return i === choiceIndex ? { ...c, selected: !c.selected } : c;
              });
              updateChoices(editor.currentFilePath, annotation.id, newChoices);
            }}
          />
        {/if}
      {/each}

      {#if annotations.length === 0}
        <div class="flex flex-col items-center justify-center px-5 py-10 gap-2 h-full min-h-[200px]">
          {#if !editor.currentFilePath}
            <svg class="text-text-secondary opacity-40 mb-1" width="32" height="32" viewBox="0 0 24 24" fill="none">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" stroke-width="1.5"/>
              <path d="M14 2v6h6M12 18v-6M9 15h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <p class="text-sm font-medium text-text-primary">No file selected</p>
            <p class="text-xs text-text-secondary text-center">Open a file from the tree to start annotating</p>
          {:else}
            <svg class="text-text-secondary opacity-40 mb-1" width="32" height="32" viewBox="0 0 24 24" fill="none">
              <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <p class="text-sm font-medium text-text-primary">No annotations yet</p>
            <p class="text-xs text-text-secondary text-center">Select text in the editor, then</p>
            <div class="flex items-center gap-1 my-1 text-xs text-text-secondary">
              <Kbd>Cmd</Kbd>
              <span>+</span>
              <Kbd>Return</Kbd>
            </div>
            <p class="text-xs text-text-secondary text-center">to add a comment</p>
          {/if}
        </div>
      {/if}
    </div>

  <!-- Project-wide view -->
  {:else}
    <div class="flex-1 overflow-y-auto">
      {#if annotationsState.projectAnnotationsLoading}
        <div class="flex items-center justify-center py-10 text-xs text-text-secondary">
          Loading annotations...
        </div>
      {:else if annotationsState.projectAnnotations.length === 0}
        <div class="flex flex-col items-center justify-center px-5 py-10 gap-2 h-full min-h-[200px]">
          <svg class="text-text-secondary opacity-40 mb-1" width="32" height="32" viewBox="0 0 24 24" fill="none">
            <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <p class="text-sm font-medium text-text-primary">No annotations in project</p>
          <p class="text-xs text-text-secondary text-center">Annotate files to see them here</p>
        </div>
      {:else}
        {#each annotationsState.projectAnnotations as fileGroup (fileGroup.filePath)}
          <div class="border-b border-border-default/40">
            <button
              class="w-full flex items-center gap-2 px-3 py-1.5 text-left hover:bg-surface-highlight/50 transition-colors
                {fileGroup.filePath === editor.currentFilePath ? 'bg-surface-highlight/30' : ''}"
              onclick={() => onFileSelect?.(fileGroup.filePath)}
            >
              <span class="text-xs font-medium text-text-primary truncate flex-1">{fileGroup.fileName}</span>
              <span class="text-xs font-mono px-1.5 py-0.5 rounded bg-surface-raised text-text-secondary shrink-0">{fileGroup.annotations.length}</span>
            </button>
            <div class="px-1.5 pb-1">
              {#each fileGroup.annotations as annotation (annotation.id)}
                <div
                  class="group px-2.5 py-1.5 my-0.5 cursor-pointer rounded text-sm text-text-secondary hover:bg-surface-highlight/50 transition-colors truncate"
                  onclick={() => handleProjectAnnotationClick(fileGroup.filePath, annotation.anchor.range.startLine)}
                  role="button"
                  tabindex="0"
                  onkeydown={(e) => e.key === "Enter" && handleProjectAnnotationClick(fileGroup.filePath, annotation.anchor.range.startLine)}
                >
                  <span class="text-xs text-text-muted font-mono mr-1.5">L{annotation.anchor.range.startLine}</span>
                  {annotation.body}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Bottom action bar -->
  {#if annotationsState.sidebarView === 'file' && editor.currentFilePath}
    <div class="px-2.5 py-2 border-t border-border-default/60" style="box-shadow: var(--shadow-xs)">
      <div class="flex gap-2 mb-2">
        <Button variant="secondary" onclick={() => openReviewPage("changes")} class="flex-1 text-xs">
          Review
        </Button>
      </div>
      {#if reviewDone}
        <Button variant="secondary" disabled class="w-full {reviewDone === 'approved' ? 'bg-success/20 text-success border-success/30' : 'bg-warning/20 text-warning border-warning/30'}">
          {reviewDone === 'approved' ? 'Approved!' : 'Changes requested!'}
        </Button>
      {:else}
        <div class="flex gap-2">
          <Button variant="secondary" onclick={() => handleReviewVerdict("approved")} class="flex-1">
            Approve
          </Button>
          <Button onclick={() => handleReviewVerdict("changes_requested")} class="flex-1">
            Request changes
          </Button>
        </div>
      {/if}
    </div>
  {/if}
</div>
