<script lang="ts">
  import type { SubmitGitHubReviewResult } from "$lib/types";

  let {
    isSelfAuthoredPr,
    onSubmitReview,
  }: {
    isSelfAuthoredPr: boolean;
    onSubmitReview: (
      action: "comment" | "approve" | "requestChanges",
      message?: string,
    ) => Promise<SubmitGitHubReviewResult | null>;
  } = $props();

  let showSubmitMenu = $state(false);
  let submitModalAction = $state<"comment" | "approve" | "requestChanges" | null>(null);
  let submitTextareaRef: HTMLTextAreaElement | undefined = $state(undefined);
  let submitModalMessage = $state("");
  let submitModalStatus = $state<"editing" | "submitting" | "success" | "error">("editing");
  let submitModalError = $state<string | null>(null);
  let submitModalResult = $state<SubmitGitHubReviewResult | null>(null);

  function openSubmitModal(action: "comment" | "approve" | "requestChanges") {
    showSubmitMenu = false;
    submitModalAction = action;
    submitModalMessage = "";
    submitModalStatus = "editing";
    submitModalError = null;
    submitModalResult = null;
    requestAnimationFrame(() => submitTextareaRef?.focus());
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
      submitModalResult = await onSubmitReview(
        submitModalAction,
        submitModalMessage.trim() || undefined,
      );
      submitModalStatus = "success";
    } catch (error) {
      submitModalError = error instanceof Error ? error.message : String(error);
      submitModalStatus = "error";
    }
  }

  function handleWindowClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (!target?.closest(".review-header-submit-menu")) {
      showSubmitMenu = false;
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && showSubmitMenu) {
      showSubmitMenu = false;
      return;
    }

    if (event.key === "Escape" && submitModalAction && submitModalStatus !== "submitting") {
      event.preventDefault();
      event.stopPropagation();
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
        <button
          class="review-summary-dropdown-item review-summary-dropdown-item-success"
          onclick={() => openSubmitModal("approve")}
        >
          Approve
        </button>
        <button
          class="review-summary-dropdown-item review-summary-dropdown-item-danger"
          onclick={() => openSubmitModal("requestChanges")}
        >
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
          <button
            type="button"
            class="review-summary-btn review-summary-btn-primary"
            onclick={closeSubmitModal}
          >
            Close
          </button>
        </div>
      {:else}
        <div class="review-submit-kicker">Submit review</div>
        <h2>{submitActionLabel(submitModalAction)}</h2>
        <p>{submitActionDescription(submitModalAction)}</p>

        <label class="review-submit-field">
          <span>Message</span>
          <!-- svelte-ignore a11y_autofocus -->
          <textarea
            bind:this={submitTextareaRef}
            class="review-submit-textarea"
            bind:value={submitModalMessage}
            placeholder="Optional summary for this review"
            rows="6"
            disabled={submitModalStatus === "submitting"}
            autofocus
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
