<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { getWorkspace, addRootFolder, removeRootFolder, toggleShowChangedOnly, getChangedFilePaths } from "$lib/stores/workspace.svelte";
  import FileTreeItem from "./FileTreeItem.svelte";

  let {
    onFileSelect,
    selectedPath,
  }: {
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
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

  async function openFolder() {
    const selected = await open({ directory: true, multiple: true });
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      for (const path of paths) {
        if (path) await addRootFolder(path);
      }
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
      workspace.expandedFolders.clear();
    }
    closeContextMenu();
  }
</script>

<svelte:window onclick={() => showContextMenu && closeContextMenu()} />

<div class="h-full overflow-y-auto overflow-x-hidden" role="tree">
  {#if workspace.rootFolders.length > 0}
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-graphite-700">
      <span class="text-[10px] font-semibold uppercase text-graphite-500 tracking-wider">Folders</span>
      <div class="flex items-center gap-1.5">
        <button
          class="transition-colors {workspace.showChangedOnly ? 'text-amber-400' : 'text-graphite-400 hover:text-graphite-200'}"
          onclick={toggleShowChangedOnly}
          title={workspace.showChangedOnly ? "Show all files" : "Show changed files only"}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M12 20h9M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z" />
          </svg>
        </button>
        <button
          class="text-graphite-400 hover:text-graphite-200 transition-colors"
          onclick={openFolder}
          title="Add folder"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
      </div>
    </div>
  {/if}

  {#each workspace.rootFolders as folder (folder)}
    {@const allEntries = workspace.fileTree.get(folder) ?? []}
    {@const entries = changedPaths ? allEntries.filter(e => changedPaths.has(e.path)) : allEntries}
    {#if !changedPaths || entries.length > 0}
      <div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="flex items-center gap-1 px-3 py-1.5 text-[11px] font-semibold uppercase text-graphite-400 tracking-wider select-none cursor-pointer hover:text-graphite-200 transition-colors"
          onclick={() => toggleRoot(folder)}
          oncontextmenu={(e) => handleContextMenu(e, folder)}
        >
          <span class="text-[10px]">{collapsedRoots.has(folder) ? "▸" : "▾"}</span>
          {folder.split("/").pop()}
        </div>
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
        class="px-4 py-1.5 rounded-md text-sm font-semibold bg-amber-400 text-graphite-950 hover:bg-amber-300 transition-colors"
        onclick={openFolder}
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
    class="fixed z-50 min-w-[160px] py-1 bg-graphite-900 border border-graphite-700 rounded-lg shadow-xl"
    style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px"
    role="menu"
  >
    <button
      class="w-full text-left px-3 py-1.5 text-xs text-graphite-50 hover:bg-graphite-800 transition-colors"
      onclick={handleOpenInFinder}
      role="menuitem"
    >
      Open in Finder
    </button>
    <button
      class="w-full text-left px-3 py-1.5 text-xs text-graphite-50 hover:bg-graphite-800 transition-colors"
      onclick={handleCollapseAll}
      role="menuitem"
    >
      Collapse All
    </button>
    <div class="my-1 border-t border-graphite-700"></div>
    <button
      class="w-full text-left px-3 py-1.5 text-xs text-red-400 hover:bg-graphite-800 transition-colors"
      onclick={handleRemoveFolder}
      role="menuitem"
    >
      Remove Folder
    </button>
  </div>
{/if}
