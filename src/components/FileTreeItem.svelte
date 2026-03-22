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
      case "M": return "text-amber-300";
      case "A": return "text-green-400";
      case "?": return "text-teal-400";
      case "D": return "text-red-400";
      case "R": return "text-purple-400";
      default: return "text-graphite-400";
    }
  }
</script>

<div
  class="flex items-center gap-1.5 py-1 pr-3 cursor-pointer text-xs whitespace-nowrap select-none min-h-7 transition-colors
    {isSelected ? 'bg-[var(--bg-selection)] text-amber-400' : entry.isDir ? 'text-graphite-50 font-medium' : 'text-graphite-200'}
    {!isSelected ? 'hover:bg-graphite-800 hover:text-graphite-50' : ''}"
  style="padding-left: {depth * 18 + 12}px"
  onclick={handleClick}
  role="treeitem"
  aria-selected={isSelected}
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && handleClick()}
>
  <span class="w-3.5 flex items-center justify-center shrink-0">
    {#if entry.isDir}
      <span class="text-[10px] text-graphite-400">{isExpanded ? "▾" : "▸"}</span>
    {:else}
      <span class="w-1 h-1 rounded-full bg-graphite-400 opacity-50"></span>
    {/if}
  </span>
  <span class="overflow-hidden text-ellipsis">{entry.name}</span>
  {#if entry.hasSidecar}
    <span class="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0 opacity-80"></span>
  {/if}
  {#if gitStatus}
    <span class="text-[11px] font-semibold ml-auto shrink-0 font-mono {gitStatusClass(gitStatus.status)}">{gitStatus.status}</span>
  {/if}
</div>

{#if entry.isDir && isExpanded}
  {#each children as child (child.path)}
    <FileTreeItem entry={child} depth={depth + 1} {onFileSelect} {selectedPath} {changedPaths} />
  {/each}
{/if}
