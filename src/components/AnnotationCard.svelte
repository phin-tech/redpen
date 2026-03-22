<script lang="ts">
  import type { Annotation } from "$lib/types";

  let {
    annotation,
    isSelected = false,
    onClick,
    onDoubleClick,
    onDelete,
  }: {
    annotation: Annotation;
    isSelected?: boolean;
    onClick: () => void;
    onDoubleClick: () => void;
    onDelete: () => void;
  } = $props();
</script>

<div
  class="group px-3 py-2.5 my-0.5 cursor-pointer transition-all border-l-[3px] rounded-r-md
    {isSelected ? 'bg-[var(--bg-selection)] border-l-amber-400' : 'border-l-graphite-700 hover:bg-graphite-800'}
    {annotation.isOrphaned ? 'border-l-red-400 opacity-60' : ''}"
  onclick={onClick}
  ondblclick={onDoubleClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && onClick()}
>
  <div class="flex items-center gap-2 mb-1.5">
    <span class="text-[11px] text-graphite-400 font-mono">
      L{annotation.anchor.range.startLine}
    </span>
    {#if annotation.isOrphaned}
      <span class="text-[10px] text-red-400 font-semibold uppercase tracking-wide">orphaned</span>
    {/if}
    <button
      class="ml-auto text-base text-graphite-400 opacity-0 group-hover:opacity-100 transition-opacity hover:text-red-400 leading-none"
      onclick={(e) => { e.stopPropagation(); onDelete(); }}
      title="Delete"
    >
      ×
    </button>
  </div>

  <div class="text-[13px] leading-relaxed mb-1.5 whitespace-pre-wrap break-words text-graphite-50">
    {annotation.body}
  </div>

  {#if annotation.labels.length > 0}
    <div class="flex flex-wrap gap-1 mb-1.5">
      {#each annotation.labels as label}
        <span class="text-[10px] px-2 py-0.5 rounded-full bg-graphite-900 text-graphite-200 border border-graphite-700">{label}</span>
      {/each}
    </div>
  {/if}

  <div class="text-[11px] text-graphite-400">{annotation.author}</div>
</div>
