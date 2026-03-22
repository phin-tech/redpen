<script lang="ts">
  import { getSettings, updateSettings } from "$lib/tauri";
  import { onMount } from "svelte";
  import { Button, Label, Input } from "flowbite-svelte";

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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-[200]" onclick={onClose}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="bg-graphite-950 border border-graphite-700 rounded-xl p-6 w-[380px] flex flex-col gap-4 shadow-2xl"
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleKeydown}
    role="dialog"
    tabindex="-1"
  >
    <h3 class="text-base font-semibold text-graphite-50">Settings</h3>

    <div class="flex flex-col gap-1.5">
      <Label class="!text-xs !text-graphite-200 !font-medium">Author name</Label>
      <Input bind:value={author} size="sm" class="!bg-graphite-900 !border-graphite-700 !text-graphite-50" />
    </div>

    <div class="flex flex-col gap-1.5">
      <Label class="!text-xs !text-graphite-200 !font-medium">Default labels (comma-separated)</Label>
      <Input bind:value={defaultLabels} size="sm" placeholder="todo, bug, question" class="!bg-graphite-900 !border-graphite-700 !text-graphite-50" />
    </div>

    <div class="flex justify-end gap-2 mt-1">
      <Button size="xs" color="alternative" onclick={onClose}>Cancel</Button>
      <Button size="xs" color="primary" onclick={save}>Save</Button>
    </div>
  </div>
</div>
