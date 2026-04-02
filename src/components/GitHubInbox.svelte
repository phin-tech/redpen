<script lang="ts">
  import {
    clearAgentStatus,
    getReviewHistory,
    getSettings,
    resumeReviewSession,
  } from "$lib/tauri";
  import type { AppSettings, ReviewHistory } from "$lib/types";
  import { getVersion } from "@tauri-apps/api/app";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import Button from "./ui/Button.svelte";
  import type { InboxCategory } from "$lib/types";
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

  let {
    onOpenFolder,
    onOpenSettings,
  }: {
    onOpenFolder: () => Promise<void>;
    onOpenSettings?: () => void;
  } = $props();

  let version = $state<string | null>(null);

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


  // Heartbeat staleness: if last_heartbeat > 60s ago, agent is stalled
  const HEARTBEAT_STALE_MS = 60_000;

  type LocalItem = { agentStatus: string | null; agentTask: string | null; lastHeartbeat: string | null };

  function agentDisplayStatus(item: LocalItem): { dot: "amber" | "red" | null; label: string | null } {
    if (!item.agentStatus || item.agentStatus === "idle") return { dot: null, label: null };
    if (item.agentStatus === "interrupted") return { dot: "red", label: "Agent: Interrupted" };
    if (item.agentStatus === "error") return { dot: "red", label: "Agent: Error" };
    if (item.agentStatus === "busy") {
      if (item.lastHeartbeat) {
        const stale = Date.now() - new Date(item.lastHeartbeat).getTime() > HEARTBEAT_STALE_MS;
        if (stale) return { dot: "red", label: "Agent: Stalled" };
      }
      return { dot: "amber", label: item.agentTask ? `Agent: ${item.agentTask}` : "Agent: Running" };
    }
    return { dot: null, label: null };
  }

  function isAgentDismissible(item: LocalItem): boolean {
    if (!item.agentStatus) return false;
    if (item.agentStatus === "interrupted" || item.agentStatus === "error") return true;
    if (item.agentStatus === "busy" && item.lastHeartbeat) {
      return Date.now() - new Date(item.lastHeartbeat).getTime() > HEARTBEAT_STALE_MS;
    }
    return false;
  }

  const CATEGORY_ORDER: InboxCategory[] = ["ReviewRequested", "Assigned", "Authored", "Mentioned"];
  const CATEGORY_LABELS: Record<InboxCategory, string> = {
    ReviewRequested: "Review Requested",
    Assigned: "Assigned",
    Authored: "Authored",
    Mentioned: "Mentioned",
  };

  type SortField = "updated" | "repo" | "title" | "org";
  let sortField = $state<SortField>("updated");
  let sortDir = $state<"asc" | "desc">("desc");

  function setSort(field: SortField) {
    if (sortField === field) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortField = field;
      sortDir = field === "updated" ? "desc" : "asc";
    }
  }

  let sortedQueue = $derived((() => {
    const items = [...githubReview.queue];
    const dir = sortDir === "asc" ? 1 : -1;
    items.sort((a, b) => {
      let cmp = 0;
      if (sortField === "updated") {
        cmp = a.updatedAt < b.updatedAt ? -1 : a.updatedAt > b.updatedAt ? 1 : 0;
      } else if (sortField === "repo") {
        cmp = a.repo.toLowerCase().localeCompare(b.repo.toLowerCase());
      } else if (sortField === "title") {
        cmp = a.title.toLowerCase().localeCompare(b.title.toLowerCase());
      } else if (sortField === "org") {
        const orgA = a.repo.split("/")[0].toLowerCase();
        const orgB = b.repo.split("/")[0].toLowerCase();
        cmp = orgA.localeCompare(orgB);
      }
      return cmp * dir;
    });
    return items;
  })());

  let queueGroups = $derived(buildQueueGroups());

  function buildQueueGroups() {
    const queue = sortedQueue;
    const distinctCategories = CATEGORY_ORDER.filter((cat) =>
      queue.some((item) => item.categories[0] === cat),
    );
    if (distinctCategories.length <= 1) {
      return [{ label: null, items: queue }];
    }
    return distinctCategories.map((cat) => ({
      label: `${CATEGORY_LABELS[cat]} · ${queue.filter((item) => item.categories[0] === cat).length}`,
      items: queue.filter((item) => item.categories[0] === cat),
    }));
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


  onMount(() => {
    void getVersion().then((v) => { version = v; }).catch(() => {});
    void initialize();
    const poll = window.setInterval(() => {
      void loadGitHubReviewQueue();
      void refreshHistory();
    }, 30000);

    // Refresh history whenever an agent posts a status update
    const unlistenPromise = listen("agent-status-changed", () => {
      void refreshHistory();
    });

    return () => {
      window.clearInterval(poll);
      void unlistenPromise.then((unlisten) => unlisten());
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
    } catch {
      // history stays as last known state
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

  async function handleClearAgent(sessionId: string) {
    await clearAgentStatus(sessionId);
    await refreshHistory();
  }
</script>

<div class="launch-screen">
  <!-- Brand bar -->
  <div class="launch-topbar">
    <div class="launch-brand">
      <span class="launch-brand-glyph">/</span>
      <span class="launch-brand-red">RED</span><span class="launch-brand-pen">PEN</span>
      {#if version}<span class="launch-brand-version">v{version}</span>{/if}
    </div>
    {#if onOpenSettings}
      <button class="launch-topbar-btn" onclick={onOpenSettings} title="Settings">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
      </button>
    {/if}
  </div>

  <!-- Search -->
  <div class="launch-search-row">
    <div class="launch-search-wrapper">
      <span class="launch-search-icon">&#x2315;</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        bind:value={manualPrRef}
        placeholder="Paste GitHub URL or type to search local sessions…"
        class="launch-search-input"
        autofocus
        onkeydown={(e) => { if (e.key === 'Enter') void openManualPr(); }}
      />
    </div>
  </div>

  {#if githubReview.actionError}
    <div class="launch-error">{githubReview.actionError}</div>
  {/if}

  <!-- INBOX section — at the top, always visible when items exist -->
  {#if githubReview.queue.length > 0}
    <div class="launch-queue">
      <div class="launch-queue-header">
        <span class="launch-section-label">INBOX <span class="launch-section-count">{githubReview.queue.length}</span></span>
        <div class="launch-sort-pills">
          {#each ([["updated", "Updated"], ["org", "Org"], ["repo", "Repo"], ["title", "Title"]] as const) as [field, label]}
            <button
              class="launch-sort-pill"
              class:launch-sort-pill-active={sortField === field}
              onclick={() => setSort(field)}
            >{label}{sortField === field ? (sortDir === "desc" ? " ↓" : " ↑") : ""}</button>
          {/each}
        </div>
      </div>
      {#each queueGroups as group}
        {#if group.label}
          <div class="launch-queue-group-label">{group.label}</div>
        {/if}
        <div class="launch-queue-list">
          {#each group.items as item (`${item.repo}#${item.number}`)}
            <button
              class="launch-queue-item"
              onclick={() => void openGitHubPullRequest(`${item.repo}#${item.number}`)}
            >
              <svg class="launch-queue-item-icon" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
              <div class="launch-queue-item-main">
                <div class="launch-queue-item-topline">
                  <span class="launch-queue-item-repo">{item.repo}</span>
                  <span class="launch-queue-item-number">#{item.number}</span>
                </div>
                <div class="launch-queue-item-title">{item.title}</div>
                <div class="launch-queue-item-meta">
                  <span>{item.author}</span>
                  <span>{relativeTime(item.updatedAt)}</span>
                </div>
              </div>
              <span class="launch-ghost-btn">Open</span>
            </button>
          {/each}
        </div>
      {/each}
    </div>
  {/if}

  <!-- LOCAL section — dirty/active repos -->
  <div class="launch-section">
    <div class="launch-section-header">
      <span class="launch-section-label">
        LOCAL
        {#if history?.workspaceLocal?.length}
          <span class="launch-section-count">{history.workspaceLocal.length}</span>
        {/if}
      </span>
      <button class="launch-ghost-btn launch-ghost-btn-small" onclick={() => void onOpenFolder()}>+ Open Folder</button>
    </div>
    {#if history?.workspaceLocal?.length}
      <div class="launch-queue-list">
        {#each history.workspaceLocal as item (item.id)}
          {@const agentInfo = agentDisplayStatus(item)}
          {@const dismissible = isAgentDismissible(item)}
          <button class="launch-queue-item" onclick={() => void handleResume(item.id)}>
            <svg class="launch-queue-item-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
            <div class="launch-queue-item-main">
              <div class="launch-queue-item-topline">
                {#if agentInfo.dot}
                  <span class="launch-agent-dot launch-agent-dot-{agentInfo.dot}" class:launch-agent-dot-pulse={agentInfo.dot === 'amber'}></span>
                {/if}
                <span class="launch-queue-item-repo">{item.subtitle}</span>
                <span class="launch-queue-item-number">{item.branchName ?? ""}</span>
                {#if agentInfo.label}
                  <span class="launch-agent-label">{agentInfo.label}</span>
                {/if}
                {#if dismissible}
                  <!-- svelte-ignore a11y_interactive_supports_focus -->
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <span role="button" class="launch-agent-dismiss" onclick={(e) => { e.stopPropagation(); void handleClearAgent(item.id); }} title="Clear agent status">×</span>
                {/if}
              </div>
              <div class="launch-queue-item-title">{item.title}</div>
              <div class="launch-queue-item-meta">
                <span>{relativeTime(item.updatedAt)}</span>
              </div>
            </div>
            <span class="launch-ghost-btn">Resume</span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="launch-section-empty">No active local sessions — <button class="launch-link-btn" onclick={() => void onOpenFolder()}>open a folder</button></div>
    {/if}
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

  /* ── Top bar ── */
  .launch-topbar {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px 0;
    box-sizing: border-box;
    flex-shrink: 0;
  }
  .launch-brand {
    display: flex;
    align-items: baseline;
    gap: 5px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    letter-spacing: 0.04em;
  }
  .launch-brand-glyph {
    color: var(--accent);
    font-weight: 700;
    font-size: 14px;
    line-height: 1;
  }
  .launch-brand-red {
    color: #8B949E;
    font-weight: 400;
  }
  .launch-brand-pen {
    color: var(--accent);
    font-weight: 700;
  }
  .launch-brand-version {
    color: #484f58;
    font-weight: 400;
    font-size: 10px;
    font-variant: small-caps;
    letter-spacing: 0.06em;
    margin-left: 2px;
  }
  .launch-topbar-btn {
    background: transparent;
    border: none;
    color: var(--text-ghost);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    transition: color 100ms;
  }
  .launch-topbar-btn:hover {
    color: var(--text-secondary);
  }

  /* ── Search ── */
  .launch-search-row {
    width: 100%;
    max-width: 720px;
    padding: 16px 20px 12px;
    box-sizing: border-box;
  }
  .launch-search-wrapper {
    max-width: 560px;
    position: relative;
  }
  .launch-search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-ghost);
    font-size: 15px;
    pointer-events: none;
  }
  .launch-search-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--surface-panel);
    border: 1px solid color-mix(in srgb, var(--accent) 25%, var(--border-default));
    border-radius: 8px;
    height: 40px;
    padding: 0 16px 0 36px;
    font-size: 13px;
    color: var(--text-primary);
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .launch-search-input::placeholder {
    color: var(--text-ghost);
  }
  .launch-search-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(217, 177, 95, 0.2);
  }

  /* ── Sections ── */
  .launch-section {
    max-width: 720px;
    width: 100%;
    padding: 0 20px 16px;
    box-sizing: border-box;
    margin: 0 auto;
  }
  .launch-section-empty {
    color: var(--text-ghost);
    font-size: 12px;
    padding: 6px 0;
  }
  .launch-link-btn {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .launch-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .launch-section-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .launch-section-count {
    font-weight: 400;
    color: var(--text-ghost);
  }
  .launch-agent-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .launch-agent-dot-amber {
    background: #d9b15f;
  }
  .launch-agent-dot-red {
    background: var(--danger, #e05c5c);
  }
  .launch-agent-dot-pulse {
    animation: agent-pulse 1.5s ease-in-out infinite;
  }
  @keyframes agent-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }
  .launch-agent-label {
    color: var(--text-muted);
    font-size: 11px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .launch-agent-dismiss {
    background: transparent;
    border: none;
    color: var(--text-ghost);
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
  }
  .launch-agent-dismiss:hover {
    color: var(--text-secondary);
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

  /* ── Column actions (used in Recent Sessions header) ── */
  .launch-col-actions {
    display: flex;
    gap: 6px;
  }

  /* ── Open Folder button (LOCAL header ghost) ── */
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

  /* ── INBOX Queue ── */
  .launch-queue {
    max-width: 720px;
    width: 100%;
    padding: 0 20px 24px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0 auto;
  }
  .launch-queue-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .launch-sort-pills {
    display: flex;
    gap: 4px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 20px;
    padding: 3px;
  }
  .launch-sort-pill {
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: 20px;
    padding: 2px 8px;
    font-size: 11px;
    color: var(--text-ghost);
    cursor: pointer;
    font-family: inherit;
    transition: background 100ms, color 100ms, border-color 100ms;
    white-space: nowrap;
  }
  .launch-sort-pill:hover {
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.04);
  }
  .launch-sort-pill-active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .launch-queue-group-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    padding: 6px 0 4px;
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
  .launch-queue-item-icon {
    flex-shrink: 0;
    color: var(--text-ghost);
    align-self: center;
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

  /* ── Empty state ── */
  .launch-empty {
    padding: 16px 20px;
    display: flex;
    justify-content: flex-start;
  }
</style>
