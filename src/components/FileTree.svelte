<script lang="ts">
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import FileTreeItem from "./FileTreeItem.svelte";

  let {
    onFileSelect,
    selectedPath,
  }: {
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
  } = $props();

  const workspace = getWorkspace();
</script>

<div class="file-tree" role="tree">
  {#each workspace.rootFolders as folder (folder)}
    {@const entries = workspace.fileTree.get(folder) ?? []}
    <div class="root-folder">
      <div class="root-label">{folder.split("/").pop()}</div>
      {#each entries as entry (entry.path)}
        <FileTreeItem {entry} {onFileSelect} {selectedPath} />
      {/each}
    </div>
  {/each}

  {#if workspace.rootFolders.length === 0}
    <div class="empty">Open a folder to get started</div>
  {/if}
</div>

<style>
  .file-tree {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .root-label {
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--text-muted);
    letter-spacing: 0.5px;
  }

  .empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
