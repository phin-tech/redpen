<script lang="ts">
  import { getReviewSession, clearReviewSession } from "$lib/stores/review.svelte";
  import IconButton from "./ui/IconButton.svelte";

  let {
    onFileSelect,
    selectedPath,
  }: {
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
  } = $props();

  const session = getReviewSession();
  let collapsed = $state(false);

  function fileName(path: string): string {
    return path.split("/").pop() ?? path;
  }

  function relativePath(path: string): string {
    // Show last 2-3 path segments for context
    const parts = path.split("/");
    return parts.length > 2 ? parts.slice(-3).join("/") : parts.join("/");
  }
</script>

{#if session.active && session.files.length > 0}
  <div class="border-b border-border-default">
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-border-default/60">
      <button
        type="button"
        class="flex items-center gap-1 text-xs font-semibold uppercase text-accent tracking-wider select-none cursor-pointer hover:text-accent-hover transition-colors"
        onclick={() => (collapsed = !collapsed)}
      >
        <span class="text-xs">{collapsed ? "▸" : "▾"}</span>
        Review
        <span class="font-normal text-text-muted">({session.files.length})</span>
      </button>
      <div class="flex items-center gap-1.5">
        <IconButton label="Dismiss review" onclick={clearReviewSession}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </IconButton>
      </div>
    </div>
    {#if !collapsed}
      <div class="py-0.5">
        {#each session.files as filePath (filePath)}
          <button
            type="button"
            class="flex w-full items-center gap-1.5 px-3 py-1 text-xs select-none cursor-pointer transition-colors text-left {selectedPath === filePath ? 'bg-accent/15 text-accent' : 'text-text-secondary hover:bg-surface-highlight hover:text-text-primary'}"
            onclick={() => onFileSelect(filePath)}
            title={filePath}
          >
            <span class="text-text-muted">&#x2022;</span>
            <span class="truncate">{relativePath(filePath)}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}
