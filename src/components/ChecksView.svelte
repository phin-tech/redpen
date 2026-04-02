<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { CheckRun, CheckRunsResult, GitHubPrSession } from "$lib/types";
  import ChecksJobList from "./checks/ChecksJobList.svelte";
  import ChecksTerminal from "./checks/ChecksTerminal.svelte";
  import ChecksErrorContext from "./checks/ChecksErrorContext.svelte";
  import ChecksMiniDiff from "./checks/ChecksMiniDiff.svelte";

  let {
    session,
  }: {
    session: GitHubPrSession;
  } = $props();

  let result = $state<CheckRunsResult | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedRun = $state<CheckRun | null>(null);

  // Cache log HTML by job id so switching jobs doesn't re-fetch
  let logCache = $state<Map<number, string>>(new Map());

  // The raw log text (pre-HTML) for file:line extraction — we strip HTML tags
  let rawLogCache = $state<Map<number, string>>(new Map());

  // Mini-diff overlay state
  let miniDiffFile = $state<string | null>(null);
  let miniDiffLine = $state<number>(0);

  const currentLogHtml = $derived(
    selectedRun ? logCache.get(selectedRun.id) ?? null : null
  );
  const currentRawLog = $derived(
    selectedRun ? rawLogCache.get(selectedRun.id) ?? null : null
  );

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

  function handleSelectRun(run: CheckRun) {
    selectedRun = run;
  }

  function handleLogsLoaded(html: string) {
    if (!selectedRun) return;
    const newCache = new Map(logCache);
    newCache.set(selectedRun.id, html);
    logCache = newCache;

    // Strip HTML tags to get raw text for file:line extraction
    const raw = html.replace(/<[^>]*>/g, "");
    const newRawCache = new Map(rawLogCache);
    newRawCache.set(selectedRun.id, raw);
    rawLogCache = newRawCache;
  }

  function handleFileLineClick(file: string, line: number) {
    miniDiffFile = file;
    miniDiffLine = line;
  }

  function closeMiniDiff() {
    miniDiffFile = null;
  }
</script>

<div class="checks-view">
  {#if loading}
    <div class="checks-center-status">Loading checks...</div>
  {:else if error}
    <div class="checks-center-status">
      <span class="error-text">{error}</span>
      <button class="retry-btn" onclick={loadChecks}>Retry</button>
    </div>
  {:else if result}
    <div class="checks-layout">
      <ChecksJobList
        {result}
        {selectedRun}
        onSelectRun={handleSelectRun}
      />

      {#if selectedRun}
        <ChecksTerminal
          run={selectedRun}
          repo={session.repo}
          logHtml={currentLogHtml}
          onLogsLoaded={handleLogsLoaded}
        />

        <ChecksErrorContext
          logText={currentRawLog}
          onFileLineClick={handleFileLineClick}
        />
      {:else}
        <div class="checks-empty-terminal">
          <span class="empty-text">Select a job to view logs</span>
        </div>
      {/if}
    </div>

    {#if miniDiffFile}
      <ChecksMiniDiff
        file={miniDiffFile}
        line={miniDiffLine}
        headSha={session.headSha}
        repoPath={session.worktreePath}
        onClose={closeMiniDiff}
      />
    {/if}
  {:else}
    <div class="checks-center-status">No check runs found</div>
  {/if}
</div>

<style>
  .checks-view {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
    position: relative;
  }
  .checks-center-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 8px;
    font-size: 12px;
  }
  .error-text {
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
  .checks-empty-terminal {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #000;
  }
  .empty-text {
    color: #555;
    font-size: 12px;
  }
</style>
