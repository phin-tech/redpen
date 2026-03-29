<script lang="ts">
  import { open as openUrl } from "@tauri-apps/plugin-shell";
  import { getReviewSession, clearReviewSession } from "$lib/stores/review.svelte";
  import {
    getGitHubReviewState,
    resyncActiveGitHubReview,
    discardActiveGitHubReviewChanges,
    submitActiveGitHubReview,
  } from "$lib/stores/githubReview.svelte";
  import { submitReviewVerdict, type ReviewVerdict } from "$lib/review";
  import ReviewActionsGroup from "./review-header/ReviewActionsGroup.svelte";
  import ReviewContextBanner from "./review-header/ReviewContextBanner.svelte";

  let { onOpenHelp }: { onOpenHelp: () => void } = $props();

  const reviewSession = getReviewSession();
  const githubReview = getGitHubReviewState();

  const hasReviewContext = $derived(
    Boolean(githubReview.activeSession || reviewSession.active)
  );
  const isSelfAuthoredPr = $derived(
    Boolean(
      githubReview.activeSession?.authorLogin
      && githubReview.activeSession?.viewerLogin
      && githubReview.activeSession.authorLogin.toLowerCase()
        === githubReview.activeSession.viewerLogin.toLowerCase()
    )
  );
  const reviewContextTitle = $derived.by(() => {
    if (githubReview.activeSession) {
      return `${githubReview.activeSession.repo} #${githubReview.activeSession.number}`;
    }
    return "Agent change review";
  });
  const reviewContextLabel = $derived.by(() => {
    if (githubReview.activeSession) return "Pull request review";
    return "Agent change review";
  });
  const reviewContextMeta = $derived.by(() => {
    if (githubReview.activeSession) {
      return `Into ${githubReview.activeSession.baseRef} from ${githubReview.activeSession.headRef}`;
    }
    return `${reviewSession.files.length} file${reviewSession.files.length === 1 ? "" : "s"} to review`;
  });
  async function handleOpenPullRequest() {
    if (!githubReview.activeSession?.url) return;
    await openUrl(githubReview.activeSession.url);
  }

  async function handleLocalReviewVerdict(verdict: ReviewVerdict) {
    const file = reviewSession.files[0];
    if (!file) return;
    await submitReviewVerdict(file, verdict);
    clearReviewSession();
  }
</script>

{#if hasReviewContext}
  <div class="review-summary">
    <ReviewContextBanner
      isGitHubReview={Boolean(githubReview.activeSession)}
      linkText={reviewContextTitle}
      meta={reviewContextMeta}
      title={githubReview.activeSession ? githubReview.activeSession.title : reviewContextTitle}
      titleLabel={reviewContextLabel}
      onOpenPullRequest={handleOpenPullRequest}
    />

    <ReviewActionsGroup
      githubReviewActive={Boolean(githubReview.activeSession)}
      {isSelfAuthoredPr}
      onDiscardPending={async () => {
        await discardActiveGitHubReviewChanges();
      }}
      onLocalApprove={async () => {
        await handleLocalReviewVerdict("approved");
      }}
      onLocalRequestChanges={async () => {
        await handleLocalReviewVerdict("changes_requested");
      }}
      onResync={async () => {
        await resyncActiveGitHubReview();
      }}
      onSubmitReview={submitActiveGitHubReview}
      reviewSessionActive={reviewSession.active}
    />

    <button type="button" class="review-summary-help" onclick={onOpenHelp}>
      Help
      <kbd>?</kbd>
    </button>
  </div>
{/if}

<style>
  .review-summary {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    background: var(--gradient-panel), var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .review-summary-help,
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
  .review-summary-help:hover,
  :global(.review-summary-btn:hover) {
    color: var(--text-primary);
    border-color: var(--border-emphasis);
    background: var(--surface-highlight);
  }
  .review-summary-help kbd {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 10px;
    color: var(--text-muted);
  }
  :global(.review-summary-main) {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
  }
  :global(.review-summary-kicker) {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  :global(.review-summary-line) {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    flex-wrap: wrap;
  }
  :global(.review-summary-link),
  :global(.review-summary-title) {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    cursor: default;
  }
  :global(.review-summary-link) {
    cursor: pointer;
  }
  :global(.review-summary-link:hover) {
    color: var(--accent);
  }
  :global(.review-summary-meta) {
    color: var(--text-secondary);
    font-size: 12px;
  }
  :global(.review-summary-actions) {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
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
  @keyframes review-submit-thinking {
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
