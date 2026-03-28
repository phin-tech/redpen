<script lang="ts">
  import {
    getReviewPageState,
    closeReviewPage,
    nextCard,
    prevCard,
    getTotalCards,
    getResolvedCount,
    getCardAtIndex,
  } from "$lib/stores/reviewPage.svelte";
  import { resolveAnnotation, updateChoices, addAnnotation } from "$lib/stores/annotations.svelte";
  import ReviewCard from "./ReviewCard.svelte";
  import type { Annotation, DiffHunk } from "$lib/types";

  let {
    onJumpToFile,
  }: {
    onJumpToFile: (filePath: string, line: number) => void;
  } = $props();

  const state = getReviewPageState();

  const modeLabel = $derived(
    state.mode === "changes" ? "Review Changes" : "Agent Feedback"
  );

  const totalCards = $derived(getTotalCards());
  const resolvedCount = $derived(getResolvedCount());
  const progressPercent = $derived(
    totalCards > 0 ? Math.round((resolvedCount / totalCards) * 100) : 0
  );

  // Build flat card entries grouped by file
  interface CardEntry {
    annotation: Annotation;
    replies: Annotation[];
    filePath: string;
    flatIndex: number;
  }

  const cardEntries = $derived.by((): { filePath: string; fileName: string; cards: CardEntry[]; jumpLine: number }[] => {
    const result: { filePath: string; fileName: string; cards: CardEntry[]; jumpLine: number }[] = [];
    let flatIndex = 0;

    for (const file of state.files) {
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

      if (cards.length > 0) {
        const jumpLine = roots[0]?.anchor.range.startLine ?? 1;
        result.push({ filePath: file.filePath, fileName: file.fileName, cards, jumpLine });
      }
    }

    return result;
  });

  function handleKeydown(e: KeyboardEvent) {
    // Don't intercept when typing in an input
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      if (e.key === "Escape") {
        (e.target as HTMLElement).blur();
        e.preventDefault();
      }
      return;
    }

    if (e.key === "j") {
      e.preventDefault();
      nextCard();
      scrollToActive();
    } else if (e.key === "k") {
      e.preventDefault();
      prevCard();
      scrollToActive();
    } else if (e.key === "r") {
      e.preventDefault();
      // Focus the reply input in the active card
      const el = document.querySelector(`[data-review-card="${state.activeCardIndex}"] .review-card-reply-input`) as HTMLInputElement | null;
      el?.focus();
    } else if (e.key === "e") {
      e.preventDefault();
      const card = getCardAtIndex(state.activeCardIndex);
      if (card) {
        handleResolve(card.annotation.id, !card.annotation.resolved);
      }
    } else if (e.key === "o") {
      e.preventDefault();
      const card = getCardAtIndex(state.activeCardIndex);
      if (card) {
        handleJumpToFile(card.filePath, card.annotation.anchor.range.startLine);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeReviewPage();
    }
  }

  function scrollToActive() {
    requestAnimationFrame(() => {
      const el = document.querySelector(`[data-review-card="${state.activeCardIndex}"]`);
      el?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
  }

  function handleJumpToFile(filePath: string, line: number) {
    closeReviewPage();
    onJumpToFile(filePath, line);
  }

  async function handleReply(annotationId: string, body: string) {
    for (const file of state.files) {
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
    for (const file of state.files) {
      const ann = file.annotations.find((a) => a.id === annotationId);
      if (ann) {
        await resolveAnnotation(file.filePath, annotationId, resolved);
        ann.resolved = resolved;
        break;
      }
    }
  }

  async function handleChoiceToggle(annotationId: string, choiceIndex: number) {
    for (const file of state.files) {
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

  function findHunkForLine(file: typeof state.files[0], line: number): DiffHunk | null {
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
    <span class="review-topbar-count">{totalCards} annotations</span>
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

  {#if state.loading}
    <div class="review-status">Loading...</div>
  {:else if state.error}
    <div class="review-status">
      <p>{state.error}</p>
      <button class="review-close-btn" onclick={closeReviewPage}>Close</button>
    </div>
  {:else if cardEntries.length === 0}
    <div class="review-status">
      <p>No annotations to review</p>
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

        {#each fileGroup.cards as entry (entry.annotation.id)}
          <div data-review-card={entry.flatIndex}>
            <ReviewCard
              annotation={entry.annotation}
              replies={entry.replies}
              filePath={entry.filePath}
              snippet={state.files.find(f => f.filePath === entry.filePath)?.snippets.get(entry.annotation.id) ?? null}
              diffHunk={findHunkForLine(
                state.files.find(f => f.filePath === entry.filePath)!,
                entry.annotation.anchor.range.startLine
              )}
              isActive={state.activeCardIndex === entry.flatIndex}
              onReply={handleReply}
              onResolve={handleResolve}
              onJumpToFile={handleJumpToFile}
              onChoiceToggle={handleChoiceToggle}
            />
          </div>
        {/each}
      {/each}
    </div>
  {/if}

  <!-- Keyboard hint bar -->
  <div class="review-nav-hint">
    <span><kbd>j</kbd> / <kbd>k</kbd> next / prev</span>
    <span><kbd>r</kbd> reply</span>
    <span><kbd>e</kbd> resolve</span>
    <span><kbd>o</kbd> open in editor</span>
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
