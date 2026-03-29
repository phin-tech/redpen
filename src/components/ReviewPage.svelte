<script lang="ts">
  import {
    getReviewPageState,
    closeReviewPage,
    toggleScope,
    setActiveCard,
    nextCard,
    prevCard,
    getTotalItems,
    getAnnotationCount,
    getResolvedCount,
    getCardAtIndex,
  } from "$lib/stores/reviewPage.svelte";
  import { resolveAnnotation, updateChoices, addAnnotation } from "$lib/stores/annotations.svelte";
  import ReviewCard from "./ReviewCard.svelte";
  import ReviewCodeSnippet from "./ReviewCodeSnippet.svelte";
  import type { Annotation, DiffHunk } from "$lib/types";

  let {
    onJumpToFile,
  }: {
    onJumpToFile: (filePath: string, line: number) => void;
  } = $props();

  const reviewState = getReviewPageState();
  const activeIndex = $derived(reviewState.activeCardIndex);

  const modeLabel = $derived(
    reviewState.mode === "changes" ? "Review Changes" : "Agent Feedback"
  );
  const scopeLabel = $derived(
    reviewState.scope === "all-changes" ? "All Changes" : "Session"
  );

  const totalItems = $derived(getTotalItems());
  const annotationCount = $derived(getAnnotationCount());
  const resolvedCount = $derived(getResolvedCount());
  const progressPercent = $derived(
    annotationCount > 0 ? Math.round((resolvedCount / annotationCount) * 100) : 0
  );

  // Build flat card entries grouped by file
  interface CardEntry {
    annotation: Annotation;
    replies: Annotation[];
    filePath: string;
    flatIndex: number;
  }

  const cardEntries = $derived.by(() => {
    const result: { filePath: string; fileName: string; cards: CardEntry[]; jumpLine: number; diffOnlyIndex: number | null }[] = [];
    let flatIndex = 0;

    for (const file of reviewState.files) {
      const roots = file.annotations.filter((a) => !a.replyTo);
      const replyMap = new Map<string, Annotation[]>();
      for (const a of file.annotations) {
        if (a.replyTo) {
          const group = replyMap.get(a.replyTo) ?? [];
          group.push(a);
          replyMap.set(a.replyTo, group);
        }
      }

      const cards: CardEntry[] = [];
      for (const root of roots) {
        cards.push({
          annotation: root,
          replies: replyMap.get(root.id) ?? [],
          filePath: file.filePath,
          flatIndex,
        });
        flatIndex++;
      }

      const jumpLine = roots[0]?.anchor.range.startLine ?? file.diff?.hunks[0]?.newStart ?? 1;
      if (cards.length > 0 || file.diff) {
        const diffOnlyIndex = cards.length === 0 && file.diff ? flatIndex : null;
        if (diffOnlyIndex !== null) flatIndex++;
        result.push({ filePath: file.filePath, fileName: file.fileName, cards, jumpLine, diffOnlyIndex });
      }
    }

    return result;
  });

  let pendingG = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    // Don't intercept when typing in an input
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }

    if (e.key === "g") {
      e.preventDefault();
      if (pendingG) {
        setActiveCard(0);
        scrollToActive();
        pendingG = false;
      } else {
        pendingG = true;
        setTimeout(() => { pendingG = false; }, 500);
      }
      return;
    }
    pendingG = false;

    if (e.key === "G") {
      e.preventDefault();
      setActiveCard(totalItems - 1);
      scrollToActive();
    } else if (e.key === "j") {
      e.preventDefault();
      nextCard();
      scrollToActive();
    } else if (e.key === "k") {
      e.preventDefault();
      prevCard();
      scrollToActive();
    } else if (e.key === "s") {
      e.preventDefault();
      toggleScope();
    } else if (e.key === "r") {
      e.preventDefault();
      // Focus the reply input in the active card
      const el = document.querySelector(`[data-review-card="${activeIndex}"] .review-card-reply-input`) as HTMLInputElement | null;
      el?.focus();
    } else if (e.key === "e") {
      e.preventDefault();
      const card = getCardAtIndex(activeIndex);
      if (card?.annotation) {
        handleResolve(card.annotation.id, !card.annotation.resolved);
      }
    } else if (e.key === "o") {
      e.preventDefault();
      const card = getCardAtIndex(activeIndex);
      if (card) {
        const line = card.annotation?.anchor.range.startLine ?? 1;
        handleJumpToFile(card.filePath, line);
      }
    } else if (e.key === "h") {
      e.preventDefault();
      const btn = document.querySelector(`[data-review-card="${activeIndex}"] .snippet-expand-above`) as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "l") {
      e.preventDefault();
      const btn = document.querySelector(`[data-review-card="${activeIndex}"] .snippet-expand-below`) as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "H") {
      e.preventDefault();
      const btn = document.querySelector(`[data-review-card="${activeIndex}"] .snippet-contract-above`) as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "L") {
      e.preventDefault();
      const btn = document.querySelector(`[data-review-card="${activeIndex}"] .snippet-contract-below`) as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "R") {
      e.preventDefault();
      const btn = document.querySelector(`[data-review-card="${activeIndex}"] .snippet-reset`) as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key >= "1" && e.key <= "9") {
      e.preventDefault();
      const card = getCardAtIndex(activeIndex);
      if (card && card.annotation.choices) {
        const idx = parseInt(e.key) - 1;
        if (idx < card.annotation.choices.length) {
          handleChoiceToggle(card.annotation.id, idx);
        }
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeReviewPage();
    }
  }

  function scrollToActive() {
    requestAnimationFrame(() => {
      const el = document.querySelector(`[data-review-card="${activeIndex}"]`);
      el?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
  }

  function handleJumpToFile(filePath: string, line: number) {
    closeReviewPage();
    onJumpToFile(filePath, line);
  }

  async function handleReply(annotationId: string, body: string) {
    for (const file of reviewState.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann) {
        await addAnnotation(
          file.filePath,
          body,
          [],
          ann.anchor.range.startLine,
          ann.anchor.range.startColumn,
          ann.anchor.range.endLine,
          ann.anchor.range.endColumn,
        );
        break;
      }
    }
  }

  async function handleResolve(annotationId: string, resolved: boolean) {
    for (const file of reviewState.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann) {
        await resolveAnnotation(file.filePath, annotationId, resolved);
        ann.resolved = resolved;
        break;
      }
    }
  }

  async function handleChoiceToggle(annotationId: string, choiceIndex: number) {
    for (const file of reviewState.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann && ann.choices) {
        const newChoices = ann.choices.map((c, i) => {
          if (ann.selectionMode === "single") {
            return { ...c, selected: i === choiceIndex };
          }
          return i === choiceIndex ? { ...c, selected: !c.selected } : c;
        });
        await updateChoices(file.filePath, annotationId, newChoices);
        ann.choices = newChoices;
        break;
      }
    }
  }

  function findHunkForLine(file: typeof reviewState.files[0], line: number): DiffHunk | null {
    if (!file.diff) return null;
    for (const hunk of file.diff.hunks) {
      const hunkEnd = hunk.newStart + hunk.newCount - 1;
      if (line >= hunk.newStart && line <= hunkEnd) {
        return hunk;
      }
    }
    return null;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="review-page">
  <!-- Top bar -->
  <div class="review-topbar">
    <span class="review-topbar-title">{modeLabel}</span>
    {#if reviewState.mode === "changes"}
      <button class="review-topbar-scope" onclick={toggleScope}>
        {scopeLabel} <kbd>s</kbd>
      </button>
    {/if}
    <span class="review-topbar-count">{reviewState.files.length} files · {annotationCount} annotations</span>
    {#if resolvedCount > 0}
      <span class="review-topbar-resolved">{resolvedCount} resolved</span>
    {/if}
    <span class="review-topbar-spacer"></span>
    <button class="review-topbar-close" onclick={closeReviewPage}>
      Close <kbd>Esc</kbd>
    </button>
  </div>

  <!-- Progress bar -->
  <div class="review-progress">
    <div class="review-progress-fill" style:width="{progressPercent}%"></div>
  </div>

  {#if reviewState.loading}
    <div class="review-status">Loading...</div>
  {:else if reviewState.error}
    <div class="review-status">
      <p>{reviewState.error}</p>
      <button class="review-close-btn" onclick={closeReviewPage}>Close</button>
    </div>
  {:else if cardEntries.length === 0 && reviewState.files.length === 0}
    <div class="review-status">
      <p>{reviewState.scope === "all-changes" ? "No changed files" : "No annotations to review"}</p>
      <button class="review-close-btn" onclick={closeReviewPage}>Close</button>
    </div>
  {:else}
    <div class="review-feed">
      {#each cardEntries as fileGroup}
        <div class="review-file-header">
          <span class="review-file-icon">📄</span>
          <span class="review-file-name">{fileGroup.fileName}</span>
          <span class="review-file-count">{fileGroup.cards.length}</span>
          <button
            class="review-file-jump"
            onclick={() => handleJumpToFile(fileGroup.filePath, fileGroup.jumpLine)}
          >
            Open in editor →
          </button>
        </div>

        {#if fileGroup.cards.length > 0}
          {#each fileGroup.cards as entry (entry.annotation.id)}
            <div data-review-card={entry.flatIndex}>
              <ReviewCard
                annotation={entry.annotation}
                replies={entry.replies}
                filePath={entry.filePath}
                snippet={reviewState.files.find(f => f.filePath === entry.filePath)?.snippets.get(entry.annotation.id) ?? null}
                diffHunk={findHunkForLine(
                  reviewState.files.find(f => f.filePath === entry.filePath)!,
                  entry.annotation.anchor.range.startLine
                )}
                isActive={activeIndex === entry.flatIndex}
                onReply={handleReply}
                onResolve={handleResolve}
                onJumpToFile={handleJumpToFile}
                onChoiceToggle={handleChoiceToggle}
              />
            </div>
          {/each}
        {:else}
          {@const fileData = reviewState.files.find(f => f.filePath === fileGroup.filePath)}
          {#if fileData?.diff}
            <div data-review-card={fileGroup.diffOnlyIndex} class="review-diff-group" class:review-card-active={activeIndex === fileGroup.diffOnlyIndex}>
            {#each fileData.diff.hunks as hunk}
              <div class="review-diff-hunk">
                <ReviewCodeSnippet
                  filePath={fileGroup.filePath}
                  snippet={null}
                  highlightLine={0}
                  diffHunk={hunk}
                />
              </div>
            {/each}
            </div>
          {/if}
        {/if}
      {/each}
    </div>
  {/if}

  <!-- Keyboard hint bar -->
  <div class="review-nav-hint">
    <span><kbd>j</kbd><kbd>k</kbd> nav</span>
    <span><kbd>gg</kbd><kbd>G</kbd> top/bottom</span>
    <span><kbd>h</kbd><kbd>l</kbd> expand</span>
    <span><kbd>H</kbd><kbd>L</kbd> contract</span>
    <span><kbd>R</kbd> reset</span>
    <span><kbd>1-9</kbd> choice</span>
    <span><kbd>s</kbd> scope</span>
    <span><kbd>r</kbd> reply</span>
    <span><kbd>e</kbd> resolve</span>
    <span><kbd>o</kbd> open</span>
    <span><kbd>Esc</kbd> close</span>
  </div>
</div>

<style>
  .review-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
    overflow: hidden;
  }
  .review-topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 20px;
    background: var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    flex-shrink: 0;
    z-index: 2;
  }
  .review-topbar-title {
    font-weight: 600;
    font-size: 15px;
    color: var(--text-primary);
  }
  .review-topbar-scope {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    border-radius: 6px;
    padding: 2px 10px;
    font-size: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: inherit;
  }
  .review-topbar-scope:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }
  .review-topbar-scope kbd {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 10px;
  }
  .review-topbar-count {
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--surface-raised);
    padding: 2px 8px;
    border-radius: 10px;
  }
  .review-topbar-resolved {
    font-size: 12px;
    color: var(--success);
  }
  .review-topbar-spacer {
    flex: 1;
  }
  .review-topbar-close {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    border-radius: 6px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .review-topbar-close:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }
  .review-topbar-close kbd {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 10px;
  }
  .review-progress {
    height: 3px;
    background: var(--surface-raised);
    flex-shrink: 0;
  }
  .review-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
    border-radius: 0 2px 2px 0;
  }
  .review-feed {
    flex: 1;
    overflow-y: auto;
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
    padding: 12px 24px 100px;
  }
  .review-file-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 0 8px;
    font-size: 12px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-default);
    margin-bottom: 8px;
  }
  .review-file-header:first-child {
    padding-top: 4px;
  }
  .review-file-icon {
    opacity: 0.5;
  }
  .review-file-name {
    color: var(--text-secondary);
    font-weight: 500;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 12px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .review-file-count {
    background: var(--surface-raised);
    padding: 1px 7px;
    border-radius: 8px;
    font-variant-numeric: tabular-nums;
  }
  .review-file-jump {
    color: var(--accent);
    font-size: 11px;
    cursor: pointer;
    border: none;
    background: none;
    font-family: inherit;
  }
  .review-diff-group {
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--border-default);
    margin: 8px 0;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .review-diff-group.review-card-active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent), 0 2px 12px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .review-diff-hunk {
    border-top: 1px solid var(--border-default);
  }
  .review-diff-hunk:first-child {
    border-top: none;
  }
  .review-file-jump:hover {
    text-decoration: underline;
  }
  .review-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 12px;
    flex: 1;
  }
  .review-close-btn {
    padding: 6px 16px;
    border-radius: 6px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 13px;
  }
  .review-nav-hint {
    position: sticky;
    bottom: 0;
    background: var(--surface-panel);
    border-top: 1px solid var(--border-default);
    padding: 8px 16px;
    font-size: 12px;
    color: var(--text-muted);
    display: flex;
    justify-content: center;
    gap: 16px;
    flex-shrink: 0;
    z-index: 2;
  }
  .review-nav-hint kbd {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: inherit;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
