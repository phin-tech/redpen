<script lang="ts">
  import {
    cleanupStaleReviewSessions,
    getReviewHistory,
    getSettings,
    resumeReviewSession,
  } from "$lib/tauri";
  import type { AppSettings, ReviewHistory } from "$lib/types";
  import { onMount } from "svelte";
  import Button from "./ui/Button.svelte";
  import {
    activateGitHubReviewSession,
    getGitHubReviewState,
    loadGitHubReviewQueue,
    openGitHubPullRequest,
  } from "$lib/stores/githubReview.svelte";
  import { activateReviewSession } from "$lib/stores/review.svelte";
  import { loadAnnotations } from "$lib/stores/annotations.svelte";
  import { openFile } from "$lib/stores/editor.svelte";
  import { getWorkspace, replaceRootFolders } from "$lib/stores/workspace.svelte";

  let { onOpenFolder }: { onOpenFolder: () => Promise<void> } = $props();

  const githubReview = getGitHubReviewState();
  const workspace = getWorkspace();

  let manualPrRef = $state("");
  let manualLocalPath = $state("");
  let settings = $state<AppSettings | null>(null);
  let pendingAutoCheckout = $state<{
    prRef: string;
    repo: string;
    checkoutPath: string;
  } | null>(null);
  let suggestedLocalPath = $derived(workspace.rootFolders[0] ?? "");
  let history = $state<ReviewHistory | null>(null);
  let historyError = $state<string | null>(null);
  let cleaningHistory = $state(false);

  let hasContent = $derived(
    history != null && (
      history.activeSession != null ||
      history.recentPullRequests.length > 0 ||
      history.recentFiles.length > 0
    )
  );

  let allSessions = $derived(buildSessionList());

  function buildSessionList() {
    if (!history) return [];
    const items: Array<{ item: typeof history.recentPullRequests[0]; label: string; isActive: boolean }> = [];
    if (history.activeSession) {
      items.push({ item: history.activeSession, label: history.activeSession.kind === "github_pr" ? "Pull request" : "Local review", isActive: true });
    }
    for (const pr of history.recentPullRequests) {
      items.push({ item: pr, label: "Pull request", isActive: false });
    }
    for (const f of history.recentFiles) {
      items.push({ item: f, label: "Local review", isActive: false });
    }
    return items;
  }

  function relativeTime(isoDate: string): string {
    const now = Date.now();
    const then = new Date(isoDate).getTime();
    const diffMs = now - then;
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return "just now";
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDays = Math.floor(diffHr / 24);
    if (diffDays === 1) return "yesterday";
    if (diffDays < 30) return `${diffDays}d ago`;
    return new Date(isoDate).toLocaleDateString();
  }

  function pluralize(count: number, singular: string): string {
    return `${count} ${singular}${count === 1 ? '' : 's'}`;
  }

  onMount(() => {
    void initialize();
    const poll = window.setInterval(() => {
      void loadGitHubReviewQueue();
      void refreshHistory();
    }, 30000);

    return () => {
      window.clearInterval(poll);
    };
  });

  async function openManualPr() {
    const prRef = manualPrRef.trim();
    if (!prRef) return;
    const localPathOverride = manualLocalPath.trim() || suggestedLocalPath || undefined;

    if (!localPathOverride) {
      const repo = parseRepoFromPrRef(prRef);
      const trackedRepo = settings?.trackedGithubRepos?.find((item) =>
        item.repo.toLowerCase() === repo?.toLowerCase(),
      );
      if (repo && !trackedRepo && settings?.defaultCheckoutRoot) {
        pendingAutoCheckout = {
          prRef,
          repo,
          checkoutPath: `${settings.defaultCheckoutRoot.replace(/\/+$/, "")}/${repo}`,
        };
        return;
      }
    }

    await performOpen(prRef, localPathOverride);
  }

  async function initialize() {
    try {
      settings = await getSettings();
    } catch {
      settings = null;
    }
    manualLocalPath = suggestedLocalPath;
    await loadGitHubReviewQueue();
    await refreshHistory();
  }

  async function confirmAutoCheckout() {
    if (!pendingAutoCheckout) return;
    const { prRef } = pendingAutoCheckout;
    pendingAutoCheckout = null;
    await performOpen(prRef);
  }

  function cancelAutoCheckout() {
    pendingAutoCheckout = null;
  }

  async function performOpen(prRef: string, localPathOverride?: string) {
    await openGitHubPullRequest(prRef, localPathOverride);
    await refreshHistory();
  }

  function parseRepoFromPrRef(prRef: string): string | null {
    if (prRef.startsWith("https://github.com/")) {
      const parts = prRef.replace("https://github.com/", "").split("/");
      if (parts.length >= 4 && parts[2] === "pull") {
        return `${parts[0]}/${parts[1]}`;
      }
    }

    const hashIndex = prRef.lastIndexOf("#");
    if (hashIndex > 0) {
      return prRef.slice(0, hashIndex).trim();
    }

    return null;
  }

  async function refreshHistory() {
    try {
      history = await getReviewHistory();
      historyError = null;
    } catch (error) {
      historyError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleResume(sessionId: string) {
    const session = await resumeReviewSession(sessionId);
    if (session.kind === "github_pr" && session.githubSession) {
      await activateGitHubReviewSession(session.githubSession, {
        refreshQueue: true,
        replaceRoots: true,
      });
      await refreshHistory();
      return;
    }

    if (session.projectRoot) {
      await replaceRootFolders([session.projectRoot]);
    }
    if (session.files.length > 0) {
      activateReviewSession(session.sessionId, session.files);
      await openFile(session.files[0]);
      await loadAnnotations(session.files[0]);
    }
    await refreshHistory();
  }

  async function handleCleanup() {
    cleaningHistory = true;
    try {
      await cleanupStaleReviewSessions();
      await refreshHistory();
    } finally {
      cleaningHistory = false;
    }
  }
</script>

<div class="launch-screen">
  <!-- Hero -->
  <div class="launch-hero">
    <div class="launch-hero-label">Start a review</div>
    <div class="launch-hero-input-wrapper">
      <span class="launch-hero-search-icon">&#x2315;</span>
      <input
        bind:value={manualPrRef}
        placeholder="Paste a GitHub PR URL or owner/repo#123"
        class="launch-hero-input"
        onkeydown={(e) => { if (e.key === 'Enter') void openManualPr(); }}
      />
    </div>
    <div class="launch-hero-hint">or drag a folder anywhere to start a local review</div>
  </div>

  {#if githubReview.actionError}
    <div class="launch-error">{githubReview.actionError}</div>
  {/if}

  {#if hasContent}
    <div class="launch-columns">
      <div class="launch-col">
        <div class="launch-col-label">Local Review</div>
        <button class="launch-open-folder" onclick={() => void onOpenFolder()}>
          <span class="launch-open-folder-icon">&#x1F4C1;</span> Open Folder&hellip;
        </button>
      </div>
      <div class="launch-divider"></div>
      <div class="launch-col">
        <div class="launch-col-label-row">
          <div class="launch-col-label">Recent Sessions</div>
          <div class="launch-col-actions">
            <button class="launch-ghost-btn launch-ghost-btn-small" onclick={() => void refreshHistory()}>Refresh</button>
            {#if history?.staleSessions && history.staleSessions.length > 0}
              <button class="launch-ghost-btn launch-ghost-btn-small" onclick={() => void handleCleanup()} disabled={cleaningHistory}>
                {cleaningHistory ? "Cleaning..." : "Clean stale"}
              </button>
            {/if}
          </div>
        </div>
        {#if historyError}
          <div class="launch-session-empty">{historyError}</div>
        {:else if !history}
          <div class="launch-session-empty">Loading...</div>
        {:else}
          <div class="launch-session-list">
            {#each allSessions as { item, label, isActive } (item.id)}
              <div class="launch-session-card" class:launch-session-card-active={isActive}>
                <div class="launch-session-card-top">
                  <span class="launch-session-card-kind">{label}</span>
                  {#if item.verdict}
                    <span class="launch-session-card-verdict">{item.verdict}</span>
                  {/if}
                  <span class="launch-session-card-time">{relativeTime(item.updatedAt)}</span>
                </div>
                <div class="launch-session-card-title">{item.title}</div>
                <div class="launch-session-card-meta">
                  <span>{item.subtitle}</span>
                  <span>{pluralize(item.fileCount, 'file')}</span>
                </div>
                <button class="launch-ghost-btn" onclick={() => void handleResume(item.id)}>
                  {isActive ? "Resume" : "Open"}
                </button>
              </div>
            {/each}

            {#if history.staleSessions.length > 0}
              {#each history.staleSessions as item (item.id)}
                <div class="launch-session-card launch-session-card-stale">
                  <div class="launch-session-card-top">
                    <span class="launch-session-card-kind">{item.kind === "github_pr" ? "Pull request" : "Local review"}</span>
                    <span class="launch-session-card-time">stale</span>
                  </div>
                  <div class="launch-session-card-title">{item.title}</div>
                  <div class="launch-session-card-meta">
                    <span>{item.subtitle}</span>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <!-- Empty state: tight cluster -->
    <div class="launch-empty">
      <button class="launch-open-folder" onclick={() => void onOpenFolder()}>
        <span class="launch-open-folder-icon">&#x1F4C1;</span> Open Folder&hellip;
      </button>
    </div>
  {/if}

  <!-- Review queue (only if items exist) -->
  {#if githubReview.queue.length > 0}
    <div class="launch-queue">
      <div class="launch-col-label">Review Queue <span class="launch-queue-count">{githubReview.queue.length}</span></div>
      <div class="launch-queue-list">
        {#each githubReview.queue as item (`${item.repo}#${item.number}`)}
          <button
            class="launch-queue-item"
            onclick={() => void openGitHubPullRequest(`${item.repo}#${item.number}`, item.localPath)}
          >
            <div class="launch-queue-item-main">
              <div class="launch-queue-item-topline">
                <span class="launch-queue-item-repo">{item.repo}</span>
                <span class="launch-queue-item-number">#{item.number}</span>
              </div>
              <div class="launch-queue-item-title">{item.title}</div>
              <div class="launch-queue-item-meta">
                <span>{item.author}</span>
                <span>{item.baseRef} &larr; {item.headRef}</span>
              </div>
            </div>
            <span class="launch-ghost-btn">Open</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="launch-tip">
    Tip: Review PRs by pasting a GitHub URL above, or open a local folder to review agent changes
  </div>
</div>

{#if pendingAutoCheckout}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="launch-modal-backdrop" onclick={cancelAutoCheckout}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="launch-modal" onclick={(event) => event.stopPropagation()}>
      <div class="launch-modal-kicker">New Checkout</div>
      <h3>Clone repo for this review?</h3>
      <p>
        <strong>{pendingAutoCheckout.repo}</strong> is not tracked yet. Red Pen will clone it to
        <code>{pendingAutoCheckout.checkoutPath}</code> and then open the pull request.
      </p>
      <div class="launch-modal-actions">
        <Button variant="secondary" size="sm" onclick={cancelAutoCheckout}>Cancel</Button>
        <Button size="sm" onclick={() => void confirmAutoCheckout()}>Clone and Open</Button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Launch Screen Shell ── */
  .launch-screen {
    height: 100%;
    overflow: auto;
    background: var(--surface-base);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* ── Hero ── */
  .launch-hero {
    padding: 60px 40px 32px;
    text-align: center;
    width: 100%;
  }
  .launch-hero-label {
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 500;
    margin-bottom: 12px;
    letter-spacing: 0.02em;
  }
  .launch-hero-input-wrapper {
    max-width: 480px;
    margin: 0 auto;
    position: relative;
  }
  .launch-hero-search-icon {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-ghost);
    font-size: 16px;
    pointer-events: none;
  }
  .launch-hero-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 10px;
    padding: 12px 16px 12px 40px;
    font-size: 14px;
    color: var(--text-primary);
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .launch-hero-input::placeholder {
    color: var(--text-ghost);
  }
  .launch-hero-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px rgba(217, 177, 95, 0.2);
  }
  .launch-hero-hint {
    color: var(--text-ghost);
    font-size: 12px;
    margin-top: 10px;
  }

  /* ── Error banner ── */
  .launch-error {
    max-width: 480px;
    width: 100%;
    padding: 10px 14px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--danger) 10%, var(--surface-panel));
    color: var(--danger);
    font-size: 13px;
    margin-bottom: 8px;
  }

  /* ── Two-column layout ── */
  .launch-columns {
    display: flex;
    gap: 0;
    padding: 0 40px 24px;
    max-width: 720px;
    width: 100%;
    box-sizing: border-box;
  }
  .launch-col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .launch-col:first-child {
    padding-right: 24px;
  }
  .launch-col:last-child {
    padding-left: 24px;
  }
  .launch-divider {
    width: 1px;
    background: rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
  }
  .launch-col-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .launch-col-label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .launch-col-actions {
    display: flex;
    gap: 6px;
  }

  /* ── Open Folder button ── */
  .launch-open-folder {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: 10px 16px;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.12s;
    width: fit-content;
  }
  .launch-open-folder:hover {
    background: var(--surface-highlight);
    color: var(--text-primary);
  }
  .launch-open-folder-icon {
    font-size: 15px;
  }

  /* ── Empty state ── */
  .launch-empty {
    padding: 0 40px 32px;
    display: flex;
    justify-content: center;
  }

  /* ── Session cards ── */
  .launch-session-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .launch-session-empty {
    color: var(--text-muted);
    font-size: 13px;
  }
  .launch-session-card {
    background: var(--surface-panel);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .launch-session-card-active {
    box-shadow: inset 0 0 0 1px rgba(217, 177, 95, 0.18);
  }
  .launch-session-card-stale {
    opacity: 0.55;
  }
  .launch-session-card-top {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }
  .launch-session-card-kind {
    color: var(--accent);
    font-weight: 600;
  }
  .launch-session-card-verdict {
    color: var(--text-muted);
    font-style: italic;
  }
  .launch-session-card-time {
    color: var(--text-ghost);
    margin-left: auto;
  }
  .launch-session-card-title {
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .launch-session-card-meta {
    display: flex;
    gap: 10px;
    font-size: 11px;
    color: var(--text-ghost);
  }

  /* ── Ghost button ── */
  .launch-ghost-btn {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border-default);
    background: transparent;
    border-radius: 6px;
    padding: 4px 10px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
    width: fit-content;
    margin-top: 2px;
  }
  .launch-ghost-btn:hover {
    background: var(--surface-highlight);
    color: var(--text-primary);
  }
  .launch-ghost-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .launch-ghost-btn-small {
    font-size: 11px;
    padding: 2px 8px;
    border: none;
    color: var(--text-ghost);
  }

  /* ── Review Queue ── */
  .launch-queue {
    max-width: 720px;
    width: 100%;
    padding: 0 40px 24px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .launch-queue-count {
    font-weight: 400;
    color: var(--text-ghost);
  }
  .launch-queue-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .launch-queue-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--surface-panel);
    border-radius: 8px;
    border: none;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s;
    color: inherit;
  }
  .launch-queue-item:hover {
    background: var(--surface-highlight);
  }
  .launch-queue-item-main {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .launch-queue-item-topline {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12px;
  }
  .launch-queue-item-repo {
    color: var(--accent);
    font-weight: 600;
  }
  .launch-queue-item-number {
    color: var(--text-muted);
  }
  .launch-queue-item-title {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .launch-queue-item-meta {
    display: flex;
    gap: 10px;
    font-size: 11px;
    color: var(--text-ghost);
  }

  /* ── Tip ── */
  .launch-tip {
    color: var(--text-ghost);
    font-size: 12px;
    padding: 16px 40px 32px;
    text-align: center;
    max-width: 480px;
  }

  /* ── Modal (kept from original) ── */
  .launch-modal-backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, black 58%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 50;
  }
  .launch-modal {
    width: min(460px, 100%);
    border: 1px solid var(--border-default);
    border-radius: 12px;
    background: var(--surface-panel);
    box-shadow: var(--shadow-lg);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .launch-modal-kicker {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .launch-modal h3 {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  .launch-modal p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.5;
  }
  .launch-modal code {
    color: var(--text-primary);
    word-break: break-word;
  }
  .launch-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }

  /* ── Responsive ── */
  @media (max-width: 640px) {
    .launch-hero {
      padding: 40px 20px 24px;
    }
    .launch-columns {
      flex-direction: column;
      gap: 24px;
      padding: 0 20px 24px;
    }
    .launch-col:first-child {
      padding-right: 0;
    }
    .launch-col:last-child {
      padding-left: 0;
    }
    .launch-divider {
      display: none;
    }
    .launch-queue,
    .launch-tip {
      padding-left: 20px;
      padding-right: 20px;
    }
  }
</style>
