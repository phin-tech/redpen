<script lang="ts">
  import type { AnnotationFilter } from "$lib/types";
  import { getAnnotationsState, setFilter } from "$lib/stores/annotations.svelte";

  const annotationsState = getAnnotationsState();

  const filters: { value: AnnotationFilter; label: string }[] = [
    { value: "all", label: "All" },
    { value: "comment", label: "Comments" },
    { value: "lineNote", label: "Notes" },
    { value: "label", label: "Labels" },
  ];
</script>

<div class="filter-bar">
  {#each filters as f}
    <button
      class="filter-btn"
      class:active={annotationsState.filter === f.value}
      onclick={() => setFilter(f.value)}
    >
      {f.label}
    </button>
  {/each}
</div>

<style>
  .filter-bar {
    display: flex;
    gap: 2px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-color);
  }

  .filter-btn {
    padding: 2px 8px;
    font-size: 11px;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .filter-btn:hover {
    color: var(--text-primary);
    background: var(--bg-highlight);
  }

  .filter-btn.active {
    color: var(--accent-blue);
    background: rgba(137, 180, 250, 0.1);
  }
</style>
