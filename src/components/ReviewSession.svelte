<script lang="ts">
  import { getReviewSession, clearReviewSession } from "$lib/stores/review.svelte";
  import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
  import { getGitStatusForFile } from "$lib/stores/workspace.svelte";
  import IconButton from "./ui/IconButton.svelte";

  let {
    onFileSelect,
    selectedPath,
  }: {
    onFileSelect: (path: string) => void;
    selectedPath: string | null;
  } = $props();

  const session = getReviewSession();
  const githubReview = getGitHubReviewState();
  let collapsed = $state(false);
  let reviewFiles = $derived.by(() => {
    if (githubReview.activeSession) {
      return githubReview.activeSession.changedFiles.map(
        (relativePath) => `${githubReview.activeSession!.worktreePath}/${relativePath}`
      );
    }
    return session.files;
  });
  let reviewLabel = $derived(
    githubReview.activeSession ? "PR files to review" : "Files to review"
  );

  function relativePath(path: string): string {
    // Show last 2-3 path segments for context
    const parts = path.split("/");
    return parts.length > 2 ? parts.slice(-3).join("/") : parts.join("/");
  }

  function statusForFile(path: string): string | null {
    if (githubReview.activeSession) return "M";
    return getGitStatusForFile(path)?.status ?? null;
  }

  function gitStatusClass(status: string): string {
    switch (status) {
      case "M": return "text-warning";
      case "A": return "text-success";
      case "?": return "text-accent-teal";
      case "D": return "text-danger";
      case "R": return "text-accent-purple";
      default: return "text-text-secondary";
    }
  }

  function reviewItemClass(filePath: string): string {
    return selectedPath === filePath
      ? "bg-accent/15 text-accent"
      : "text-text-secondary hover:bg-surface-highlight hover:text-text-primary";
  }
</script>

{#if reviewFiles.length > 0}
  <div class="border-b border-border-default">
    <div class="flex items-center justify-between px-3 py-1.5 border-b border-border-default/60">
      <button
        type="button"
        class="flex items-center gap-1 text-xs font-semibold uppercase text-accent tracking-wider select-none cursor-pointer hover:text-accent-hover transition-colors"
        onclick={() => (collapsed = !collapsed)}
      >
        <span class="text-xs">{collapsed ? "▸" : "▾"}</span>
        {reviewLabel}
        <span class="font-normal text-text-muted">({reviewFiles.length})</span>
      </button>
      <div class="flex items-center gap-1.5">
        {#if session.active && !githubReview.activeSession}
          <IconButton label="Dismiss review" onclick={clearReviewSession}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </IconButton>
        {/if}
      </div>
    </div>
    {#if !collapsed}
      <div class="py-0.5">
        {#each reviewFiles as filePath (filePath)}
          {@const status = statusForFile(filePath)}
          <button
            type="button"
            class={`flex w-full items-center gap-1.5 px-3 py-1 text-xs select-none cursor-pointer transition-colors text-left ${reviewItemClass(filePath)}`}
            onclick={() => onFileSelect(filePath)}
            title={filePath}
          >
            <span class="text-text-muted">&#x2022;</span>
            <span class="truncate">{relativePath(filePath)}</span>
            {#if status}
              <span class="ml-auto text-xs font-semibold font-mono {gitStatusClass(status)}">{status}</span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}
