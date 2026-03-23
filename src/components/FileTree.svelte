<script lang="ts">
  import { getWorkspace, removeRootFolder, getChangedFilePaths } from "$lib/stores/workspace.svelte";
  import FileTreeItem from "./FileTreeItem.svelte";
  import IconButton from "./ui/IconButton.svelte";

  let {
    onFileSelect,
    selectedPath,
    onOpenFolder,
    onExpandAll,
    onCollapseAll,
    onToggleShowChangedOnly,
  }: {
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
    onOpenFolder: () => Promise<void>;
    onExpandAll: () => Promise<void>;
    onCollapseAll: () => void;
    onToggleShowChangedOnly: () => void;
  } = $props();

  import { SvelteSet } from "svelte/reactivity";

  const workspace = getWorkspace();
  let collapsedRoots = new SvelteSet<string>();
  let changedPaths = $derived(workspace.showChangedOnly ? getChangedFilePaths() : null);

  function toggleRoot(folder: string) {
    if (collapsedRoots.has(folder)) {
      collapsedRoots.delete(folder);
    } else {
      collapsedRoots.add(folder);
    }
  }

  // Context menu state
  let contextMenuFolder: string | null = $state(null);
  let contextMenuPos = $state({ x: 0, y: 0 });
  let showContextMenu = $state(false);

  function handleContextMenu(e: MouseEvent, folder: string) {
    e.preventDefault();
    e.stopPropagation();
    contextMenuFolder = folder;
    contextMenuPos = { x: e.clientX, y: e.clientY };
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
    contextMenuFolder = null;
  }

  async function handleRemoveFolder() {
    if (contextMenuFolder) {
      await removeRootFolder(contextMenuFolder);
    }
    closeContextMenu();
  }

  async function handleOpenInFinder() {
    if (contextMenuFolder) {
      const { open: shellOpen } = await import("@tauri-apps/plugin-shell");
      await shellOpen(contextMenuFolder);
    }
    closeContextMenu();
  }

  function handleCollapseAll() {
    if (contextMenuFolder) {
      onCollapseAll();
    }
    closeContextMenu();
  }
</script>

<svelte:window onclick={() => showContextMenu && closeContextMenu()} />

<div class="h-full overflow-y-auto overflow-x-hidden" role="tree">
  {#if workspace.rootFolders.length > 0}
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-border-default">
      <span class="text-xs font-semibold uppercase text-text-muted tracking-wider">Folders</span>
      <div class="flex items-center gap-1.5">
        <IconButton label="Expand all" onclick={onExpandAll}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 8l5 5 5-5" />
            <path d="M7 13l5 5 5-5" />
          </svg>
        </IconButton>
        <IconButton label="Collapse all" onclick={onCollapseAll}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 16l5-5 5 5" />
            <path d="M7 11l5-5 5 5" />
          </svg>
        </IconButton>
        <IconButton label={workspace.showChangedOnly ? "Show all files" : "Changed only"} active={workspace.showChangedOnly} onclick={onToggleShowChangedOnly}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M12 20h9M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z" />
          </svg>
        </IconButton>
        <IconButton label="Add folder" onclick={onOpenFolder}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </IconButton>
      </div>
    </div>
  {/if}

  {#each workspace.rootFolders as folder (folder)}
    {@const allEntries = workspace.fileTree.get(folder) ?? []}
    {@const entries = changedPaths ? allEntries.filter(e => changedPaths.has(e.path)) : allEntries}
    {#if !changedPaths || entries.length > 0}
      <div>
        <button
          type="button"
          class="flex w-full items-center gap-1 px-3 py-1.5 text-xs font-semibold uppercase text-text-secondary tracking-wider select-none cursor-pointer hover:text-text-primary transition-colors text-left"
          onclick={() => toggleRoot(folder)}
          oncontextmenu={(e) => handleContextMenu(e, folder)}
        >
          <span class="text-xs">{collapsedRoots.has(folder) ? "▸" : "▾"}</span>
          {folder.split("/").pop()}
        </button>
        {#if !collapsedRoots.has(folder)}
          {#each entries as entry (entry.path)}
            <FileTreeItem {entry} {onFileSelect} {selectedPath} {changedPaths} />
          {/each}
        {/if}
      </div>
    {/if}
  {/each}

  {#if workspace.rootFolders.length === 0}
    <div class="flex justify-center p-6">
      <button
        class="px-4 py-1.5 rounded-md text-sm font-semibold bg-accent text-surface-base hover:bg-accent-hover transition-colors"
        onclick={onOpenFolder}
      >
        Open Folder
      </button>
    </div>
  {/if}
</div>

{#if showContextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="fixed inset-0 z-40" onclick={closeContextMenu}></div>
  <div
    class="fixed z-50 min-w-[160px] py-1 border border-border-default/60 rounded-lg backdrop-blur-sm"
    style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px; background: var(--gradient-panel), var(--surface-raised); box-shadow: var(--shadow-popover), 0 0 0 1px var(--border-subtle)"
    role="menu"
  >
    <button
      class="w-full text-left px-3 py-1.5 text-xs text-text-primary hover:bg-surface-highlight transition-colors"
      onclick={handleOpenInFinder}
      role="menuitem"
    >
      Open in Finder
    </button>
    <button
      class="w-full text-left px-3 py-1.5 text-xs text-text-primary hover:bg-surface-highlight transition-colors"
      onclick={handleCollapseAll}
      role="menuitem"
    >
      Collapse All
    </button>
    <div class="my-1 border-t border-border-default"></div>
    <button
      class="w-full text-left px-3 py-1.5 text-xs text-danger hover:bg-surface-highlight transition-colors"
      onclick={handleRemoveFolder}
      role="menuitem"
    >
      Remove Folder
    </button>
  </div>
{/if}
