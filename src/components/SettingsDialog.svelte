<script lang="ts">
  import { getSettings, updateSettings } from "$lib/tauri";
  import { onMount } from "svelte";

  let { onClose }: { onClose: () => void } = $props();

  let author = $state("");
  let defaultLabels = $state("");

  onMount(async () => {
    const [a, labels] = await getSettings();
    author = a;
    defaultLabels = labels.join(", ");
  });

  async function save() {
    const labels = defaultLabels
      .split(",")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    await updateSettings(author, labels);
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) save();
  }
</script>

<div class="overlay" onclick={onClose} role="dialog" tabindex="-1">
  <div class="dialog" onclick={(e) => e.stopPropagation()} onkeydown={handleKeydown} role="document">
    <h3>Settings</h3>

    <label>
      Author name
      <input type="text" bind:value={author} />
    </label>

    <label>
      Default labels (comma-separated)
      <input type="text" bind:value={defaultLabels} placeholder="todo, bug, question" />
    </label>

    <div class="actions">
      <button onclick={onClose}>Cancel</button>
      <button class="save-btn" onclick={save}>Save</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .dialog {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 20px;
    width: 360px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h3 {
    font-size: 16px;
    font-weight: 600;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--text-subtle);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  .actions button {
    padding: 4px 14px;
    border-radius: 4px;
    font-size: 13px;
  }

  .save-btn {
    background: var(--accent-blue);
    color: var(--bg-primary);
    font-weight: 600;
  }
</style>
