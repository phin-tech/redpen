<script lang="ts">
  import Editor from "./Editor.svelte";
  import MarkdownPreview from "./MarkdownPreview.svelte";
  import DiffEditor from "./DiffEditor.svelte";
  import DiffModeToggle from "./DiffModeToggle.svelte";
  import DiffRefPicker from "./DiffRefPicker.svelte";
  import ReviewPage from "./ReviewPage.svelte";
  import PullRequestView from "./PullRequestView.svelte";
  import GitHubInbox from "./GitHubInbox.svelte";
  import { getReviewSession, clearReviewSession } from "$lib/stores/review.svelte";
  import { getEditor, getFileExtension, isMarkdownFile, getShowPreview, togglePreview } from "$lib/stores/editor.svelte";
  import {
    getDiffState,
    enterDiff,
    exitDiff,
    setDiffMode,
    setDiffDefaults,
    resetDiffDefaults,
    computeDiff,
  } from "$lib/stores/diff.svelte";
  import {
    getReviewPageState,
    isReviewPageOpen,
    getAnnotationCount,
    getResolvedCount,
    toggleScope,
    openReviewPage,
    closeReviewPage,
  } from "$lib/stores/reviewPage.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
  import { sortedAnnotations, getBubblesEnabled, toggleBubbles } from "$lib/stores/annotations.svelte";
  import BubbleKindFilter from "./BubbleKindFilter.svelte";
  import { highlightsModeExtensions, buildUnifiedDocument, buildSplitDecorations, scrollSync } from "$lib/codemirror/diff";
  import { submitReviewVerdict, type ReviewVerdict } from "$lib/review";
  import { onDestroy } from "svelte";

  let {
    onSelectionChange,
    onJumpToFile,
    onOpenFolder,
    showShortcutHelp = $bindable(false),
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    onJumpToFile?: (filePath: string, line: number) => void;
    onOpenFolder?: () => Promise<void>;
    showShortcutHelp?: boolean;
    ref?: {
      scrollToLine: (line: number) => void;
      openSearch: () => void;
      closeSearch: () => void;
      navigateMatch: (dir: 1 | -1) => void;
      getView?: () => any;
      moveCursorLine?: (dir: 1 | -1) => void;
      jumpToBoundary?: (boundary: "top" | "bottom") => void;
      toggleVisualSelection?: (mode: "char" | "line") => void;
      clearVisualSelection?: () => void;
      hasVisualSelection?: () => boolean;
    } | undefined;
  } = $props();

  const editor = getEditor();
  const diff = getDiffState();
  const workspace = getWorkspace();
  const reviewState = getReviewPageState();
  const reviewSession = getReviewSession();
  const githubReview = getGitHubReviewState();

  type EditorRef = {
    scrollToLine: (line: number) => void;
    openSearch: () => void;
    closeSearch: () => void;
    navigateMatch: (dir: 1 | -1) => void;
    getView?: () => any;
    moveCursorLine?: (dir: 1 | -1) => void;
    jumpToBoundary?: (boundary: "top" | "bottom") => void;
    toggleVisualSelection?: (mode: "char" | "line") => void;
    clearVisualSelection?: () => void;
    hasVisualSelection?: () => boolean;
  };
  let editorRef: EditorRef | undefined = $state(undefined);
  let previewRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);
  let leftDiffEditor: { getView: () => any } | undefined = $state(undefined);
  let rightDiffEditor: { getView: () => any } | undefined = $state(undefined);
  let unifiedDiffEditor: {
    getView: () => any;
    moveCursorByLine: (dir: 1 | -1) => void;
    jumpCursorToBoundary: (boundary: "top" | "bottom") => void;
    toggleVisualMode: (mode: "char" | "line") => void;
    clearVisualMode: () => void;
    hasVisualMode: () => boolean;
  } | undefined = $state(undefined);
  let cleanupScrollSync: (() => void) | null = null;
  let pendingCodeG = $state(false);
  let showPrView = $state(false);
  let lastGitHubSessionId = $state<string | null>(null);

  interface ShortcutHelpSection {
    title: string;
    items: { keys: string[]; label: string }[];
  }

  const codeShortcutHelpSections: ShortcutHelpSection[] = [
    {
      title: "Global",
      items: [
        { keys: ["Mod", "K"], label: "open command palette" },
        { keys: ["Mod", "P"], label: "go to file" },
        { keys: ["[", "]"], label: "switch code / review view" },
      ],
    },
    {
      title: "Editing",
      items: [
        { keys: ["j", "k"], label: "move cursor up / down one line" },
        { keys: ["gg", "G"], label: "jump to top / bottom" },
        { keys: ["v", "V"], label: "toggle visual / line selection" },
        { keys: ["Esc"], label: "clear visual selection" },
        { keys: ["Mod", "Enter"], label: "add annotation" },
        { keys: ["Mod", "F"], label: "find in file" },
        { keys: ["Mod", "G"], label: "next search match" },
        { keys: ["Mod", "Shift", "G"], label: "previous search match" },
      ],
    },
    {
      title: "UI",
      items: [
        { keys: ["Mod", "Shift", "R"], label: "open or close review" },
        { keys: ["Mod", "Shift", "M"], label: "toggle markdown preview" },
        { keys: ["Mod", ","], label: "open settings" },
      ],
    },
  ];

  const reviewScopeLabel = $derived(
    reviewState.scope === "all-changes" ? "All Changes" : "Session"
  );
  const reviewAnnotationCount = $derived(getAnnotationCount());
  const reviewResolvedCount = $derived(getResolvedCount());
  const reviewTabBadge = $derived(
    isReviewPageOpen() ? reviewAnnotationCount : null
  );

  $effect(() => {
    if (!githubReview.activeSession && showPrView) {
      showPrView = false;
    }
  });

  $effect(() => {
    const sessionId = githubReview.activeSession?.id ?? null;
    if (sessionId !== lastGitHubSessionId) {
      showPrView = false;
      lastGitHubSessionId = sessionId;
    }
  });

  // Forward the active ref to parent
  $effect(() => {
    ref = getShowPreview() && isMarkdownFile() ? previewRef as EditorRef | undefined : editorRef;
  });

  // Set up scroll sync for split mode
  // Only track diff.enabled and diff.mode — not the editor refs (avoids loops on destroy)
  $effect(() => {
    // Clean up previous sync
    cleanupScrollSync?.();
    cleanupScrollSync = null;

    if (diff.enabled && diff.mode === "split") {
      // Delay to let DiffEditor instances mount and populate bind:this
      const timer = setTimeout(() => {
        const leftView = leftDiffEditor?.getView();
        const rightView = rightDiffEditor?.getView();
        if (leftView && rightView) {
          cleanupScrollSync = scrollSync(leftView, rightView);
        }
      }, 50);
      return () => clearTimeout(timer);
    }
  });

  onDestroy(() => {
    cleanupScrollSync?.();
  });

  const directory = $derived(workspace.rootFolders[0] ?? "");

  $effect(() => {
    const session = githubReview.activeSession;
    const filePath = editor.currentFilePath;

    if (session && session.worktreePath === directory) {
      setDiffDefaults(
        session.baseSha,
        session.headSha,
        session.baseRef,
        session.headRef,
      );
      if (diff.enabled && filePath) {
        void computeDiff(directory, filePath);
      }
      return;
    }

    resetDiffDefaults();
    if (diff.enabled && filePath && directory) {
      void computeDiff(directory, filePath);
    }
  });

  function handleEnterDiff(mode: import("$lib/types").DiffMode) {
    if (editor.currentFilePath && directory) {
      setDiffMode(mode);
      enterDiff(directory, editor.currentFilePath);
    }
  }

  function handleSelectCodeView() {
    showPrView = false;
    if (isReviewPageOpen()) {
      closeReviewPage();
    }
  }

  function handleSelectReviewView() {
    showPrView = false;
    if (!isReviewPageOpen()) {
      openReviewPage("changes");
    }
  }

  function handleSelectPrView() {
    showPrView = true;
    if (isReviewPageOpen()) {
      closeReviewPage();
    }
  }

  async function handleAgentReviewVerdict(verdict: ReviewVerdict) {
    if (!editor.currentFilePath) return;
    await submitReviewVerdict(editor.currentFilePath, verdict);
    clearReviewSession();
  }

  function handleWindowClick(e: MouseEvent) {
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if (isReviewPageOpen()) return;

    const target = e.target;
    const isInputTarget = target instanceof HTMLElement
      && !target.closest(".cm-editor")
      && !!target.closest("input, textarea, select, [contenteditable='true']");

    if (isInputTarget) return;

    if (showShortcutHelp) {
      if (e.key === "Escape" || e.key === "?") {
        e.preventDefault();
        e.stopPropagation();
        showShortcutHelp = false;
      }
      return;
    }

    if (e.metaKey || e.ctrlKey || e.altKey) return;

    if (e.key === "?") {
      e.preventDefault();
      e.stopPropagation();
      showShortcutHelp = true;
      pendingCodeG = false;
      return;
    }

    if (e.key === "g") {
      e.preventDefault();
      e.stopPropagation();
      if (pendingCodeG) {
        jumpActiveCodeBoundary("top");
        pendingCodeG = false;
      } else {
        pendingCodeG = true;
        setTimeout(() => { pendingCodeG = false; }, 500);
      }
      return;
    }

    pendingCodeG = false;

    if (e.key === "j") {
      e.preventDefault();
      e.stopPropagation();
      moveActiveCodeLine(1);
    } else if (e.key === "k") {
      e.preventDefault();
      e.stopPropagation();
      moveActiveCodeLine(-1);
    } else if (e.key === "G") {
      e.preventDefault();
      e.stopPropagation();
      jumpActiveCodeBoundary("bottom");
    } else if (e.key === "v") {
      e.preventDefault();
      e.stopPropagation();
      toggleActiveCodeVisual("char");
    } else if (e.key === "V") {
      e.preventDefault();
      e.stopPropagation();
      toggleActiveCodeVisual("line");
    } else if (e.key === "Escape") {
      if (clearActiveCodeVisual()) {
        e.preventDefault();
        e.stopPropagation();
      }
    }
  }

  function moveActiveCodeLine(dir: 1 | -1) {
    if (diff.enabled && diff.mode === "split") {
      rightDiffEditor?.moveCursorByLine?.(dir);
      return;
    }

    if (diff.enabled && diff.mode === "unified") {
      unifiedDiffEditor?.moveCursorByLine?.(dir);
      return;
    }

    editorRef?.moveCursorLine?.(dir);
  }

  function jumpActiveCodeBoundary(boundary: "top" | "bottom") {
    if (diff.enabled && diff.mode === "split") {
      rightDiffEditor?.jumpCursorToBoundary?.(boundary);
      return;
    }

    if (diff.enabled && diff.mode === "unified") {
      unifiedDiffEditor?.jumpCursorToBoundary?.(boundary);
      return;
    }

    editorRef?.jumpToBoundary?.(boundary);
  }

  function toggleActiveCodeVisual(mode: "char" | "line") {
    if (diff.enabled && diff.mode === "split") {
      rightDiffEditor?.toggleVisualMode?.(mode);
      return;
    }

    if (diff.enabled && diff.mode === "unified") {
      unifiedDiffEditor?.toggleVisualMode?.(mode);
      return;
    }

    editorRef?.toggleVisualSelection?.(mode);
  }

  function clearActiveCodeVisual(): boolean {
    if (diff.enabled && diff.mode === "split") {
      const hadVisual = rightDiffEditor?.hasVisualMode?.() ?? false;
      if (hadVisual) rightDiffEditor?.clearVisualMode?.();
      return hadVisual;
    }

    if (diff.enabled && diff.mode === "unified") {
      const hadVisual = unifiedDiffEditor?.hasVisualMode?.() ?? false;
      if (hadVisual) unifiedDiffEditor?.clearVisualMode?.();
      return hadVisual;
    }

    const hadVisual = editorRef?.hasVisualSelection?.() ?? false;
    if (hadVisual) editorRef?.clearVisualSelection?.();
    return hadVisual;
  }

  // For unified mode: filter selections to only allow annotatable (non-delete) lines
  function createUnifiedSelectionHandler(lineMap: Map<number, number>) {
    return (fromLine: number, fromCol: number, toLine: number, toCol: number) => {
      // Check if all selected lines are in the lineMap (not delete lines)
      for (let line = fromLine; line <= toLine; line++) {
        if (!lineMap.has(line)) {
          return; // Selection includes a delete line — block annotation
        }
      }
      // Map synthetic line numbers to real new-content line numbers
      const mappedFromLine = lineMap.get(fromLine)!;
      const mappedToLine = lineMap.get(toLine)!;
      onSelectionChange?.(mappedFromLine, fromCol, mappedToLine, toCol);
    };
  }

</script>

<svelte:window onkeydowncapture={handleWindowKeydown} onclick={handleWindowClick} />

<div class="editor-pane">
  <!-- Toolbar: always visible when a file is open -->
  {#if editor.currentFilePath}
    <div class="toggle-bar">
      <div class="view-tabs">
        <button
          class="toggle-btn view-tab"
          class:active={!isReviewPageOpen() && !showPrView}
          onclick={handleSelectCodeView}
        >
          Code
        </button>
        <button
          class="toggle-btn view-tab view-tab-review"
          class:active={isReviewPageOpen() && !showPrView}
          onclick={handleSelectReviewView}
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
            onclick={handleSelectPrView}
          >
            PR
          </button>
        {/if}
      </div>
      <div class="separator"></div>
      {#if isReviewPageOpen()}
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
            <button class="toggle-btn review-toolbar-btn review-toolbar-btn-success" onclick={() => void handleAgentReviewVerdict("approved")}>
              Approve
            </button>
            <button class="toggle-btn review-toolbar-btn review-toolbar-btn-danger" onclick={() => void handleAgentReviewVerdict("changes_requested")}>
              Request changes
            </button>
          {/if}
        </div>
      {:else if showPrView && githubReview.activeSession}
        <div class="review-toolbar">
          <span class="review-toolbar-meta">PR overview</span>
        </div>
      {:else}
        <DiffModeToggle onEnterDiff={handleEnterDiff} />
        {#if diff.enabled}
          <DiffRefPicker {directory} filePath={editor.currentFilePath ?? ""} />
        {/if}
        <div class="separator"></div>
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
          <div class="separator"></div>
          <button
            class="toggle-btn"
            class:active={!getShowPreview()}
            onclick={() => { if (getShowPreview()) togglePreview(); }}
          >
            Source
          </button>
          <button
            class="toggle-btn"
            class:active={getShowPreview()}
            onclick={() => { if (!getShowPreview()) togglePreview(); }}
          >
            Preview
          </button>
        {/if}
        {#if reviewSession.active && !githubReview.activeSession}
          <div class="separator"></div>
          <button class="toggle-btn review-toolbar-btn review-toolbar-btn-success" onclick={() => void handleAgentReviewVerdict("approved")}>
            Approve
          </button>
          <button class="toggle-btn review-toolbar-btn review-toolbar-btn-danger" onclick={() => void handleAgentReviewVerdict("changes_requested")}>
            Request changes
          </button>
        {/if}
      {/if}
    </div>
  {/if}

  <!-- Content area -->
  <div class="pane-content">
    {#if workspace.rootFolders.length === 0}
      <GitHubInbox onOpenFolder={onOpenFolder ?? (async () => {})} />
    {:else if showPrView && githubReview.activeSession}
      <PullRequestView session={githubReview.activeSession} />
    {:else if isReviewPageOpen()}
      <ReviewPage
        onJumpToFile={(path, line) => onJumpToFile?.(path, line)}
        bind:showShortcutHelp
      />
    {:else if diff.enabled && diff.loading}
      <div class="diff-status">Computing diff...</div>
    {:else if diff.enabled && diff.error}
      <div class="diff-status">
        <p>Cannot compute diff: {diff.error}</p>
        <button class="exit-btn" onclick={() => exitDiff()}>Exit diff mode</button>
      </div>
    {:else if diff.enabled && diff.diffResult}
      {#if diff.diffResult.hunks.length === 0}
        <div class="diff-status">No changes between {diff.baseLabel} and {diff.targetLabel}</div>
      {:else if diff.mode === "split"}
        {@const splitDeco = buildSplitDecorations(diff.diffResult)}
        <div class="split-diff">
          <DiffEditor
            bind:this={leftDiffEditor}
            content={diff.diffResult.oldContent}
            fileExtension={getFileExtension()}
            diffExtensions={splitDeco.oldExtensions}
          />
          <div class="split-divider"></div>
          <DiffEditor
            bind:this={rightDiffEditor}
            content={diff.diffResult.newContent}
            fileExtension={getFileExtension()}
            diffExtensions={splitDeco.newExtensions}
          />
        </div>
      {:else if diff.mode === "unified"}
        {@const unified = buildUnifiedDocument(diff.diffResult)}
        <DiffEditor
          bind:this={unifiedDiffEditor}
          content={unified.syntheticDoc}
          fileExtension={getFileExtension()}
          diffExtensions={unified.extensions}
          showLineNumbers={false}
          onSelectionChange={createUnifiedSelectionHandler(unified.lineMap)}
        />
      {:else}
        <!-- Highlights mode: existing Editor with diff decorations overlaid -->
        <Editor {onSelectionChange} bind:ref={editorRef} />
      {/if}
    {:else if isMarkdownFile() && getShowPreview()}
      <MarkdownPreview
        content={editor.content}
        annotations={sortedAnnotations()}
        {onSelectionChange}
        bind:ref={previewRef}
      />
    {:else}
      <Editor {onSelectionChange} bind:ref={editorRef} />
    {/if}
  </div>

  {#if showShortcutHelp && !isReviewPageOpen()}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="shortcut-help-overlay" onclick={() => (showShortcutHelp = false)}>
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="shortcut-help-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Code keyboard shortcuts"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="shortcut-help-header">
          <div>
            <h2>Code shortcuts</h2>
            <p>Global and editor shortcuts for the code view.</p>
          </div>
          <button class="shortcut-help-close" onclick={() => (showShortcutHelp = false)} aria-label="Close keyboard shortcut help">
            ×
          </button>
        </div>

        <div class="shortcut-help-grid">
          {#each codeShortcutHelpSections as section}
            <section class="shortcut-help-section">
              <h3>{section.title}</h3>
              <div class="shortcut-help-list">
                {#each section.items as item}
                  <div class="shortcut-help-row">
                    <div class="shortcut-help-keys">
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

</div>

<style>
  .editor-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .toggle-bar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 8px;
    background: var(--gradient-toolbar), var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .view-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    flex-shrink: 0;
  }
  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 30px;
    padding: 4px 10px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }
  .view-tab {
    min-width: 76px;
    justify-content: center;
    flex: 0 0 auto;
  }
  .view-tab.active {
    background: transparent;
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
  .separator {
    width: 1px;
    height: 16px;
    background: var(--border-default);
    margin: 0 4px;
  }
  .pane-content { flex: 1; overflow: hidden; }
  .review-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }
  .review-toolbar-meta {
    display: inline-flex;
    align-items: center;
    min-height: 30px;
    padding: 4px 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 4px;
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
    white-space: nowrap;
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
  .review-toolbar-spacer {
    flex: 1;
    min-width: 0;
  }
  .shortcut-help-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(6, 7, 10, 0.76);
    backdrop-filter: blur(2px);
    z-index: 30;
    padding: 24px;
  }
  .shortcut-help-modal {
    width: min(760px, 100%);
    max-height: min(680px, calc(100vh - 48px));
    overflow: auto;
    background: var(--surface-panel);
    border: 1px solid var(--border-emphasis);
    border-radius: 10px;
    box-shadow: var(--shadow-popover);
    padding: 20px;
  }
  .shortcut-help-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 18px;
  }
  .shortcut-help-header h2 {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  .shortcut-help-header p {
    margin: 6px 0 0;
    color: var(--text-muted);
    font-size: 13px;
  }
  .shortcut-help-close {
    border: 1px solid var(--border-default);
    background: var(--surface-raised);
    color: var(--text-muted);
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
  }
  .shortcut-help-close:hover {
    color: var(--text-secondary);
    border-color: var(--border-emphasis);
  }
  .shortcut-help-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 16px;
  }
  .shortcut-help-section {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: 14px;
  }
  .shortcut-help-section h3 {
    margin: 0 0 10px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .shortcut-help-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .shortcut-help-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .shortcut-help-keys {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }
  .shortcut-help-keys kbd {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }
  .split-diff {
    display: flex;
    height: 100%;
  }
  .split-diff > :global(*) { flex: 1; min-width: 0; }
  .split-divider {
    width: 1px;
    background: var(--border-default);
    flex: none;
  }
  .diff-status {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 8px;
  }
  .exit-btn {
    padding: 4px 12px;
    border-radius: 4px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 12px;
  }
</style>
