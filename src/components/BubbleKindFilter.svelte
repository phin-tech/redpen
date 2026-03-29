<script lang="ts">
  import { getBubbleKindFilter, toggleBubbleKind, ALL_KINDS, type BubbleAnnotationKind } from "$lib/stores/annotations.svelte";

  let open = $state(false);
  let buttonEl: HTMLButtonElement;

  const kindLabels: Record<BubbleAnnotationKind, string> = {
    comment: "Comments",
    explanation: "Explanations",
    lineNote: "Notes",
    label: "Labels",
  };

  const kindColors: Record<BubbleAnnotationKind, string> = {
    comment: "var(--kind-comment)",
    explanation: "var(--kind-explanation)",
    lineNote: "var(--kind-linenote)",
    label: "var(--kind-label)",
  };

  const filter = $derived(getBubbleKindFilter());
  const allEnabled = $derived(filter.size === ALL_KINDS.length);

  function handleClickOutside(e: MouseEvent) {
    if (open && buttonEl && !buttonEl.closest(".bubble-kind-filter")?.contains(e.target as Node)) {
      open = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) {
      open = false;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeydown} />

<div class="bubble-kind-filter">
  <button
    bind:this={buttonEl}
    class="filter-btn"
    class:has-filter={!allEnabled}
    onclick={(e) => { e.stopPropagation(); open = !open; }}
    title="Filter inline annotations by kind"
  >
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"></polygon>
    </svg>
    {#if !allEnabled}
      <span class="filter-count">{filter.size}</span>
    {/if}
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="filter-dropdown" onclick={(e) => e.stopPropagation()}>
      {#each ALL_KINDS as kind}
        <label class="filter-option">
          <input
            type="checkbox"
            checked={filter.has(kind)}
            onchange={() => toggleBubbleKind(kind)}
          />
          <span class="kind-dot" style:background={kindColors[kind]}></span>
          <span class="kind-label">{kindLabels[kind]}</span>
        </label>
      {/each}
    </div>
  {/if}
</div>

<style>
  .bubble-kind-filter {
    position: relative;
  }

  .filter-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 30px;
    padding: 4px 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
  }

  .filter-btn:hover {
    color: var(--text-secondary);
    background: var(--surface-raised);
    border-color: var(--border-default);
  }

  .filter-btn.has-filter {
    color: var(--view-active);
    background: var(--view-active-subtle);
    border-color: var(--view-active-border);
  }

  .filter-count {
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .filter-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 50;
    min-width: 160px;
    padding: 4px;
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .filter-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    color: var(--text-secondary);
    transition: background 0.1s;
  }

  .filter-option:hover {
    background: var(--surface-raised);
  }

  .filter-option input[type="checkbox"] {
    width: 13px;
    height: 13px;
    margin: 0;
    accent-color: var(--view-active);
    cursor: pointer;
  }

  .kind-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .kind-label {
    flex: 1;
  }
</style>
