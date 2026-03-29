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

<div class="github-inbox">
  <div class="github-inbox-shell">
    <div class="github-inbox-header">
      <div>
        <div class="github-inbox-kicker">GitHub Review</div>
        <h2>Pull requests to review</h2>
        <p>Open a PR directly or pick one from your tracked review queue.</p>
      </div>
      <div class="github-inbox-header-actions">
        <Button variant="secondary" size="sm" onclick={() => void loadGitHubReviewQueue()}>
          Refresh
        </Button>
        <Button size="sm" onclick={() => void onOpenFolder()}>
          Open Folder
        </Button>
      </div>
    </div>

    <div class="github-inbox-manual">
      <div class="github-inbox-field">
        <label for="github-pr-ref">PR</label>
        <input
          id="github-pr-ref"
          bind:value={manualPrRef}
          placeholder="phin-tech/test-repo#16 or GitHub PR URL"
          class="github-inbox-input"
        />
      </div>
      <div class="github-inbox-field">
        <label for="github-local-path">Local checkout override</label>
        <input
          id="github-local-path"
          bind:value={manualLocalPath}
          placeholder="Optional. Defaults to Settings checkout location."
          class="github-inbox-input"
        />
      </div>
      <Button onclick={() => void openManualPr()} disabled={!manualPrRef.trim() || githubReview.opening}>
        Open PR
      </Button>
    </div>

    {#if githubReview.actionError}
      <div class="github-inbox-banner github-inbox-banner-error">{githubReview.actionError}</div>
    {/if}

    <div class="github-inbox-section">
      <div class="github-inbox-section-header">
        <h3>Review Queue</h3>
        <span>{githubReview.queue.length} items</span>
      </div>

      {#if githubReview.loadingQueue}
        <div class="github-inbox-empty">Loading review queue...</div>
      {:else if githubReview.queueError}
        <div class="github-inbox-empty">{githubReview.queueError}</div>
      {:else if githubReview.queue.length === 0}
        <div class="github-inbox-empty">
          No review-requested PRs found. Add tracked repos in Settings or open a PR manually.
        </div>
      {:else}
        <div class="github-inbox-list">
          {#each githubReview.queue as item (`${item.repo}#${item.number}`)}
            <button
              class="github-inbox-item"
              onclick={() => void openGitHubPullRequest(`${item.repo}#${item.number}`, item.localPath)}
            >
              <div class="github-inbox-item-main">
                <div class="github-inbox-item-topline">
                  <span class="github-inbox-item-repo">{item.repo}</span>
                  <span class="github-inbox-item-pr">#{item.number}</span>
                </div>
                <div class="github-inbox-item-title">{item.title}</div>
                <div class="github-inbox-item-meta">
                  <span>{item.author}</span>
                  <span>{item.baseRef} ← {item.headRef}</span>
                </div>
              </div>
              <div class="github-inbox-item-arrow">Open</div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="github-inbox-section">
      <div class="github-inbox-section-header">
        <h3>History</h3>
        <div class="github-inbox-history-actions">
          <Button variant="secondary" size="sm" onclick={() => void refreshHistory()}>
            Refresh
          </Button>
          <Button variant="secondary" size="sm" onclick={() => void handleCleanup()} disabled={cleaningHistory}>
            {cleaningHistory ? "Cleaning..." : "Clean stale"}
          </Button>
        </div>
      </div>

      {#if historyError}
        <div class="github-inbox-empty">{historyError}</div>
      {:else if !history}
        <div class="github-inbox-empty">Loading review history...</div>
      {:else}
        {#if history.activeSession}
          <div class="github-inbox-history-group">
            <div class="github-inbox-history-label">Resume active session</div>
            <button class="github-inbox-item" onclick={() => void handleResume(history.activeSession.id)}>
              <div class="github-inbox-item-main">
                <div class="github-inbox-item-topline">
                  <span class="github-inbox-item-repo">{history.activeSession.kind === "github_pr" ? "Pull request" : "File review"}</span>
                  <span class="github-inbox-item-pr">{history.activeSession.status}</span>
                </div>
                <div class="github-inbox-item-title">{history.activeSession.title}</div>
                <div class="github-inbox-item-meta">
                  <span>{history.activeSession.subtitle}</span>
                  <span>{history.activeSession.fileCount} files</span>
                </div>
              </div>
              <div class="github-inbox-item-arrow">Resume</div>
            </button>
          </div>
        {/if}

        {#if history.recentPullRequests.length > 0}
          <div class="github-inbox-history-group">
            <div class="github-inbox-history-label">Recent PRs</div>
            <div class="github-inbox-list">
              {#each history.recentPullRequests as item (item.id)}
                <button class="github-inbox-item" onclick={() => void handleResume(item.id)}>
                  <div class="github-inbox-item-main">
                    <div class="github-inbox-item-topline">
                      <span class="github-inbox-item-repo">{item.subtitle}</span>
                      <span class="github-inbox-item-pr">{item.status}</span>
                    </div>
                    <div class="github-inbox-item-title">{item.title}</div>
                    <div class="github-inbox-item-meta">
                      <span>{item.fileCount} files</span>
                      {#if item.verdict}<span>{item.verdict}</span>{/if}
                    </div>
                  </div>
                  <div class="github-inbox-item-arrow">Open</div>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        {#if history.recentFiles.length > 0}
          <div class="github-inbox-history-group">
            <div class="github-inbox-history-label">Recent file reviews</div>
            <div class="github-inbox-list">
              {#each history.recentFiles as item (item.id)}
                <button class="github-inbox-item" onclick={() => void handleResume(item.id)}>
                  <div class="github-inbox-item-main">
                    <div class="github-inbox-item-topline">
                      <span class="github-inbox-item-repo">File review</span>
                      <span class="github-inbox-item-pr">{item.status}</span>
                    </div>
                    <div class="github-inbox-item-title">{item.title}</div>
                    <div class="github-inbox-item-meta">
                      <span>{item.subtitle}</span>
                      {#if item.verdict}<span>{item.verdict}</span>{/if}
                    </div>
                  </div>
                  <div class="github-inbox-item-arrow">Resume</div>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        {#if history.staleSessions.length > 0}
          <div class="github-inbox-history-group">
            <div class="github-inbox-history-label">Stale sessions</div>
            <div class="github-inbox-list">
              {#each history.staleSessions as item (item.id)}
                <div class="github-inbox-item github-inbox-item-stale">
                  <div class="github-inbox-item-main">
                    <div class="github-inbox-item-topline">
                      <span class="github-inbox-item-repo">{item.kind === "github_pr" ? "Pull request" : "File review"}</span>
                      <span class="github-inbox-item-pr">stale</span>
                    </div>
                    <div class="github-inbox-item-title">{item.title}</div>
                    <div class="github-inbox-item-meta">
                      <span>{item.subtitle}</span>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/if}
    </div>
  </div>

  {#if pendingAutoCheckout}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="github-inbox-modal-backdrop" onclick={cancelAutoCheckout}>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class="github-inbox-modal" onclick={(event) => event.stopPropagation()}>
        <div class="github-inbox-modal-kicker">New Checkout</div>
        <h3>Clone repo for this review?</h3>
        <p>
          <strong>{pendingAutoCheckout.repo}</strong> is not tracked yet. Red Pen will clone it to
          <code>{pendingAutoCheckout.checkoutPath}</code> and then open the pull request.
        </p>
        <div class="github-inbox-modal-actions">
          <Button variant="secondary" size="sm" onclick={cancelAutoCheckout}>Cancel</Button>
          <Button size="sm" onclick={() => void confirmAutoCheckout()}>Clone and Open</Button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .github-inbox {
    height: 100%;
    overflow: auto;
    background: var(--surface-base);
  }
  .github-inbox-shell {
    max-width: 980px;
    margin: 0 auto;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .github-inbox-modal-backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, black 58%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 50;
  }
  .github-inbox-modal {
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
  .github-inbox-modal-kicker {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .github-inbox-modal h3 {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  .github-inbox-modal p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.5;
  }
  .github-inbox-modal code {
    color: var(--text-primary);
    word-break: break-word;
  }
  .github-inbox-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }
  .github-inbox-header,
  .github-inbox-manual,
  .github-inbox-section,
  .github-inbox-empty,
  .github-inbox-banner {
    border: 1px solid var(--border-default);
    border-radius: 10px;
    background: var(--surface-panel);
    box-shadow: var(--shadow-xs);
  }
  .github-inbox-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 16px;
  }
  .github-inbox-kicker {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
    margin-bottom: 4px;
  }
  .github-inbox-header h2 {
    margin: 0;
    font-size: 24px;
    line-height: 1.2;
    color: var(--text-primary);
  }
  .github-inbox-header p {
    margin: 6px 0 0;
    color: var(--text-secondary);
    font-size: 14px;
  }
  .github-inbox-header-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .github-inbox-manual {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr) auto;
    gap: 12px;
    padding: 12px;
    align-items: end;
  }
  .github-inbox-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .github-inbox-field label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .github-inbox-input {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-base);
    color: var(--text-primary);
    padding: 10px 12px;
    outline: none;
  }
  .github-inbox-input:focus {
    border-color: var(--accent);
  }
  .github-inbox-banner {
    padding: 12px 14px;
    color: var(--text-secondary);
  }
  .github-inbox-banner-error {
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border-default));
    color: var(--danger);
  }
  .github-inbox-section {
    overflow: hidden;
  }
  .github-inbox-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-default);
    background: var(--surface-raised);
  }
  .github-inbox-section-header h3 {
    margin: 0;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .github-inbox-section-header span {
    font-size: 12px;
    color: var(--text-muted);
  }
  .github-inbox-history-actions {
    display: flex;
    gap: 8px;
  }
  .github-inbox-history-group {
    display: grid;
    gap: 10px;
    padding: 14px;
    border-top: 1px solid var(--border-subtle);
  }
  .github-inbox-history-group:first-child {
    border-top: 0;
  }
  .github-inbox-history-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .github-inbox-list {
    display: flex;
    flex-direction: column;
  }
  .github-inbox-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 14px;
    border-top: 1px solid var(--border-subtle);
    text-align: left;
    background: transparent;
  }
  .github-inbox-item:first-child {
    border-top: 0;
  }
  .github-inbox-item:hover {
    background: var(--surface-highlight);
  }
  .github-inbox-item-main {
    min-width: 0;
    flex: 1;
  }
  .github-inbox-item-topline,
  .github-inbox-item-meta {
    display: flex;
    gap: 12px;
    align-items: center;
    font-size: 12px;
    color: var(--text-muted);
  }
  .github-inbox-item-title {
    margin: 6px 0;
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
  }
  .github-inbox-item-arrow {
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    flex-shrink: 0;
  }
  .github-inbox-item-stale {
    cursor: default;
    opacity: 0.72;
  }
  .github-inbox-empty {
    padding: 20px;
    color: var(--text-secondary);
    font-size: 14px;
    box-shadow: none;
  }
  @media (max-width: 840px) {
    .github-inbox-manual {
      grid-template-columns: 1fr;
    }
    .github-inbox-header {
      flex-direction: column;
    }
    .github-inbox-header-actions {
      width: 100%;
      justify-content: flex-start;
    }
    .github-inbox-item {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
