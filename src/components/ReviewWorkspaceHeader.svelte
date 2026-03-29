<script lang="ts">
  import { open as openUrl } from "@tauri-apps/plugin-shell";
  import type { SubmitGitHubReviewResult } from "$lib/types";
  import { getReviewSession } from "$lib/stores/review.svelte";
  import {
    getGitHubReviewState,
    resyncActiveGitHubReview,
    discardActiveGitHubReviewChanges,
    submitActiveGitHubReview,
  } from "$lib/stores/githubReview.svelte";

  let { onOpenHelp }: { onOpenHelp: () => void } = $props();

  const reviewSession = getReviewSession();
  const githubReview = getGitHubReviewState();
  let showSubmitMenu = $state(false);
  let submitModalAction = $state<"comment" | "approve" | "requestChanges" | null>(null);
  let submitModalMessage = $state("");
  let submitModalStatus = $state<"editing" | "submitting" | "success" | "error">("editing");
  let submitModalError = $state<string | null>(null);
  let submitModalResult = $state<SubmitGitHubReviewResult | null>(null);

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

  function openSubmitModal(action: "comment" | "approve" | "requestChanges") {
    showSubmitMenu = false;
    submitModalAction = action;
    submitModalMessage = "";
    submitModalStatus = "editing";
    submitModalError = null;
    submitModalResult = null;
  }

  function closeSubmitModal() {
    if (submitModalStatus === "submitting") return;
    submitModalAction = null;
    submitModalMessage = "";
    submitModalStatus = "editing";
    submitModalError = null;
    submitModalResult = null;
  }

  async function handleSubmitReview() {
    if (!submitModalAction) return;
    submitModalStatus = "submitting";
    submitModalError = null;

    try {
      const result = await submitActiveGitHubReview(
        submitModalAction,
        submitModalMessage.trim() || undefined,
      );
      submitModalResult = result;
      submitModalStatus = "success";
    } catch (error) {
      submitModalError = error instanceof Error ? error.message : String(error);
      submitModalStatus = "error";
    }
  }

  function handleWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target?.closest(".review-header-submit-menu")) {
      showSubmitMenu = false;
    }
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && showSubmitMenu) {
      showSubmitMenu = false;
      return;
    }

    if (e.key === "Escape" && submitModalAction && submitModalStatus !== "submitting") {
      e.preventDefault();
      e.stopPropagation();
      closeSubmitModal();
    }
  }

  function submitActionLabel(action: "comment" | "approve" | "requestChanges"): string {
    switch (action) {
      case "comment":
        return "Submit comments";
      case "approve":
        return "Approve";
      case "requestChanges":
        return "Request changes";
    }
  }

  function submitActionDescription(action: "comment" | "approve" | "requestChanges"): string {
    switch (action) {
      case "comment":
        return "Post your pending inline comments without approving or blocking the PR.";
      case "approve":
        return "Approve this PR and publish your pending inline comments.";
      case "requestChanges":
        return "Request changes on this PR and publish your pending inline comments.";
    }
  }

  function submitSuccessMessage(result: SubmitGitHubReviewResult | null): string {
    if (!result) return "GitHub accepted the review.";

    const parts = [];
    if (result.publishedCount > 0) {
      parts.push(`${result.publishedCount} comment${result.publishedCount === 1 ? "" : "s"}`);
    }
    if (result.replyCount > 0) {
      parts.push(`${result.replyCount} repl${result.replyCount === 1 ? "y" : "ies"}`);
    }

    return parts.length > 0
      ? `Published ${parts.join(" and ")} to GitHub.`
      : "GitHub accepted the review.";
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydowncapture={handleWindowKeydown} />

{#if hasReviewContext}
  <div class="review-summary">
    <button type="button" class="review-summary-help" onclick={onOpenHelp}>
      Help
      <kbd>?</kbd>
    </button>

    <div class="review-summary-main">
      <div class="review-summary-line">
        <div class="review-summary-kicker">{reviewContextLabel}</div>
        {#if githubReview.activeSession}
          <button
            type="button"
            class="review-summary-link"
            onclick={() => void handleOpenPullRequest()}
          >
            {reviewContextTitle}
          </button>
        {/if}
      </div>
      {#if githubReview.activeSession}
        <div class="review-summary-line">
          <div class="review-summary-title">{githubReview.activeSession.title}</div>
          <div class="review-summary-meta">{reviewContextMeta}</div>
        </div>
      {:else}
        <div class="review-summary-line">
          <div class="review-summary-title">{reviewContextMeta}</div>
        </div>
      {/if}
    </div>

    {#if githubReview.activeSession}
      <div class="review-summary-actions">
        <button type="button" class="review-summary-btn" onclick={() => void resyncActiveGitHubReview()}>
          Resync
        </button>
        <button type="button" class="review-summary-btn" onclick={() => void discardActiveGitHubReviewChanges()}>
          Discard pending
        </button>
        <div class="review-header-submit-menu">
          <button
            type="button"
            class="review-summary-btn review-summary-btn-primary"
            onclick={(event) => {
              event.stopPropagation();
              showSubmitMenu = !showSubmitMenu;
            }}
          >
            Submit review
            <span class="review-summary-chevron">▾</span>
          </button>
          {#if showSubmitMenu}
            <div class="review-summary-dropdown">
              <button class="review-summary-dropdown-item" onclick={() => openSubmitModal("comment")}>
                Submit comments
              </button>
              {#if !isSelfAuthoredPr}
                <button class="review-summary-dropdown-item review-summary-dropdown-item-success" onclick={() => openSubmitModal("approve")}>
                  Approve
                </button>
                <button class="review-summary-dropdown-item review-summary-dropdown-item-danger" onclick={() => openSubmitModal("requestChanges")}>
                  Request changes
                </button>
              {:else}
                <div class="review-summary-dropdown-note">
                  You cannot approve or request changes on your own pull request.
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

{#if submitModalAction}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="review-submit-backdrop" onclick={closeSubmitModal}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="review-submit-modal"
      role="dialog"
      aria-modal="true"
      aria-label="Submit GitHub review"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
    >
      {#if submitModalStatus === "success"}
        <div class="review-submit-kicker">Review submitted</div>
        <h2>{submitActionLabel(submitModalAction)}</h2>
        <p>{submitSuccessMessage(submitModalResult)}</p>
        <div class="review-submit-actions">
          <button type="button" class="review-summary-btn review-summary-btn-primary" onclick={closeSubmitModal}>
            Close
          </button>
        </div>
      {:else}
        <div class="review-submit-kicker">Submit review</div>
        <h2>{submitActionLabel(submitModalAction)}</h2>
        <p>{submitActionDescription(submitModalAction)}</p>

        <label class="review-submit-field">
          <span>Message</span>
          <textarea
            class="review-submit-textarea"
            bind:value={submitModalMessage}
            placeholder="Optional summary for this review"
            rows="6"
            disabled={submitModalStatus === "submitting"}
          ></textarea>
        </label>

        {#if submitModalError}
          <div class="review-submit-error">{submitModalError}</div>
        {/if}

        <div class="review-submit-actions">
          <button
            type="button"
            class="review-summary-btn"
            onclick={closeSubmitModal}
            disabled={submitModalStatus === "submitting"}
          >
            Cancel
          </button>
          <button
            type="button"
            class="review-summary-btn review-summary-btn-primary"
            onclick={() => void handleSubmitReview()}
            disabled={submitModalStatus === "submitting"}
          >
            {#if submitModalStatus === "submitting"}
              <span class="review-submit-thinking-arrow" aria-hidden="true">↗</span>
              Submitting…
            {:else}
              {submitActionLabel(submitModalAction)}
            {/if}
          </button>
        </div>
      {/if}
    </div>
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
  .review-summary-btn {
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
  .review-summary-btn:hover {
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
  .review-summary-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
  }
  .review-summary-kicker {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .review-summary-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    flex-wrap: wrap;
  }
  .review-summary-link,
  .review-summary-title {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    cursor: default;
  }
  .review-summary-link {
    cursor: pointer;
  }
  .review-summary-link:hover {
    color: var(--accent);
  }
  .review-summary-meta {
    color: var(--text-secondary);
    font-size: 12px;
  }
  .review-summary-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .review-summary-btn-primary {
    color: var(--surface-base);
    background: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 75%, black 25%);
  }
  .review-summary-btn-primary:hover {
    color: var(--surface-base);
    background: var(--accent-hover);
    border-color: color-mix(in srgb, var(--accent-hover) 75%, black 25%);
  }
  .review-header-submit-menu {
    position: relative;
  }
  .review-summary-chevron {
    font-size: 10px;
    line-height: 1;
  }
  .review-summary-dropdown {
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
  .review-summary-dropdown-item {
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
  .review-summary-dropdown-item:hover {
    background: var(--surface-highlight);
  }
  .review-summary-dropdown-item-success {
    color: var(--color-success);
  }
  .review-summary-dropdown-item-danger {
    color: var(--color-danger);
  }
  .review-summary-dropdown-note {
    padding: 8px 10px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-muted);
  }
  .review-submit-backdrop {
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
  .review-submit-modal {
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
  .review-submit-kicker {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .review-submit-modal h2 {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  .review-submit-modal p {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-secondary);
  }
  .review-submit-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .review-submit-field span {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .review-submit-textarea {
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
  .review-submit-textarea:focus {
    border-color: var(--accent);
  }
  .review-submit-error {
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--color-danger) 35%, var(--border-default));
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    color: var(--color-danger);
    font-size: 12px;
  }
  .review-submit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .review-submit-thinking-arrow {
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
