<script lang="ts">
  import type { FileEntry, GitFileStatus } from "$lib/types";
  import { toggleFolder, getWorkspace, getGitStatusForFile } from "$lib/stores/workspace.svelte";
  import FileTreeItem from "./FileTreeItem.svelte";

  let {
    entry,
    depth = 0,
    onFileSelect,
    selectedPath,
  }: {
    entry: FileEntry;
    depth?: number;
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
  } = $props();

  const workspace = getWorkspace();

  let isExpanded = $derived(workspace.expandedFolders.has(entry.path));
  let children = $derived(workspace.fileTree.get(entry.path) ?? []);
  let gitStatus = $derived(getGitStatusForFile(entry.path));
  let isSelected = $derived(selectedPath === entry.path);

  function handleClick() {
    if (entry.isDir) {
      toggleFolder(entry.path);
    } else {
      onFileSelect(entry.path);
    }
  }

  function gitStatusColor(status: string): string {
    switch (status) {
      case "M": return "var(--accent-yellow)";
      case "A": return "var(--accent-green)";
      case "?": return "var(--accent-teal)";
      case "D": return "var(--accent-red)";
      case "R": return "var(--accent-purple)";
      default: return "var(--text-muted)";
    }
  }
</script>

<div
  class="tree-item"
  class:selected={isSelected}
  style="padding-left: {depth * 16 + 8}px"
  onclick={handleClick}
  role="treeitem"
  aria-selected={isSelected}
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && handleClick()}
>
  <span class="icon">
    {#if entry.isDir}
      {isExpanded ? "▾" : "▸"}
    {:else}
      &nbsp;
    {/if}
  </span>
  <span class="name">{entry.name}</span>
  {#if entry.hasSidecar}
    <span class="sidecar-dot"></span>
  {/if}
  {#if gitStatus}
    <span class="git-status" style="color: {gitStatusColor(gitStatus.status)}">{gitStatus.status}</span>
  {/if}
</div>

{#if entry.isDir && isExpanded}
  {#each children as child (child.path)}
    <FileTreeItem entry={child} depth={depth + 1} {onFileSelect} {selectedPath} />
  {/each}
{/if}

<style>
  .tree-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
    user-select: none;
  }

  .tree-item:hover {
    background: var(--bg-highlight);
  }

  .tree-item.selected {
    background: var(--bg-highlight);
    color: var(--accent-blue);
  }

  .icon {
    width: 14px;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sidecar-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-green);
    flex-shrink: 0;
  }

  .git-status {
    font-size: 11px;
    font-weight: 600;
    margin-left: auto;
    flex-shrink: 0;
  }
</style>
