<script lang="ts">
  let {
    isGitHubReview,
    linkText,
    meta,
    title,
    titleLabel,
    onOpenPullRequest,
  }: {
    isGitHubReview: boolean;
    linkText: string;
    meta: string;
    title: string;
    titleLabel: string;
    onOpenPullRequest: () => Promise<void>;
  } = $props();
</script>

<div class="review-summary-main">
  <div class="review-summary-line">
    <div class="review-summary-kicker">{titleLabel}</div>
    {#if isGitHubReview}
      <button
        type="button"
        class="review-summary-link"
        onclick={() => void onOpenPullRequest()}
      >
        {linkText}
      </button>
    {/if}
  </div>

  {#if isGitHubReview}
    <div class="review-summary-line">
      <div class="review-summary-title">{title}</div>
      <div class="review-summary-meta">{meta}</div>
    </div>
  {:else}
    <div class="review-summary-line">
      <div class="review-summary-title">{meta}</div>
    </div>
  {/if}
</div>
