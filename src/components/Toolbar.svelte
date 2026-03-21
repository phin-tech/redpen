<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { addRootFolder } from "$lib/stores/workspace.svelte";

  let { onSettingsClick }: { onSettingsClick: () => void } = $props();

  async function openFolder() {
    const selected = await open({ directory: true, multiple: true });
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      for (const path of paths) {
        if (path) await addRootFolder(path);
      }
    }
  }
</script>

<div class="toolbar">
  <div class="toolbar-left">
    <button class="toolbar-btn" onclick={openFolder} title="Open Folder">
      + Open Folder
    </button>
  </div>
  <div class="toolbar-right">
    <button class="toolbar-btn" onclick={onSettingsClick} title="Settings">
      Settings
    </button>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-color);
    height: 36px;
    -webkit-app-region: drag;
  }

  .toolbar-left, .toolbar-right {
    display: flex;
    gap: 8px;
    -webkit-app-region: no-drag;
  }

  .toolbar-btn {
    padding: 2px 10px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-subtle);
  }

  .toolbar-btn:hover {
    background: var(--bg-highlight);
    color: var(--text-primary);
  }
</style>
