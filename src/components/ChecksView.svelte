<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { CheckRun, CheckRunsResult, GitHubPrSession } from "$lib/types";

  let {
    session,
  }: {
    session: GitHubPrSession;
  } = $props();

  let result = $state<CheckRunsResult | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedRun = $state<CheckRun | null>(null);

  async function loadChecks() {
    loading = true;
    error = null;
    try {
      result = await invoke<CheckRunsResult>("get_pr_check_runs", {
        repo: session.repo,
        headSha: session.headSha,
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (session.headSha) {
      loadChecks();
    }
  });

  function statusIcon(run: CheckRun): string {
    if (run.status !== "completed") return "pending";
    switch (run.conclusion) {
      case "success": return "pass";
      case "failure": case "timed_out": case "cancelled": return "fail";
      case "skipped": return "skip";
      default: return "neutral";
    }
  }

  function statusColor(run: CheckRun): string {
    const icon = statusIcon(run);
    switch (icon) {
      case "pass": return "var(--color-success)";
      case "fail": return "var(--color-danger)";
      case "pending": return "var(--color-warning)";
      default: return "var(--text-muted)";
    }
  }

  function formatDuration(run: CheckRun): string {
    if (!run.startedAt || !run.completedAt) return "";
    const start = new Date(run.startedAt).getTime();
    const end = new Date(run.completedAt).getTime();
    const seconds = Math.round((end - start) / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  }
</script>

<div class="checks-view">
  {#if loading}
    <div class="checks-status">Loading checks...</div>
  {:else if error}
    <div class="checks-status">
      <p class="error">{error}</p>
      <button class="retry-btn" onclick={loadChecks}>Retry</button>
    </div>
  {:else if result}
    <div class="checks-layout">
      <div class="checks-rail">
        <div class="checks-summary">
          <span class="summary-count pass">{result.passed}</span>
          <span class="summary-count fail">{result.failed}</span>
          <span class="summary-count pending">{result.pending}</span>
        </div>
        <div class="checks-list">
          {#each result.checkRuns as run}
            <button
              class="check-item"
              class:selected={selectedRun?.name === run.name}
              onclick={() => selectedRun = run}
            >
              <span class="check-status-dot" style:background={statusColor(run)}></span>
              <span class="check-name">{run.name}</span>
              {#if run.status === "completed"}
                <span class="check-duration">{formatDuration(run)}</span>
              {:else}
                <span class="check-in-progress">running</span>
              {/if}
            </button>
          {/each}
        </div>
      </div>
      <div class="checks-detail">
        {#if selectedRun}
          <div class="detail-header">
            <span class="detail-status-dot" style:background={statusColor(selectedRun)}></span>
            <h3>{selectedRun.name}</h3>
            {#if selectedRun.conclusion}
              <span class="detail-conclusion">{selectedRun.conclusion}</span>
            {:else}
              <span class="detail-conclusion pending">in progress</span>
            {/if}
          </div>
          <div class="detail-meta">
            {#if selectedRun.startedAt}
              <span>Started: {new Date(selectedRun.startedAt).toLocaleString()}</span>
            {/if}
            {#if selectedRun.completedAt}
              <span>Completed: {new Date(selectedRun.completedAt).toLocaleString()}</span>
            {/if}
            {#if selectedRun.startedAt && selectedRun.completedAt}
              <span>Duration: {formatDuration(selectedRun)}</span>
            {/if}
          </div>
          {#if selectedRun.detailsUrl}
            <a class="detail-link" href={selectedRun.detailsUrl} target="_blank" rel="noopener">
              View full logs on GitHub
            </a>
          {/if}
        {:else}
          <div class="checks-status">Select a check to view details</div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="checks-status">No check runs found</div>
  {/if}
</div>

<style>
  .checks-view {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
  }
  .checks-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 8px;
  }
  .checks-status .error {
    color: var(--color-danger);
  }
  .retry-btn {
    padding: 4px 12px;
    border-radius: 4px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 12px;
  }
  .checks-layout {
    display: flex;
    height: 100%;
    overflow: hidden;
  }
  .checks-rail {
    width: 280px;
    min-width: 280px;
    border-right: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    background: var(--surface-panel);
  }
  .checks-summary {
    display: flex;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-default);
    font-size: 13px;
    font-weight: 600;
  }
  .summary-count {
    font-family: var(--font-mono);
  }
  .summary-count.pass { color: var(--color-success); }
  .summary-count.pass::before { content: "\2713 "; }
  .summary-count.fail { color: var(--color-danger); }
  .summary-count.fail::before { content: "\2717 "; }
  .summary-count.pending { color: var(--color-warning); }
  .summary-count.pending::before { content: "\25CB "; }
  .checks-list {
    flex: 1;
    overflow-y: auto;
  }
  .check-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }
  .check-item:hover {
    background: var(--surface-highlight);
  }
  .check-item.selected {
    background: var(--surface-selection);
  }
  .check-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .check-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .check-duration {
    color: var(--text-muted);
    font-size: 11px;
    font-family: var(--font-mono);
  }
  .check-in-progress {
    color: var(--color-warning);
    font-size: 11px;
    font-style: italic;
  }
  .checks-detail {
    flex: 1;
    padding: 16px;
    overflow-y: auto;
    background: var(--surface-base);
  }
  .detail-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .detail-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .detail-status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .detail-conclusion {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--surface-raised);
    color: var(--text-secondary);
    text-transform: uppercase;
    font-weight: 600;
  }
  .detail-conclusion.pending {
    color: var(--color-warning);
  }
  .detail-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 12px;
  }
  .detail-link {
    font-size: 12px;
    color: var(--view-active);
    text-decoration: none;
  }
  .detail-link:hover {
    text-decoration: underline;
  }
</style>
