<script lang="ts">
  import BubbleKindFilter from "../BubbleKindFilter.svelte";
  import DiffModeToggle from "../DiffModeToggle.svelte";
  import DiffRefPicker from "../DiffRefPicker.svelte";
  import ReviewSubmitControl from "../review-header/ReviewSubmitControl.svelte";
  import { getEditor, getShowPreview, isMarkdownFile, togglePreview } from "$lib/stores/editor.svelte";
  import { getDiffState } from "$lib/stores/diff.svelte";
  import {
    getAnnotationCount,
    getReviewPageState,
    isReviewPageOpen,
  } from "$lib/stores/reviewPage.svelte";
  import { getReviewSession } from "$lib/stores/review.svelte";
  import {
    getGitHubReviewState,
    resyncActiveGitHubReview,
    discardActiveGitHubReviewChanges,
    submitActiveGitHubReview,
  } from "$lib/stores/githubReview.svelte";
  import { getBubblesEnabled, toggleBubbles } from "$lib/stores/annotations.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { submitReviewVerdict } from "$lib/review";
  import { clearReviewSession } from "$lib/stores/review.svelte";
  import { open as openUrl } from "@tauri-apps/plugin-shell";
  import type { SubmitGitHubReviewResult } from "$lib/types";

  let {
    showPrView,
    onAgentReviewVerdict,
    onEnterDiff,
    onSelectCodeView,
    onSelectPrView,
    onSelectReviewView,
  }: {
    showPrView: boolean;
    onAgentReviewVerdict: (verdict: "approved" | "changes_requested") => Promise<void>;
    onEnterDiff: (mode: import("$lib/types").DiffMode) => void;
    onSelectCodeView: () => void;
    onSelectPrView: () => void;
    onSelectReviewView: () => void;
  } = $props();

  const editor = getEditor();
  const diff = getDiffState();
  const reviewState = getReviewPageState();
  const reviewSession = getReviewSession();
  const githubReview = getGitHubReviewState();
  const workspace = getWorkspace();

  const directory = $derived(workspace.rootFolders[0] ?? "");
  const reviewAnnotationCount = $derived(getAnnotationCount());

  // Badge: always visible when count > 0, regardless of active tab
  const reviewTabBadge = $derived(reviewAnnotationCount > 0 ? reviewAnnotationCount : null);

  // Review context for left zone
  const reviewContextText = $derived.by(() => {
    if (githubReview.activeSession) {
      const s = githubReview.activeSession;
      return { label: `${s.repo} #${s.number}`, title: s.title };
    }
    if (reviewSession.active) {
      const count = reviewSession.files.length;
      return { label: `Agent change review`, title: `${count} file${count === 1 ? '' : 's'}` };
    }
    return null;
  });

  const isSelfAuthoredPr = $derived(
    Boolean(
      githubReview.activeSession?.authorLogin
      && githubReview.activeSession?.viewerLogin
      && githubReview.activeSession.authorLogin.toLowerCase()
        === githubReview.activeSession.viewerLogin.toLowerCase()
    )
  );

  // Current active view
  const isCodeView = $derived(!isReviewPageOpen() && !showPrView);
  const isReviewView = $derived(isReviewPageOpen() && !showPrView);
  const isPrViewActive = $derived(showPrView);

  async function handleLocalReviewVerdict(verdict: "approved" | "changes_requested") {
    const file = reviewSession.files[0];
    if (!file) return;
    await submitReviewVerdict(file, verdict);
    clearReviewSession();
  }
</script>

{#if editor.currentFilePath}
  <div class="consolidated-toolbar">
    <!-- Left zone: review context -->
    <div class="toolbar-left">
      {#if reviewContextText}
        {#if githubReview.activeSession?.url}
          <button class="context-link" onclick={() => openUrl(githubReview.activeSession!.url)}>{reviewContextText.label}</button>
        {:else}
          <span class="context-label">{reviewContextText.label}</span>
        {/if}
        {#if reviewContextText.title}
          <span class="context-separator">&middot;</span>
          <span class="context-title">{reviewContextText.title}</span>
        {/if}
      {/if}
    </div>

    <!-- Center zone: view switcher -->
    <div class="toolbar-center">
      <div class="view-tabs">
        <button
          class="view-tab"
          class:active={isCodeView}
          onclick={onSelectCodeView}
        >
          Code
        </button>
        <button
          class="view-tab"
          class:active={isReviewView}
          onclick={onSelectReviewView}
        >
          Review
          {#if reviewTabBadge !== null}
            <span class="view-tab-badge">{reviewTabBadge}</span>
          {/if}
        </button>
        {#if githubReview.activeSession}
          <button
            class="view-tab"
            class:active={isPrViewActive}
            onclick={onSelectPrView}
          >
            PR
          </button>
        {/if}
      </div>
    </div>

    <!-- Right zone: mode-specific actions -->
    <div class="toolbar-right">
      {#if isReviewView && githubReview.activeSession}
        <!-- GitHub review mode actions -->
        <button
          class="icon-btn"
          title="Resync review"
          onclick={() => void resyncActiveGitHubReview()}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21.5 2v6h-6"></path>
            <path d="M2.5 22v-6h6"></path>
            <path d="M2 11.5a10 10 0 0 1 18.8-4.3"></path>
            <path d="M22 12.5a10 10 0 0 1-18.8 4.2"></path>
          </svg>
        </button>
        <button
          class="icon-btn"
          title="Revert pending changes"
          onclick={() => void discardActiveGitHubReviewChanges()}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="1 4 1 10 7 10"></polyline>
            <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
          </svg>
        </button>
        <ReviewSubmitControl {isSelfAuthoredPr} onSubmitReview={submitActiveGitHubReview} />
      {:else if isReviewView && reviewSession.active && !githubReview.activeSession}
        <!-- Local review mode actions -->
        <button
          class="ghost-btn ghost-btn-success"
          onclick={() => void handleLocalReviewVerdict("approved")}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          Approve
        </button>
        <button
          class="ghost-btn ghost-btn-danger"
          onclick={() => void handleLocalReviewVerdict("changes_requested")}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>
          Request Changes
        </button>
      {:else if isCodeView}
        <!-- Code view actions -->
        <DiffModeToggle onEnterDiff={onEnterDiff} />
        {#if diff.enabled}
          <DiffRefPicker {directory} filePath={editor.currentFilePath ?? ""} />
        {/if}

        <button
          class="toggle-btn"
          class:active={getBubblesEnabled()}
          onclick={() => toggleBubbles()}
          title="Toggle inline annotations"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
          </svg>
          Inline
        </button>

        {#if getBubblesEnabled()}
          <BubbleKindFilter />
        {/if}

        {#if isMarkdownFile() && !diff.enabled}
          <button
            class="toggle-btn"
            class:active={!getShowPreview()}
            onclick={() => {
              if (getShowPreview()) togglePreview();
            }}
          >
            Source
          </button>
          <button
            class="toggle-btn"
            class:active={getShowPreview()}
            onclick={() => {
              if (!getShowPreview()) togglePreview();
            }}
          >
            Preview
          </button>
        {/if}

      {/if}
    </div>
  </div>
{/if}

<style>
  .consolidated-toolbar {
    display: flex;
    align-items: center;
    padding: 0 10px;
    min-height: 42px;
    box-sizing: border-box;
    background: var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
    gap: 12px;
  }

  /* Left zone: review context */
  .toolbar-left {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }
  .context-label {
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .context-link {
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
    border: none;
    background: none;
    padding: 0;
    font-family: inherit;
  }
  .context-link:hover {
    color: var(--accent-hover);
  }
  .context-separator {
    color: var(--text-muted);
    font-size: 12px;
    flex-shrink: 0;
  }
  .context-title {
    color: var(--text-muted);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  /* Center zone: view switcher */
  .toolbar-center {
    flex-shrink: 0;
  }
  .view-tabs {
    display: flex;
    align-items: center;
    gap: 0;
  }
  .view-tab {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    min-height: 42px;
    padding: 0 14px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s;
    white-space: nowrap;
  }
  .view-tab:hover {
    color: var(--text-secondary);
  }
  .view-tab.active {
    color: var(--view-active);
  }
  .view-tab.active::after {
    content: "";
    position: absolute;
    bottom: 0;
    left: 6px;
    right: 6px;
    height: 2px;
    background: var(--accent-badge-text, #D9B15F);
    border-radius: 1px;
  }
  .view-tab-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: transparent;
    border: 1px solid var(--accent-badge-border);
    color: var(--accent-badge-text);
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  /* Right zone: actions */
  .toolbar-right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Icon buttons (Resync, Revert) */
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border-default);
    background: var(--surface-raised);
    color: var(--text-secondary);
    border-radius: 6px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }
  .icon-btn:hover {
    color: var(--text-primary);
    border-color: var(--border-emphasis);
    background: var(--surface-highlight);
  }

  /* Ghost buttons (Approve, Request Changes) */
  .ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 28px;
    padding: 4px 10px;
    border: none;
    background: transparent;
    font-size: 12px;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s, background 0.15s;
    white-space: nowrap;
  }
  .ghost-btn-success {
    color: var(--color-success);
  }
  .ghost-btn-success:hover {
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
  }
  .ghost-btn-danger {
    color: color-mix(in srgb, var(--color-danger) 65%, var(--text-muted));
  }
  .ghost-btn-danger:hover {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
  }

  /* Toggle buttons (reused for code view tools) */
  .toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 30px;
    padding: 4px 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    border-radius: 6px;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
    white-space: nowrap;
  }
  .toggle-btn:hover {
    color: var(--text-secondary);
    background: var(--surface-raised);
    border-color: var(--border-default);
  }
  .toggle-btn.active {
    color: var(--view-active);
    background: var(--view-active-subtle);
    border-color: var(--view-active-border);
  }

  /* Global styles needed by ReviewSubmitControl */
  :global(.review-summary-btn) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 30px;
    padding: 4px 10px;
    border: 1px solid var(--border-default);
    background: var(--surface-raised);
    color: var(--text-secondary);
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
    white-space: nowrap;
  }
  :global(.review-summary-btn:hover) {
    color: var(--text-primary);
    border-color: var(--border-emphasis);
    background: var(--surface-highlight);
  }
  :global(.review-summary-btn-primary) {
    color: var(--surface-base);
    background: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 75%, black 25%);
  }
  :global(.review-summary-btn-primary:hover) {
    color: var(--surface-base);
    background: var(--accent-hover);
    border-color: color-mix(in srgb, var(--accent-hover) 75%, black 25%);
  }
  :global(.review-summary-btn-success) {
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 35%, var(--border-default));
    background: color-mix(in srgb, var(--color-success) 14%, transparent);
  }
  :global(.review-summary-btn-success:hover) {
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 60%, var(--border-default));
    background: color-mix(in srgb, var(--color-success) 22%, transparent);
  }
  :global(.review-summary-btn-danger) {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-default));
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
  }
  :global(.review-summary-btn-danger:hover) {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 60%, var(--border-default));
    background: color-mix(in srgb, var(--color-danger) 22%, transparent);
  }
  :global(.review-header-submit-menu) {
    position: relative;
  }
  :global(.review-summary-chevron) {
    font-size: 10px;
    line-height: 1;
  }
  :global(.review-summary-dropdown) {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    padding: 6px;
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    box-shadow: var(--shadow-popover);
    z-index: 20;
  }
  :global(.review-summary-dropdown-item) {
    display: flex;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    font-size: 12px;
    border-radius: 6px;
    cursor: pointer;
  }
  :global(.review-summary-dropdown-item:hover) {
    background: var(--surface-highlight);
  }
  :global(.review-summary-dropdown-item-success) {
    color: var(--color-success);
  }
  :global(.review-summary-dropdown-item-danger) {
    color: var(--color-danger);
  }
  :global(.review-summary-dropdown-note) {
    padding: 8px 10px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-muted);
  }
  :global(.review-submit-backdrop) {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(6, 7, 10, 0.76);
    backdrop-filter: blur(2px);
    z-index: 40;
  }
  :global(.review-submit-modal) {
    width: min(520px, 100%);
    background: var(--surface-panel);
    border: 1px solid var(--border-emphasis);
    border-radius: 10px;
    box-shadow: var(--shadow-popover);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  :global(.review-submit-kicker) {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  :global(.review-submit-modal h2) {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  :global(.review-submit-modal p) {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-secondary);
  }
  :global(.review-submit-field) {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  :global(.review-submit-field span) {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  :global(.review-submit-textarea) {
    width: 100%;
    min-height: 132px;
    resize: vertical;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-base);
    color: var(--text-primary);
    padding: 10px 12px;
    font: inherit;
    outline: none;
  }
  :global(.review-submit-textarea:focus) {
    border-color: var(--accent);
  }
  :global(.review-submit-error) {
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--color-danger) 35%, var(--border-default));
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    color: var(--color-danger);
    font-size: 12px;
  }
  :global(.review-submit-actions) {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  :global(.review-submit-thinking-arrow) {
    display: inline-block;
    animation: review-submit-thinking 0.9s ease-in-out infinite;
  }
  @keyframes -global-review-submit-thinking {
    0%,
    100% {
      transform: translateX(0) translateY(0);
      opacity: 0.65;
    }
    50% {
      transform: translateX(2px) translateY(-1px);
      opacity: 1;
    }
  }
</style>
