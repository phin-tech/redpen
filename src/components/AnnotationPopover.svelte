<script lang="ts">
  import type { AnnotationKind } from "$lib/types";

  let {
    onSubmit,
    onCancel,
    position = { x: 0, y: 0 },
    initialKind = "comment",
  }: {
    onSubmit: (body: string, labels: string[], kind: AnnotationKind) => void;
    onCancel: () => void;
    position: { x: number; y: number };
    initialKind?: AnnotationKind;
  } = $props();

  let body = $state("");
  let labelsInput = $state("");
  let kind: AnnotationKind = $state(initialKind);
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
    onSubmit(body.trim(), labels, kind);
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

<div
  class="popover"
  style="left: {position.x}px; top: {position.y}px"
  onkeydown={handleKeydown}
  role="dialog"
  tabindex="-1"
>
  <div class="kind-selector">
    <button class:active={kind === "comment"} onclick={() => (kind = "comment")}>Comment</button>
    <button class:active={kind === "lineNote"} onclick={() => (kind = "lineNote")}>Note</button>
    <button class:active={kind === "label"} onclick={() => (kind = "label")}>Label</button>
  </div>

  <textarea
    bind:this={textareaEl}
    bind:value={body}
    placeholder="Add your annotation..."
    rows="3"
  ></textarea>

  <input
    type="text"
    bind:value={labelsInput}
    placeholder="Labels (comma-separated)"
  />

  <div class="actions">
    <button class="cancel-btn" onclick={onCancel}>Cancel</button>
    <button class="submit-btn" onclick={handleSubmit}>Save (Cmd+Return)</button>
  </div>
</div>

<style>
  .popover {
    position: fixed;
    z-index: 100;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 12px;
    width: 320px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .kind-selector {
    display: flex;
    gap: 4px;
  }

  .kind-selector button {
    padding: 2px 8px;
    font-size: 11px;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .kind-selector button.active {
    color: var(--accent-blue);
    background: rgba(137, 180, 250, 0.1);
  }

  textarea {
    resize: vertical;
    min-height: 60px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .cancel-btn {
    padding: 4px 12px;
    font-size: 12px;
    color: var(--text-muted);
    border-radius: 4px;
  }

  .cancel-btn:hover {
    background: var(--bg-highlight);
  }

  .submit-btn {
    padding: 4px 12px;
    font-size: 12px;
    background: var(--accent-blue);
    color: var(--bg-primary);
    border-radius: 4px;
    font-weight: 600;
  }

  .submit-btn:hover {
    opacity: 0.9;
  }
</style>
