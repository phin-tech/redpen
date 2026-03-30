<script lang="ts">
  import Kbd from "./ui/Kbd.svelte";
  import Button from "./ui/Button.svelte";
  import { formatShortcut } from "$lib/shortcuts";

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
  const submitShortcut = formatShortcut(["Mod", "Enter"]);

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
  class="fixed z-[100] border border-border-default/60 rounded-xl p-3.5 w-[340px] flex flex-col gap-2.5 backdrop-blur-sm"
  style="left: {position.x}px; top: {position.y}px; background: var(--surface-panel); box-shadow: var(--shadow-popover), 0 0 0 1px var(--border-subtle)"
  onkeydown={handleKeydown}
  role="dialog"
  tabindex="-1"
>
  <textarea
    bind:this={textareaEl}
    bind:value={body}
    placeholder="Add your annotation..."
    rows="3"
    class="w-full bg-surface-panel border-border-default/60 text-text-primary text-sm rounded-md p-2 resize-y min-h-[70px] focus:border-accent focus:ring-accent/20"
    style="box-shadow: var(--shadow-inset)"
  ></textarea>

  <input
    type="text"
    bind:value={labelsInput}
    placeholder="Labels (comma-separated)"
    class="w-full bg-surface-panel border-border-default/60 text-text-primary text-sm rounded-md p-2 focus:border-accent focus:ring-accent/20"
    style="box-shadow: var(--shadow-inset)"
  />

  <div class="flex justify-end gap-2">
    <Button variant="secondary" size="sm" onclick={onCancel}>Cancel</Button>
    <Button size="sm" onclick={handleSubmit}>
      Save
      <span class="flex items-center gap-1 text-xs opacity-60 font-mono">
        <Kbd class="px-1 py-0 text-[10px]">{submitShortcut[0]}</Kbd>
        <span>+</span>
        <Kbd class="px-1 py-0 text-[10px]">{submitShortcut[1]}</Kbd>
      </span>
    </Button>
  </div>
</div>
