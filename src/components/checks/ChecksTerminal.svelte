<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { CheckRun } from "$lib/types";

  let {
    run,
    repo,
    logHtml,
    onLogsLoaded,
  }: {
    run: CheckRun;
    repo: string;
    logHtml: string | null;
    onLogsLoaded: (html: string) => void;
  } = $props();

  let loading = $state(false);
  let error = $state<string | null>(null);
  let scrollContainer: HTMLDivElement | undefined = $state();

  function formatDuration(r: CheckRun): string {
    if (!r.startedAt || !r.completedAt) return "";
    const start = new Date(r.startedAt).getTime();
    const end = new Date(r.completedAt).getTime();
    const seconds = Math.round((end - start) / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  }

  function conclusionClass(r: CheckRun): string {
    switch (r.conclusion) {
      case "success": return "pass";
      case "failure": case "timed_out": case "cancelled": return "fail";
      default: return "neutral";
    }
  }

  function statusColor(r: CheckRun): string {
    if (r.status !== "completed") return "var(--color-warning)";
    switch (r.conclusion) {
      case "success": return "var(--color-success)";
      case "failure": case "timed_out": case "cancelled": return "var(--color-danger)";
      default: return "var(--text-muted)";
    }
  }

  async function fetchLogs() {
    if (logHtml !== null) return; // Already loaded
    loading = true;
    error = null;
    try {
      const html = await invoke<string>("get_job_logs", {
        repo,
        jobId: run.id,
      });
      onLogsLoaded(html);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Fetch logs when run changes
    if (run.id) {
      fetchLogs();
    }
  });

  // Scroll to first error marker after logs load
  $effect(() => {
    if (logHtml && scrollContainer) {
      requestAnimationFrame(() => {
        const firstError = scrollContainer?.querySelector(".error-line");
        if (firstError) {
          firstError.scrollIntoView({ block: "center" });
        }
      });
    }
  });
</script>

<div class="terminal">
  <div class="terminal-header">
    <span class="header-dot" style:background={statusColor(run)}></span>
    <span class="header-name">{run.name}</span>
    <span class="header-spacer"></span>
    {#if run.startedAt && run.completedAt}
      <span class="header-duration">{formatDuration(run)}</span>
    {/if}
    {#if run.conclusion}
      <span class="header-badge {conclusionClass(run)}">{run.conclusion}</span>
    {:else if run.status !== "completed"}
      <span class="header-badge pending">in progress</span>
    {/if}
    {#if run.detailsUrl}
      <a class="header-link" href={run.detailsUrl} target="_blank" rel="noopener">GitHub</a>
    {/if}
  </div>
  <div class="terminal-body" bind:this={scrollContainer}>
    {#if loading}
      <div class="terminal-status">Loading logs...</div>
    {:else if error}
      <div class="terminal-status">
        <span class="terminal-error">{error}</span>
        <button class="retry-btn" onclick={fetchLogs}>Retry</button>
      </div>
    {:else if logHtml}
      <pre class="log-output">{@html logHtml}</pre>
    {:else}
      <div class="terminal-status">No logs available</div>
    {/if}
  </div>
</div>

<style>
  .terminal {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #000;
    min-width: 0;
  }
  .terminal-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: #0a0a0a;
    border-bottom: 1px solid #222;
    flex-shrink: 0;
  }
  .header-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .header-name {
    font-size: 12px;
    font-weight: 600;
    color: #ccc;
  }
  .header-spacer { flex: 1; }
  .header-duration {
    font-size: 10px;
    color: #666;
    font-family: var(--font-mono);
  }
  .header-badge {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    font-weight: 600;
  }
  .header-badge.fail {
    background: color-mix(in srgb, var(--color-danger) 20%, transparent);
    color: var(--color-danger);
  }
  .header-badge.pass {
    background: color-mix(in srgb, var(--color-success) 20%, transparent);
    color: var(--color-success);
  }
  .header-badge.pending {
    color: var(--color-warning);
    background: color-mix(in srgb, var(--color-warning) 20%, transparent);
  }
  .header-badge.neutral {
    background: var(--surface-raised);
    color: var(--text-muted);
  }
  .header-link {
    font-size: 10px;
    color: var(--text-muted);
    text-decoration: none;
    padding: 2px 6px;
    border-radius: 3px;
    background: #1a1a1a;
  }
  .header-link:hover {
    color: var(--text-primary);
    background: #222;
  }
  .terminal-body {
    flex: 1;
    overflow: auto;
    padding: 12px 16px;
  }
  .terminal-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #555;
    gap: 8px;
    font-size: 12px;
  }
  .terminal-error {
    color: var(--color-danger);
  }
  .retry-btn {
    padding: 4px 12px;
    border-radius: 4px;
    background: #1a1a1a;
    border: 1px solid #333;
    color: #ccc;
    cursor: pointer;
    font-size: 11px;
  }
  .retry-btn:hover {
    background: #222;
  }
  .log-output {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.6;
    color: #ccc;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
