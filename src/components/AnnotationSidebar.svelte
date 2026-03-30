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
  import Kbd from "./ui/Kbd.svelte";
  import Button from "./ui/Button.svelte";
  import { formatShortcut } from "$lib/shortcuts";
  import AnnotationCard from "./AnnotationCard.svelte";

  let {
    onAnnotationClick,
    onFileSelect,
    onCollapse,
  }: {
    onAnnotationClick: (line: number) => void;
    onFileSelect?: (path: string) => void;
    onCollapse?: () => void;
  } = $props();

  const annotationsState = getAnnotationsState();
  const editor = getEditor();
  const workspace = getWorkspace();

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
  const addAnnotationShortcut = formatShortcut(["Mod", "Enter"]);
</script>

<div class="h-full flex flex-col">
  <!-- View toggle -->
  <div class="sidebar-header">
    {#if onCollapse}
      <button class="sidebar-collapse-btn" onclick={onCollapse} title="Collapse annotations">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 18l6-6-6-6" />
        </svg>
      </button>
    {/if}
    <button
      class="sidebar-tab"
      class:active={annotationsState.sidebarView === 'file'}
      onclick={() => switchView('file')}
    >
      Current File
      {#if annotations.length > 0}
        <span class="sidebar-tab-badge">{annotations.length}</span>
      {/if}
      {#if annotationsState.sidebarView === 'file'}
        <span class="sidebar-tab-indicator"></span>
      {/if}
    </button>
    <button
      class="sidebar-tab"
      class:active={annotationsState.sidebarView === 'project'}
      onclick={() => switchView('project')}
    >
      All Files
      {#if totalProjectAnnotations > 0}
        <span class="sidebar-tab-badge">{totalProjectAnnotations}</span>
      {/if}
      {#if annotationsState.sidebarView === 'project'}
        <span class="sidebar-tab-indicator"></span>
      {/if}
    </button>
  </div>

  <!-- Current file view -->
  {#if annotationsState.sidebarView === 'file'}
    <div class="flex-1 overflow-y-auto px-1.5 py-1">
      {#each annotations as annotation (annotation.id)}
        {#if editingId === annotation.id}
          <div class="p-2.5 my-1 flex flex-col gap-2 rounded-md" style="background: var(--surface-base); box-shadow: var(--shadow-card)">
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
              <Kbd>{addAnnotationShortcut[0]}</Kbd>
              <span>+</span>
              <Kbd>{addAnnotationShortcut[1]}</Kbd>
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

</div>

<style>
  .sidebar-header {
    display: flex;
    align-items: stretch;
    padding: 8px 10px;
    min-height: 52px;
    box-sizing: border-box;
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .sidebar-collapse-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--text-ghost);
    cursor: pointer;
    padding: 0;
    transition: color 0.15s;
  }
  .sidebar-collapse-btn:hover {
    color: var(--text-muted);
  }
  .sidebar-tab {
    flex: 1;
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 4px 10px;
    min-height: 30px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: color 0.15s;
  }
  .sidebar-tab:hover {
    color: var(--text-primary);
  }
  .sidebar-tab.active {
    color: var(--text-primary);
  }
  .sidebar-tab-badge {
    font-size: 10px;
    font-family: var(--font-mono);
    padding: 0 4px;
    border-radius: 4px;
    background: var(--surface-raised);
    color: var(--text-secondary);
  }
  .sidebar-tab-indicator {
    position: absolute;
    bottom: -8px;
    left: 8px;
    right: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 9999px;
  }
</style>
