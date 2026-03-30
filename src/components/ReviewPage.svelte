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
  import type { Annotation, AnnotationKind, DiffHunk } from "$lib/types";
  import type { FileSnippet } from "$lib/tauri";
  import { Bot } from "lucide-svelte";

  const PROXIMITY_THRESHOLD = 5;

  const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

  const KIND_COLORS: Record<AnnotationKind, string> = {
    comment: "var(--kind-comment-border)",
    lineNote: "var(--kind-linenote-border)",
    explanation: "var(--kind-explanation-border)",
    label: "var(--kind-label-border)",
  };

  const KIND_LABELS: Record<AnnotationKind, string> = {
    comment: "comment",
    lineNote: "note",
    explanation: "explanation",
    label: "label",
  };

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

  // Build flat card entries grouped by file, with proximity grouping
  interface CardEntry {
    annotation: Annotation;
    replies: Annotation[];
    filePath: string;
    flatIndex: number;
  }

  interface ContextGroup {
    cards: CardEntry[];
    /** The combined snippet covering all annotations in this group */
    mergedSnippet: FileSnippet | null;
    /** Lines that have annotations, for status dots */
    annotatedLines: { line: number; resolved: boolean }[];
    /** The first card's startLine, for highlight range */
    rangeStart: number;
    /** The last card's endLine, for highlight range */
    rangeEnd: number;
    /** DiffHunk to show if available */
    diffHunk: DiffHunk | null;
  }

  interface FileGroupEntry {
    filePath: string;
    fileName: string;
    groups: ContextGroup[];
    jumpLine: number;
    diffOnlyIndex: number | null;
    cardCount: number;
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

  function mergeSnippets(snippets: (FileSnippet | null)[]): FileSnippet | null {
    const valid = snippets.filter((s): s is FileSnippet => s !== null);
    if (valid.length === 0) return null;
    if (valid.length === 1) return valid[0];

    let minStart = Infinity;
    let maxEnd = -Infinity;
    let totalLines = valid[0].totalLines;

    for (const s of valid) {
      const sEnd = s.startLine + s.lines.length - 1;
      if (s.startLine < minStart) minStart = s.startLine;
      if (sEnd > maxEnd) maxEnd = sEnd;
      totalLines = Math.max(totalLines, s.totalLines);
    }

    // Build merged lines array covering minStart..maxEnd
    const mergedLength = maxEnd - minStart + 1;
    const lines: string[] = new Array(mergedLength).fill("");
    for (const s of valid) {
      const offset = s.startLine - minStart;
      for (let i = 0; i < s.lines.length; i++) {
        lines[offset + i] = s.lines[i];
      }
    }

    return { startLine: minStart, lines, totalLines };
  }

  const cardEntries = $derived.by(() => {
    const result: FileGroupEntry[] = [];
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

      // Proximity grouping
      const groups: ContextGroup[] = [];
      if (cards.length > 0) {
        // Sort by startLine
        const sorted = [...cards].sort(
          (a, b) => a.annotation.anchor.range.startLine - b.annotation.anchor.range.startLine
        );

        let currentGroup: CardEntry[] = [sorted[0]];
        let groupEndLine = sorted[0].annotation.anchor.range.endLine ?? sorted[0].annotation.anchor.range.startLine;

        for (let i = 1; i < sorted.length; i++) {
          const card = sorted[i];
          const cardStart = card.annotation.anchor.range.startLine;
          if (cardStart - groupEndLine <= PROXIMITY_THRESHOLD) {
            currentGroup.push(card);
            const cardEnd = card.annotation.anchor.range.endLine ?? cardStart;
            if (cardEnd > groupEndLine) groupEndLine = cardEnd;
          } else {
            groups.push(buildContextGroup(currentGroup, file));
            currentGroup = [card];
            groupEndLine = card.annotation.anchor.range.endLine ?? cardStart;
          }
        }
        groups.push(buildContextGroup(currentGroup, file));
      }

      const jumpLine = roots[0]?.anchor.range.startLine ?? file.diff?.hunks[0]?.newStart ?? 1;
      if (cards.length > 0 || file.diff) {
        const diffOnlyIndex = cards.length === 0 && file.diff ? flatIndex : null;
        if (diffOnlyIndex !== null) flatIndex++;
        result.push({ filePath: file.filePath, fileName: file.fileName, groups, jumpLine, diffOnlyIndex, cardCount: cards.length });
      }
    }

    return result;
  });

  function buildContextGroup(cards: CardEntry[], file: typeof reviewState.files[0]): ContextGroup {
    const snippets = cards.map(c => file.snippets.get(c.annotation.id) ?? null);
    const mergedSnippet = mergeSnippets(snippets);

    const annotatedLines = cards.map(c => ({
      line: c.annotation.anchor.range.startLine,
      resolved: !!c.annotation.resolved,
    }));

    const rangeStart = Math.min(...cards.map(c => c.annotation.anchor.range.startLine));
    const rangeEnd = Math.max(...cards.map(c => c.annotation.anchor.range.endLine ?? c.annotation.anchor.range.startLine));

    // Find a diff hunk that covers this range
    let diffHunk: DiffHunk | null = null;
    if (file.diff) {
      for (const hunk of file.diff.hunks) {
        const hunkEnd = hunk.newStart + hunk.newCount - 1;
        if (rangeStart <= hunkEnd && rangeEnd >= hunk.newStart) {
          diffHunk = hunk;
          break;
        }
      }
    }

    return { cards, mergedSnippet, annotatedLines, rangeStart, rangeEnd, diffHunk };
  }

  // Reply state
  let replyingTo: string | null = $state(null);
  let replyText = $state("");
  let replyInputRef: HTMLInputElement | undefined = $state(undefined);

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
      const card = getCardAtIndex(activeIndex);
      if (card?.annotation) {
        replyingTo = card.annotation.id;
        replyText = "";
        // Focus will happen via autofocus on the input after render
        requestAnimationFrame(() => {
          const el = document.querySelector(`[data-review-card="${activeIndex}"] .review-thread-reply-input`) as HTMLInputElement | null;
          el?.focus();
        });
      }
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
      const activeEl = document.querySelector(`[data-review-card="${activeIndex}"]`);
      const block = activeEl?.closest('.review-context-block') ?? activeEl;
      const btn = block?.querySelector('.snippet-expand-above') as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "l") {
      e.preventDefault();
      const activeEl = document.querySelector(`[data-review-card="${activeIndex}"]`);
      const block = activeEl?.closest('.review-context-block') ?? activeEl;
      const btn = block?.querySelector('.snippet-expand-below') as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "H") {
      e.preventDefault();
      const activeEl = document.querySelector(`[data-review-card="${activeIndex}"]`);
      const block = activeEl?.closest('.review-context-block') ?? activeEl;
      const btn = block?.querySelector('.snippet-contract-above') as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "L") {
      e.preventDefault();
      const activeEl = document.querySelector(`[data-review-card="${activeIndex}"]`);
      const block = activeEl?.closest('.review-context-block') ?? activeEl;
      const btn = block?.querySelector('.snippet-contract-below') as HTMLButtonElement | null;
      btn?.click();
    } else if (e.key === "R") {
      e.preventDefault();
      const activeEl = document.querySelector(`[data-review-card="${activeIndex}"]`);
      const block = activeEl?.closest('.review-context-block') ?? activeEl;
      const btn = block?.querySelector('.snippet-reset') as HTMLButtonElement | null;
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
      if (replyingTo) {
        replyingTo = null;
        replyText = "";
      } else if (pendingDelete) {
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
          undefined,
          annotationId,
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

  function submitReply(annotationId: string) {
    if (!replyText.trim()) return;
    handleReply(annotationId, replyText.trim());
    replyText = "";
    replyingTo = null;
  }

  function handleReplyKeydown(e: KeyboardEvent, annotationId: string) {
    if (e.key === "Escape") {
      e.stopPropagation();
      replyingTo = null;
      replyText = "";
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submitReply(annotationId);
    }
    e.stopPropagation();
  }

  function relativeTime(dateStr: string | null | undefined): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  function handleLineTagEnter(e: MouseEvent, lineNum: number) {
    const block = (e.currentTarget as HTMLElement).closest('.review-context-block');
    block?.querySelector(`.snippet-line[data-line="${lineNum}"]`)?.classList.add('snippet-line-hover');
  }

  function handleLineTagLeave(e: MouseEvent, lineNum: number) {
    const block = (e.currentTarget as HTMLElement).closest('.review-context-block');
    block?.querySelector('.snippet-line-hover')?.classList.remove('snippet-line-hover');
  }

  function syncBadge(syncState: string | null | undefined): { label: string; className: string } | null {
    switch (syncState) {
      case "pendingPublish":
        return { label: "pending", className: "review-thread-sync-pending" };
      case "published":
      case "imported":
        return { label: "submitted", className: "review-thread-sync-submitted" };
      case "localOnly":
        return { label: "local only", className: "review-thread-sync-local" };
      case "conflict":
        return { label: "conflict", className: "review-thread-sync-conflict" };
      default:
        return null;
    }
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
          <span class="review-file-count">{fileGroup.cardCount}</span>
          <button
            class="review-file-jump"
            onclick={() => handleJumpToFile(fileGroup.filePath, fileGroup.jumpLine)}
          >
            Open in editor →
          </button>
        </div>

        {#if fileGroup.groups.length > 0}
          {#each fileGroup.groups as group}
            <div class="review-context-block">
              <ReviewCodeSnippet
                filePath={fileGroup.filePath}
                snippet={group.mergedSnippet}
                highlightLine={group.rangeStart}
                highlightEndLine={group.rangeEnd}
                diffHunk={group.diffHunk}
                annotatedLines={group.annotatedLines}
              />

              {#each group.cards as entry (entry.annotation.id)}
                {@const ann = entry.annotation}
                {@const isActive = activeIndex === entry.flatIndex}
                {@const isAgent = AGENT_AUTHORS.has(ann.author.toLowerCase())}
                {@const kindColor = KIND_COLORS[ann.kind]}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  data-review-card={entry.flatIndex}
                  class="review-thread"
                  class:review-thread-active={isActive}
                  class:review-thread-resolved={ann.resolved}
                  onclick={() => setActiveCard(entry.flatIndex)}
                >
                  {#if pendingDelete && isActive}
                    <div class="review-delete-confirm"><span>Press <kbd>d</kbd> again to delete</span></div>
                  {/if}

                  <div class="review-thread-header">
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <span
                      class="review-thread-line-tag"
                      onmouseenter={(e) => handleLineTagEnter(e, ann.anchor.range.startLine)}
                      onmouseleave={(e) => handleLineTagLeave(e, ann.anchor.range.startLine)}
                    >L{ann.anchor.range.startLine}</span>
                    <span class="review-thread-author">
                      {#if isAgent}<Bot size={13} class="review-thread-agent-icon" />{/if}
                      {ann.author}
                    </span>
                    <span class="review-thread-badge" style:background="color-mix(in srgb, {kindColor} 15%, transparent)" style:color={kindColor}>
                      {KIND_LABELS[ann.kind]}
                    </span>
                    {#if syncBadge(ann.github?.syncState)}
                      {@const badge = syncBadge(ann.github?.syncState)}
                      <span class={`review-thread-sync-badge ${badge?.className}`}>{badge?.label}</span>
                    {/if}
                    <span class="review-thread-time">{relativeTime(ann.createdAt)}</span>
                  </div>

                  <div class="review-thread-body">{ann.body}</div>

                  {#if ann.choices && ann.choices.length > 0}
                    <div class="review-thread-choices">
                      {#each ann.choices as choice, i}
                        <label
                          class="review-thread-choice"
                          class:review-thread-choice-selected={choice.selected}
                        >
                          <input
                            type={ann.selectionMode === "multi" ? "checkbox" : "radio"}
                            name="review-choice-{ann.id}"
                            checked={choice.selected}
                            onchange={() => handleChoiceToggle(ann.id, i)}
                            hidden
                          />
                          <kbd class="review-thread-choice-key">{i + 1}</kbd>
                          <span>{choice.label}</span>
                        </label>
                      {/each}
                    </div>
                  {/if}

                  {#if ann.labels.length > 0}
                    <div class="review-thread-labels">
                      {#each ann.labels as label}
                        <span class="review-thread-label">{label}</span>
                      {/each}
                    </div>
                  {/if}

                  {#if entry.replies.length > 0}
                    <div class="review-thread-replies">
                      {#each entry.replies as reply}
                        <div class="review-thread-reply">
                          <span class="review-thread-reply-indicator">↳</span>
                          <span class="review-thread-reply-author">
                            {#if AGENT_AUTHORS.has(reply.author.toLowerCase())}<Bot size={11} class="review-thread-agent-icon" />{/if}
                            {reply.author}
                          </span>
                          {#if syncBadge(reply.github?.syncState)}
                            {@const badge = syncBadge(reply.github?.syncState)}
                            <span class={`review-thread-sync-badge ${badge?.className}`}>{badge?.label}</span>
                          {/if}
                          {#if reply.createdAt}
                            <span class="review-thread-reply-time">{relativeTime(reply.createdAt)}</span>
                          {/if}
                          <span class="review-thread-reply-text">{reply.body}</span>
                        </div>
                      {/each}
                    </div>
                  {/if}

                  <div class="review-thread-actions">
                    <button
                      class="review-thread-resolve-btn"
                      class:review-thread-resolve-btn-resolved={ann.resolved}
                      onclick={(e) => { e.stopPropagation(); handleResolve(ann.id, !ann.resolved); }}
                    >
                      {ann.resolved ? "Resolved" : "Resolve"}
                    </button>
                    {#if replyingTo !== ann.id}
                      <button
                        class="review-thread-reply-btn"
                        onclick={(e) => { e.stopPropagation(); replyingTo = ann.id; replyText = ""; }}
                      >
                        Reply
                      </button>
                    {/if}
                  </div>

                  {#if replyingTo === ann.id}
                    <div class="review-thread-reply-input-wrapper">
                      <!-- svelte-ignore a11y_autofocus -->
                      <input
                        bind:this={replyInputRef}
                        class="review-thread-reply-input"
                        placeholder="Reply..."
                        bind:value={replyText}
                        onkeydown={(e) => handleReplyKeydown(e, ann.id)}
                        autofocus
                      />
                    </div>
                  {/if}
                </div>
              {/each}
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
  .review-file-jump:hover {
    text-decoration: underline;
  }

  /* Context block — surface elevation, replaces heavy bordered cards */
  .review-context-block {
    background: var(--surface-panel);
    border-radius: 8px;
    overflow: hidden;
    margin: 10px 0;
  }

  /* Thread within a context block */
  .review-thread {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-subtle, var(--border-default));
    cursor: pointer;
    position: relative;
  }
  .review-thread:last-child {
    border-bottom: none;
  }
  .review-thread-active {
    background: rgba(255, 255, 255, 0.015);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .review-thread-resolved {
    opacity: 0.5;
  }

  .review-thread-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .review-thread-line-tag {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    padding: 1px 6px;
    border-radius: 4px;
    cursor: default;
    user-select: none;
  }
  .review-thread-author {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.review-thread-agent-icon) {
    color: var(--accent);
  }
  .review-thread-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .review-thread-time {
    font-size: 11px;
    color: var(--text-muted);
    margin-left: auto;
    white-space: nowrap;
  }
  .review-thread-body {
    color: var(--text-primary);
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
    padding-left: 2px;
  }
  .review-thread-choices {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .review-thread-choice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .review-thread-choice:hover {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }
  .review-thread-choice-key {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 0 5px;
    font-family: inherit;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .review-thread-choice-selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--text-primary);
  }
  .review-thread-labels {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }
  .review-thread-label {
    font-size: 10px;
    background: var(--surface-raised);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 4px;
  }
  .review-thread-sync-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 999px;
    border: 1px solid var(--border-default);
  }
  .review-thread-sync-pending {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--accent) 30%, var(--border-default));
  }
  .review-thread-sync-submitted {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-success) 30%, var(--border-default));
  }
  .review-thread-sync-local {
    color: var(--text-secondary);
    background: var(--surface-raised);
  }
  .review-thread-sync-conflict {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-default));
  }

  /* Inline replies */
  .review-thread-replies {
    margin-top: 6px;
    padding-left: 2px;
  }
  .review-thread-reply {
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: baseline;
    gap: 4px;
    flex-wrap: wrap;
    padding: 4px 0;
  }
  .review-thread-reply-indicator {
    color: var(--text-muted);
  }
  .review-thread-reply-author {
    font-weight: 600;
    font-size: 12px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .review-thread-reply-time {
    color: var(--text-muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .review-thread-reply-text {
    flex: 1;
  }

  /* Thread actions */
  .review-thread-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }
  .review-thread-resolve-btn {
    font-size: 12px;
    padding: 3px 10px;
    border-radius: 5px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .review-thread-resolve-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
    background: var(--surface-raised);
  }
  .review-thread-resolve-btn-resolved {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-success) 30%, transparent);
  }
  .review-thread-reply-btn {
    font-size: 12px;
    padding: 3px 10px;
    border-radius: 5px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .review-thread-reply-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
    background: var(--surface-raised);
  }

  /* Reply input — hidden until activated */
  .review-thread-reply-input-wrapper {
    margin-top: 8px;
  }
  .review-thread-reply-input {
    width: 100%;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 5px;
    padding: 5px 10px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
    box-sizing: border-box;
  }
  .review-thread-reply-input::placeholder {
    color: var(--text-muted);
  }
  .review-thread-reply-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  /* Delete confirmation overlay */
  .review-delete-confirm {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(2px);
    border-radius: 0;
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

  /* Diff-only group (unchanged) */
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

  /* Status / empty */
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

  /* Help overlay */
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
    background: var(--surface-panel);
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
