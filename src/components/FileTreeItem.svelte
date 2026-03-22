<script lang="ts">
  import type { FileEntry } from "$lib/types";
  import { toggleFolder, getWorkspace, getGitStatusForFile } from "$lib/stores/workspace.svelte";
  import FileTreeItem from "./FileTreeItem.svelte";

  let {
    entry,
    depth = 0,
    onFileSelect,
    selectedPath,
    changedPaths = null,
  }: {
    entry: FileEntry;
    depth?: number;
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
    changedPaths?: Set<string> | null;
  } = $props();

  const workspace = getWorkspace();

  let isExpanded = $derived(workspace.expandedFolders.has(entry.path));
  let allChildren = $derived(workspace.fileTree.get(entry.path) ?? []);
  let children = $derived(changedPaths ? allChildren.filter(c => changedPaths.has(c.path)) : allChildren);
  let gitStatus = $derived(getGitStatusForFile(entry.path));
  let isSelected = $derived(selectedPath === entry.path);

  function handleClick() {
    if (entry.isDir) {
      toggleFolder(entry.path);
    } else {
      onFileSelect(entry.path);
    }
  }

  function gitStatusClass(status: string): string {
    switch (status) {
      case "M": return "text-warning";
      case "A": return "text-success";
      case "?": return "text-accent-teal";
      case "D": return "text-danger";
      case "R": return "text-accent-purple";
      default: return "text-text-secondary";
    }
  }
</script>

<div
  class="flex items-center gap-1.5 py-1 pr-3 cursor-pointer text-xs whitespace-nowrap select-none min-h-7 transition-colors
    {isSelected ? 'bg-surface-selection text-accent' : entry.isDir ? 'text-text-primary font-medium' : 'text-text-primary'}
    {!isSelected ? 'hover:bg-surface-highlight hover:text-text-primary' : ''}"
  style="padding-left: {depth * 18 + 12}px"
  onclick={handleClick}
  role="treeitem"
  aria-selected={isSelected}
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && handleClick()}
>
  <span class="w-3.5 flex items-center justify-center shrink-0">
    {#if entry.isDir}
      <span class="text-xs text-text-secondary">{isExpanded ? "▾" : "▸"}</span>
    {:else}
      <span class="w-1 h-1 rounded-full bg-border-emphasis opacity-50"></span>
    {/if}
  </span>
  <span class="overflow-hidden text-ellipsis">{entry.name}</span>
  <div class="flex items-center gap-1.5 ml-auto shrink-0">
    {#if entry.hasSidecar}
      <span class="flex items-center gap-0.5 text-xs font-semibold tracking-wide uppercase px-1 py-px rounded bg-accent-subtle text-accent border border-accent/20">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z"/>
        </svg>
      </span>
    {/if}
    {#if gitStatus}
      <span class="text-xs font-semibold font-mono {gitStatusClass(gitStatus.status)}">{gitStatus.status}</span>
    {/if}
  </div>
</div>

{#if entry.isDir && isExpanded}
  {#each children as child (child.path)}
    <FileTreeItem entry={child} depth={depth + 1} {onFileSelect} {selectedPath} {changedPaths} />
  {/each}
{/if}
