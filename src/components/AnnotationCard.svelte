<script lang="ts">
  import type { Annotation } from "$lib/types";

  import type { AnnotationKind } from "$lib/types";

  let {
    annotation,
    isReply = false,
    isSelected = false,
    onClick,
    onDoubleClick,
    onDelete,
  }: {
    annotation: Annotation;
    isReply?: boolean;
    isSelected?: boolean;
    onClick: () => void;
    onDoubleClick: () => void;
    onDelete: () => void;
  } = $props();

  const kindColorMap: Record<AnnotationKind, string> = {
    comment: "var(--kind-comment-border)",
    explanation: "var(--kind-explanation-border)",
    lineNote: "var(--kind-linenote-border)",
    label: "var(--kind-label-border)",
  };

  const kindBorderColor = $derived(
    annotation.isOrphaned ? undefined : kindColorMap[annotation.kind]
  );
</script>

<div
  class="group px-3 py-2.5 my-1 cursor-pointer transition-all duration-150 border-l-[3px] rounded-md
    {isReply ? 'ml-4' : ''}
    {isSelected ? 'bg-surface-selection annotation-card-selected' : 'hover:bg-surface-highlight/70 annotation-card'}
    {annotation.isOrphaned ? 'border-l-danger opacity-60' : ''}"
  style:border-left-color={kindBorderColor}
  onclick={onClick}
  ondblclick={onDoubleClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && onClick()}
>
  <div class="flex items-center gap-2 mb-1.5">
    {#if isReply}
      <span class="text-xs text-text-muted">↳ reply</span>
    {:else}
      <span class="text-xs text-text-secondary font-mono">
        L{annotation.anchor.range.startLine}
      </span>
    {/if}
    {#if annotation.kind !== "comment"}
      <span class="kind-badge" style:color={kindColorMap[annotation.kind]}>
        {annotation.kind === "explanation" ? "explanation" : annotation.kind === "lineNote" ? "note" : "label"}
      </span>
    {/if}
    {#if annotation.isOrphaned}
      <span class="text-xs text-danger font-semibold uppercase tracking-wide">orphaned</span>
    {/if}
    <button
      class="ml-auto text-base text-text-secondary opacity-0 group-hover:opacity-100 transition-opacity hover:text-danger leading-none"
      onclick={(e) => { e.stopPropagation(); onDelete(); }}
      title="Delete"
    >
      ×
    </button>
  </div>

  <div class="text-base leading-relaxed mb-1.5 whitespace-pre-wrap break-words text-text-primary">
    {annotation.body}
  </div>

  {#if annotation.labels.length > 0}
    <div class="flex flex-wrap gap-1 mb-1.5">
      {#each annotation.labels as label}
        <span class="text-xs px-2 py-0.5 rounded-full bg-surface-panel/80 text-text-primary border border-border-default/50" style="box-shadow: var(--shadow-xs)">{label}</span>
      {/each}
    </div>
  {/if}

  <div class="text-xs text-text-secondary">{annotation.author}</div>
</div>

<style>
  .annotation-card {
    box-shadow: var(--shadow-card);
    background: var(--gradient-panel), var(--surface-panel);
  }
  .annotation-card:hover {
    box-shadow: var(--shadow-card-hover);
    transform: translateY(-0.5px);
  }
  .annotation-card-selected {
    box-shadow: var(--shadow-card-hover), 0 0 0 1px rgba(227, 154, 45, 0.15);
    background: linear-gradient(180deg, rgba(227, 154, 45, 0.04) 0%, transparent 100%), var(--surface-selection);
  }
  .kind-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
