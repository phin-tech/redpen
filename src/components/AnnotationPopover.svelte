<script lang="ts">
  import { Button, Kbd } from "flowbite-svelte";

  let {
    onSubmit,
    onCancel,
    position = { x: 0, y: 0 },
  }: {
    onSubmit: (body: string, labels: string[]) => void;
    onCancel: () => void;
    position: { x: number; y: number };
  } = $props();

  let body = $state("");
  let labelsInput = $state("");
  let textareaEl: HTMLTextAreaElement;

  $effect(() => {
    textareaEl?.focus();
  });

  function handleSubmit() {
    if (!body.trim()) return;
    const labels = labelsInput
      .split(",")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    onSubmit(body.trim(), labels);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
    if (e.key === "Escape") {
      onCancel();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="fixed inset-0 z-[99]" onclick={onCancel}></div>
<div
  class="fixed z-[100] bg-graphite-950 border border-graphite-700 rounded-xl p-3.5 w-[340px] shadow-2xl flex flex-col gap-2.5"
  style="left: {position.x}px; top: {position.y}px"
  onkeydown={handleKeydown}
  role="dialog"
  tabindex="-1"
>
  <textarea
    bind:this={textareaEl}
    bind:value={body}
    placeholder="Add your annotation..."
    rows="3"
    class="w-full bg-graphite-900 border-graphite-700 text-graphite-50 text-sm rounded-md p-2 resize-y min-h-[70px] focus:border-amber-400 focus:ring-amber-400/20"
  ></textarea>

  <input
    type="text"
    bind:value={labelsInput}
    placeholder="Labels (comma-separated)"
    class="w-full bg-graphite-900 border-graphite-700 text-graphite-50 text-sm rounded-md p-2 focus:border-amber-400 focus:ring-amber-400/20"
  />

  <div class="flex justify-end gap-2">
    <Button size="xs" color="alternative" onclick={onCancel}>Cancel</Button>
    <Button size="xs" color="primary" onclick={handleSubmit} class="gap-2">
      Save
      <Kbd class="!px-1 !py-0 !text-[10px] opacity-70">Cmd+Return</Kbd>
    </Button>
  </div>
</div>
