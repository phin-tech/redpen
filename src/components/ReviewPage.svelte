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
  import { resolveAnnotation, updateChoices, addAnnotation, removeAnnotation } from "$lib/stores/annotations.svelte";
  import ReviewCard from "./ReviewCard.svelte";
  import ReviewCodeSnippet from "./ReviewCodeSnippet.svelte";
  import type { Annotation, DiffHunk } from "$lib/types";

  let {
    onJumpToFile,
    showShortcutHelp = $bindable(false),
  }: {
    onJumpToFile: (filePath: string, line: number) => void;
    showShortcutHelp?: boolean;
  } = $props();

  const reviewState = getReviewPageState();
  const activeIndex = $derived(reviewState.activeCardIndex);

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

  interface ShortcutHelpSection {
    title: string;
    items: { keys: string[]; label: string }[];
  }

  const shortcutHelpSections: ShortcutHelpSection[] = [
    {
      title: "Navigation",
      items: [
        { keys: ["j", "k"], label: "next / previous card" },
        { keys: ["gg", "G"], label: "jump to top / bottom" },
        { keys: ["o"], label: "open current file in editor" },
        { keys: ["s"], label: "toggle review scope" },
      ],
    },
    {
      title: "Snippet",
      items: [
        { keys: ["h", "l"], label: "expand above / below" },
        { keys: ["H", "L"], label: "contract above / below" },
        { keys: ["R"], label: "reset snippet context" },
      ],
    },
    {
      title: "Actions",
      items: [
        { keys: ["r"], label: "reply to current thread" },
        { keys: ["e"], label: "resolve / unresolve current item" },
        { keys: ["1-9"], label: "toggle a choice" },
        { keys: ["dd"], label: "delete current annotation" },
      ],
    },
    {
      title: "UI",
      items: [
        { keys: ["[", "]"], label: "switch code / review view" },
        { keys: ["?"], label: "toggle this help" },
        { keys: ["Esc"], label: "close help or exit review" },
      ],
    },
  ];

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
  let pendingDelete = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    // Don't intercept when typing in an input
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }

    if (e.key === "?") {
      e.preventDefault();
      e.stopPropagation();
      showShortcutHelp = !showShortcutHelp;
      pendingG = false;
      pendingDelete = false;
      return;
    }

    if (showShortcutHelp) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        showShortcutHelp = false;
      }
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
    if (e.key !== "d") pendingDelete = false;

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
    } else if (e.key === "d") {
      e.preventDefault();
      const card = getCardAtIndex(activeIndex);
      if (card?.annotation) {
        if (pendingDelete) {
          handleDismiss(card.filePath, card.annotation.id);
          pendingDelete = false;
        } else {
          pendingDelete = true;
          setTimeout(() => { pendingDelete = false; }, 2000);
        }
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
      if (card?.annotation?.choices) {
        const idx = parseInt(e.key) - 1;
        if (idx < card.annotation.choices.length) {
          handleChoiceToggle(card.annotation.id, idx);
        }
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (pendingDelete) {
        pendingDelete = false;
      } else {
        closeReviewPage();
      }
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

  async function handleDismiss(filePath: string, annotationId: string) {
    await removeAnnotation(filePath, annotationId);
    // Remove from review state
    for (const file of reviewState.files) {
      if (file.filePath === filePath) {
        file.annotations = file.annotations.filter((a) => a.id !== annotationId);
        break;
      }
    }
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

<svelte:window onkeydowncapture={handleKeydown} />

<div class="review-page">
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
            <div data-review-card={entry.flatIndex} class="review-card-wrapper">
              {#if pendingDelete && activeIndex === entry.flatIndex}
                <div class="review-delete-confirm"><span>Press <kbd>d</kbd> again to delete</span></div>
              {/if}
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

  {#if showShortcutHelp}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="review-help-overlay" onclick={() => (showShortcutHelp = false)}>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="review-help-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Review keyboard shortcuts"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="review-help-header">
          <div>
            <h2>Review shortcuts</h2>
            <p>Scoped to this review screen. The existing Vim-style keys stay unchanged.</p>
          </div>
          <button class="review-help-close" onclick={() => (showShortcutHelp = false)} aria-label="Close keyboard shortcut help">
            ×
          </button>
        </div>

        <div class="review-help-grid">
          {#each shortcutHelpSections as section}
            <section class="review-help-section">
              <h3>{section.title}</h3>
              <div class="review-help-list">
                {#each section.items as item}
                  <div class="review-help-row">
                    <div class="review-help-keys">
                      {#each item.keys as key}
                        <kbd>{key}</kbd>
                      {/each}
                    </div>
                    <span>{item.label}</span>
                  </div>
                {/each}
              </div>
            </section>
          {/each}
        </div>
      </div>
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
    <span><kbd>dd</kbd> delete</span>
    <span><kbd>o</kbd> open</span>
    <span><kbd>?</kbd> help</span>
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
  .review-card-wrapper {
    position: relative;
  }
  .review-delete-confirm {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(2px);
    border-radius: 8px;
    z-index: 3;
    pointer-events: none;
  }
  .review-delete-confirm span {
    background: var(--surface-raised);
    color: var(--color-danger);
    font-size: 13px;
    font-weight: 500;
    padding: 8px 20px;
    border-radius: 6px;
    border: 1px solid color-mix(in srgb, var(--color-danger) 30%, var(--border-default));
  }
  .review-delete-confirm kbd {
    background: color-mix(in srgb, var(--color-danger) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-danger) 30%, transparent);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: inherit;
    font-size: 12px;
    color: var(--color-danger);
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
  .review-help-overlay {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.62);
    backdrop-filter: blur(3px);
  }
  .review-help-modal {
    width: min(760px, 100%);
    max-height: min(560px, calc(100vh - 80px));
    overflow: auto;
    background: var(--gradient-panel), var(--surface-panel);
    border: 1px solid var(--border-emphasis);
    border-radius: 14px;
    box-shadow: var(--shadow-popover);
    padding: 20px;
  }
  .review-help-header {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    justify-content: space-between;
    margin-bottom: 18px;
  }
  .review-help-header h2 {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  .review-help-header p {
    margin: 6px 0 0;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .review-help-close {
    width: 30px;
    height: 30px;
    border-radius: 999px;
    border: 1px solid var(--border-default);
    background: var(--surface-raised);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    flex-shrink: 0;
  }
  .review-help-close:hover {
    color: var(--text-primary);
    border-color: var(--border-emphasis);
  }
  .review-help-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
  }
  .review-help-section {
    background: color-mix(in srgb, var(--surface-panel) 80%, var(--surface-raised));
    border: 1px solid var(--border-default);
    border-radius: 10px;
    padding: 14px;
  }
  .review-help-section h3 {
    margin: 0 0 10px;
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .review-help-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .review-help-row {
    display: grid;
    grid-template-columns: minmax(92px, auto) 1fr;
    gap: 10px;
    align-items: start;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .review-help-keys {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .review-help-row kbd {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    padding: 1px 6px;
    font-family: inherit;
    font-size: 11px;
    color: var(--text-primary);
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
