<script lang="ts">
  import { getSettings, updateSettings } from "$lib/tauri";
  import { onMount } from "svelte";
  import Button from "./ui/Button.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let author = $state("");
  let defaultLabels = $state("");
  let ignoredFolderNames = $state("");
  const authorInputId = "settings-author";
  const defaultLabelsInputId = "settings-default-labels";
  const ignoredFoldersInputId = "settings-ignored-folders";

  onMount(async () => {
    const settings = await getSettings();
    author = settings.author;
    defaultLabels = settings.defaultLabels.join(", ");
    ignoredFolderNames = settings.ignoredFolderNames.join(", ");
  });

  async function save() {
    const labels = defaultLabels
      .split(",")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    const ignoredFolders = ignoredFolderNames
      .split(",")
      .map((name) => name.trim())
      .filter((name) => name.length > 0);
    await updateSettings({
      author,
      defaultLabels: labels,
      ignoredFolderNames: ignoredFolders,
    });
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) save();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="fixed inset-0 bg-black/60 backdrop-blur-[2px] flex items-center justify-center z-[200]" onclick={onClose}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="border border-border-default/60 rounded-xl p-6 w-[380px] flex flex-col gap-4"
    style="background: var(--gradient-panel), var(--surface-panel); box-shadow: var(--shadow-popover), 0 0 0 1px var(--border-subtle)"
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleKeydown}
    role="dialog"
    tabindex="-1"
  >
    <h3 class="text-base font-semibold text-text-primary">Settings</h3>

    <div class="flex flex-col gap-1.5">
      <label class="text-xs text-text-secondary font-medium" for={authorInputId}>Author name</label>
      <input
        id={authorInputId}
        bind:value={author}
        class="w-full bg-surface-panel border border-border-default/60 text-text-primary text-sm rounded-md px-2.5 py-1.5 focus:border-accent focus:ring-1 focus:ring-accent/20 outline-none transition-colors"
        style="box-shadow: var(--shadow-inset)"
      />
    </div>

    <div class="flex flex-col gap-1.5">
      <label class="text-xs text-text-secondary font-medium" for={defaultLabelsInputId}>Default labels (comma-separated)</label>
      <input
        id={defaultLabelsInputId}
        bind:value={defaultLabels}
        placeholder="todo, bug, question"
        class="w-full bg-surface-panel border border-border-default/60 text-text-primary text-sm rounded-md px-2.5 py-1.5 focus:border-accent focus:ring-1 focus:ring-accent/20 outline-none transition-colors"
        style="box-shadow: var(--shadow-inset)"
      />
    </div>

    <div class="flex flex-col gap-1.5">
      <label class="text-xs text-text-secondary font-medium" for={ignoredFoldersInputId}>Ignored folders (comma-separated)</label>
      <input
        id={ignoredFoldersInputId}
        bind:value={ignoredFolderNames}
        placeholder=".git, node_modules, .venv"
        class="w-full bg-surface-panel border border-border-default/60 text-text-primary text-sm rounded-md px-2.5 py-1.5 focus:border-accent focus:ring-1 focus:ring-accent/20 outline-none transition-colors"
        style="box-shadow: var(--shadow-inset)"
      />
    </div>

    <div class="flex justify-end gap-2 mt-1">
      <Button variant="secondary" size="sm" onclick={onClose}>Cancel</Button>
      <Button size="sm" onclick={save}>Save</Button>
    </div>
  </div>
</div>
