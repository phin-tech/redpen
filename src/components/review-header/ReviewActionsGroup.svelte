<script lang="ts">
  import type { SubmitGitHubReviewResult } from "$lib/types";
  import ReviewSubmitControl from "./ReviewSubmitControl.svelte";

  let {
    githubReviewActive,
    isSelfAuthoredPr,
    onDiscardPending,
    onLocalApprove,
    onLocalRequestChanges,
    onResync,
    onSubmitReview,
    reviewSessionActive,
  }: {
    githubReviewActive: boolean;
    isSelfAuthoredPr: boolean;
    onDiscardPending: () => Promise<void>;
    onLocalApprove: () => Promise<void>;
    onLocalRequestChanges: () => Promise<void>;
    onResync: () => Promise<void>;
    onSubmitReview: (
      action: "comment" | "approve" | "requestChanges",
      message?: string,
    ) => Promise<SubmitGitHubReviewResult | null>;
    reviewSessionActive: boolean;
  } = $props();
</script>

{#if githubReviewActive}
  <div class="review-summary-actions">
    <button type="button" class="review-summary-btn" onclick={() => void onResync()}>
      Resync
    </button>
    <button type="button" class="review-summary-btn" onclick={() => void onDiscardPending()}>
      Discard pending
    </button>
    <ReviewSubmitControl {isSelfAuthoredPr} {onSubmitReview} />
  </div>
{:else if reviewSessionActive}
  <div class="review-summary-actions">
    <button
      type="button"
      class="review-summary-btn review-summary-btn-success"
      onclick={() => void onLocalApprove()}
    >
      Approve
    </button>
    <button
      type="button"
      class="review-summary-btn review-summary-btn-danger"
      onclick={() => void onLocalRequestChanges()}
    >
      Request changes
    </button>
  </div>
{/if}
