<script lang="ts">
  import type { CheckRun, CheckRunsResult } from "$lib/types";

  let {
    result,
    selectedRun,
    onSelectRun,
  }: {
    result: CheckRunsResult;
    selectedRun: CheckRun | null;
    onSelectRun: (run: CheckRun) => void;
  } = $props();

  let filter = $state<"all" | "failed" | "passed">("all");

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

  const filteredRuns = $derived(() => {
    const runs = result.checkRuns;
    if (filter === "all") return runs;
    if (filter === "failed") return runs.filter(r => statusIcon(r) === "fail");
    return runs.filter(r => statusIcon(r) === "pass");
  });

  // Sort: failed first, then pending, then passed
  const sortedRuns = $derived(() => {
    const order = { fail: 0, pending: 1, pass: 2, skip: 3, neutral: 3 };
    return [...filteredRuns()].sort((a, b) => {
      return (order[statusIcon(a) as keyof typeof order] ?? 3) -
             (order[statusIcon(b) as keyof typeof order] ?? 3);
    });
  });
</script>

<div class="jobs-list">
  <div class="jobs-header">
    <div class="summary-counts">
      <span class="count pass">&#10003; {result.passed}</span>
      <span class="count fail">&#10007; {result.failed}</span>
      <span class="count pending">
        {#if result.pending > 0}
          <span class="pulse-dot"></span>
        {:else}
          &#9675;
        {/if}
        {result.pending}
      </span>
    </div>
    <div class="filter-tabs">
      <button class="filter-tab" class:active={filter === "all"} onclick={() => filter = "all"}>All</button>
      <button class="filter-tab" class:active={filter === "failed"} onclick={() => filter = "failed"}>Failed</button>
      <button class="filter-tab" class:active={filter === "passed"} onclick={() => filter = "passed"}>Passed</button>
    </div>
  </div>
  <div class="jobs-scroll">
    {#each sortedRuns() as run (run.id)}
      <button
        class="job-item"
        class:selected={selectedRun?.id === run.id}
        onclick={() => onSelectRun(run)}
      >
        {#if run.status !== "completed"}
          <span class="pulse-dot"></span>
        {:else}
          <span class="status-dot" style:background={statusColor(run)}></span>
        {/if}
        <span class="job-name">{run.name}</span>
        {#if run.status === "completed"}
          <span class="job-duration">{formatDuration(run)}</span>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .jobs-list {
    width: 240px;
    min-width: 240px;
    display: flex;
    flex-direction: column;
    background: var(--surface-panel);
    border-right: 1px solid var(--border-default);
    height: 100%;
  }
  .jobs-header {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-default);
  }
  .summary-counts {
    display: flex;
    gap: 10px;
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
  }
  .count.pass { color: var(--color-success); }
  .count.fail { color: var(--color-danger); }
  .count.pending { color: var(--color-warning); display: flex; align-items: center; gap: 4px; }
  .filter-tabs {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }
  .filter-tab {
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 3px;
    border: none;
    background: var(--surface-raised);
    color: var(--text-muted);
    cursor: pointer;
  }
  .filter-tab:hover { color: var(--text-primary); }
  .filter-tab.active {
    background: var(--surface-selection);
    color: var(--text-primary);
  }
  .jobs-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .job-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 14px;
    border: none;
    border-left: 2px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    text-align: left;
  }
  .job-item:hover {
    background: var(--surface-highlight);
  }
  .job-item.selected {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-left-color: var(--color-danger);
    color: var(--text-primary);
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--color-warning);
    animation: pulse 1.5s ease-in-out infinite;
  }
  .job-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .job-duration {
    color: var(--text-muted);
    font-size: 10px;
    font-family: var(--font-mono);
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>
