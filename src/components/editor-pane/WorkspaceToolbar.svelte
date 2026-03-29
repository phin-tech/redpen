<script lang="ts">
  import BubbleKindFilter from "../BubbleKindFilter.svelte";
  import DiffModeToggle from "../DiffModeToggle.svelte";
  import DiffRefPicker from "../DiffRefPicker.svelte";
  import { getEditor, getShowPreview, isMarkdownFile, togglePreview } from "$lib/stores/editor.svelte";
  import { getDiffState } from "$lib/stores/diff.svelte";
  import {
    getAnnotationCount,
    getResolvedCount,
    getReviewPageState,
    isReviewPageOpen,
    toggleScope,
  } from "$lib/stores/reviewPage.svelte";
  import { getReviewSession } from "$lib/stores/review.svelte";
  import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
  import { getBubblesEnabled, toggleBubbles } from "$lib/stores/annotations.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";

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
  const reviewScopeLabel = $derived(
    reviewState.scope === "all-changes" ? "All Changes" : "Session",
  );
  const reviewAnnotationCount = $derived(getAnnotationCount());
  const reviewResolvedCount = $derived(getResolvedCount());
  const reviewTabBadge = $derived(isReviewPageOpen() ? reviewAnnotationCount : null);
</script>

{#if editor.currentFilePath}
  <div class="workspace-toolbar">
    <div class="toolbar-section">
      <div class="toolbar-label">Workspace</div>
      <div class="view-tabs">
        <button
          class="toggle-btn view-tab"
          class:active={!isReviewPageOpen() && !showPrView}
          onclick={onSelectCodeView}
        >
          Code
        </button>
        <button
          class="toggle-btn view-tab view-tab-review"
          class:active={isReviewPageOpen() && !showPrView}
          onclick={onSelectReviewView}
        >
          Review
          <span class="view-tab-badge" class:view-tab-badge-hidden={reviewTabBadge === null}>
            {reviewTabBadge ?? ""}
          </span>
        </button>
        {#if githubReview.activeSession}
          <button
            class="toggle-btn view-tab"
            class:active={showPrView}
            onclick={onSelectPrView}
          >
            PR
          </button>
        {/if}
      </div>
    </div>

    <div class="toolbar-divider"></div>

    {#if isReviewPageOpen()}
      <div class="toolbar-section toolbar-section-stretch">
        <div class="toolbar-label">Review</div>
        <div class="review-toolbar">
          {#if reviewState.mode === "changes"}
            <button class="toggle-btn review-toolbar-btn" onclick={toggleScope}>
              {reviewScopeLabel}
              <kbd>s</kbd>
            </button>
          {/if}
          <span class="review-toolbar-meta">
            {reviewState.files.length} files · {reviewAnnotationCount} annotations
          </span>
          {#if reviewResolvedCount > 0}
            <span class="review-toolbar-meta review-toolbar-meta-success">
              {reviewResolvedCount} resolved
            </span>
          {/if}
          {#if reviewSession.active && !githubReview.activeSession}
            <button
              class="toggle-btn review-toolbar-btn review-toolbar-btn-success"
              onclick={() => void onAgentReviewVerdict("approved")}
            >
              Approve
            </button>
            <button
              class="toggle-btn review-toolbar-btn review-toolbar-btn-danger"
              onclick={() => void onAgentReviewVerdict("changes_requested")}
            >
              Request changes
            </button>
          {/if}
        </div>
      </div>
    {:else if showPrView && githubReview.activeSession}
      <div class="toolbar-section toolbar-section-stretch">
        <div class="toolbar-label">Overview</div>
        <div class="review-toolbar">
          <span class="review-toolbar-meta">PR overview</span>
        </div>
      </div>
    {:else}
      <div class="toolbar-section toolbar-section-stretch">
        <div class="toolbar-label">Code View</div>
        <div class="review-toolbar">
          <DiffModeToggle onEnterDiff={onEnterDiff} />
          {#if diff.enabled}
            <DiffRefPicker {directory} filePath={editor.currentFilePath ?? ""} />
          {/if}

          <div class="toolbar-divider toolbar-divider-inline"></div>

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
            <div class="toolbar-divider toolbar-divider-inline"></div>
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

          {#if reviewSession.active && !githubReview.activeSession}
            <div class="toolbar-divider toolbar-divider-inline"></div>
            <button
              class="toggle-btn review-toolbar-btn review-toolbar-btn-success"
              onclick={() => void onAgentReviewVerdict("approved")}
            >
              Approve
            </button>
            <button
              class="toggle-btn review-toolbar-btn review-toolbar-btn-danger"
              onclick={() => void onAgentReviewVerdict("changes_requested")}
            >
              Request changes
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .workspace-toolbar {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 10px;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--surface-panel) 92%, white 8%), transparent 100%),
      var(--gradient-toolbar),
      var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .toolbar-section {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .toolbar-section-stretch {
    flex: 1;
  }
  .toolbar-label {
    color: var(--text-muted);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .toolbar-divider {
    width: 1px;
    align-self: stretch;
    background: color-mix(in srgb, var(--border-default) 75%, transparent);
  }
  .toolbar-divider-inline {
    align-self: center;
    height: 16px;
  }
  .view-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    flex-shrink: 0;
  }
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
  .view-tab {
    min-width: 76px;
    justify-content: center;
    flex: 0 0 auto;
  }
  .view-tab-review {
    min-width: 96px;
  }
  .view-tab-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--surface-base);
    border: 1px solid color-mix(in srgb, currentColor 18%, var(--border-default));
    color: inherit;
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
  }
  .view-tab-badge-hidden {
    visibility: hidden;
  }
  .review-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
    flex-wrap: wrap;
  }
  .review-toolbar-meta {
    display: inline-flex;
    align-items: center;
    min-height: 30px;
    padding: 4px 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    white-space: nowrap;
  }
  .review-toolbar-meta-success {
    color: var(--color-success);
  }
  .review-toolbar-btn {
    border-color: var(--border-default);
    background: var(--surface-raised);
    color: var(--text-secondary);
  }
  .review-toolbar-btn:hover {
    border-color: var(--border-emphasis);
  }
  .review-toolbar-btn-success {
    color: var(--color-success);
    border-color: color-mix(in srgb, var(--color-success) 35%, var(--border-default));
    background: color-mix(in srgb, var(--color-success) 14%, transparent);
  }
  .review-toolbar-btn-danger {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-default));
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
  }
  .review-toolbar-btn kbd {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
