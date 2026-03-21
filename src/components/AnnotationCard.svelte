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

  function kindColor(kind: string, orphaned: boolean): string {
    if (orphaned) return "var(--accent-red)";
    switch (kind) {
      case "comment": return "var(--accent-blue)";
      case "lineNote": return "var(--accent-yellow)";
      case "label": return "var(--accent-purple)";
      default: return "var(--text-muted)";
    }
  }
</script>

<div
  class="annotation-card"
  class:selected={isSelected}
  class:orphaned={annotation.isOrphaned}
  onclick={onClick}
  ondblclick={onDoubleClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && onClick()}
>
  <div class="card-header">
    <span class="location" style="color: {kindColor(annotation.kind, annotation.isOrphaned)}">
      L{annotation.anchor.range.startLine}:{annotation.anchor.range.startColumn}
    </span>
    <span class="kind">{annotation.kind}</span>
    <button class="delete-btn" onclick={(e) => { e.stopPropagation(); onDelete(); }} title="Delete">×</button>
  </div>

  <div class="card-body">{annotation.body}</div>

  {#if annotation.labels.length > 0}
    <div class="labels">
      {#each annotation.labels as label}
        <span class="label-tag">{label}</span>
      {/each}
    </div>
  {/if}

  <div class="card-footer">
    <span class="author">{annotation.author}</span>
  </div>
</div>

<style>
  .annotation-card {
    padding: 8px 12px;
    border-left: 3px solid var(--border-color);
    margin: 4px 0;
    border-radius: 0 4px 4px 0;
    cursor: pointer;
    transition: background 0.1s;
  }

  .annotation-card:hover {
    background: var(--bg-highlight);
  }

  .annotation-card.selected {
    background: var(--bg-highlight);
    border-left-color: var(--accent-blue);
  }

  .annotation-card.orphaned {
    border-left-color: var(--accent-red);
    opacity: 0.7;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .location {
    font-size: 11px;
    font-weight: 600;
    font-family: monospace;
  }

  .kind {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .delete-btn {
    margin-left: auto;
    font-size: 16px;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity 0.1s;
  }

  .annotation-card:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--accent-red);
  }

  .card-body {
    font-size: 13px;
    line-height: 1.4;
    margin-bottom: 4px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .labels {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 4px;
  }

  .label-tag {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 8px;
    background: var(--bg-highlight);
    color: var(--text-subtle);
  }

  .card-footer {
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
