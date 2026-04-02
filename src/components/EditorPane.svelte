<script lang="ts">
  import CodeInputController from "./editor-pane/CodeInputController.svelte";
  import WorkspaceContentRouter from "./editor-pane/WorkspaceContentRouter.svelte";
  import { getReviewSession, clearReviewSession } from "$lib/stores/review.svelte";
  import { getEditor, getShowPreview, isMarkdownFile } from "$lib/stores/editor.svelte";
  import {
    getDiffState,
    enterDiff as enterDiffMode,
    setDiffMode,
    setDiffDefaults,
    resetDiffDefaults,
    computeDiff,
  } from "$lib/stores/diff.svelte";
  import {
    isReviewPageOpen,
    openReviewPage,
    closeReviewPage,
  } from "$lib/stores/reviewPage.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
  import { scrollSync } from "$lib/codemirror/diff";
  import { submitReviewVerdict, type ReviewVerdict } from "$lib/review";
  import { onDestroy } from "svelte";

  let {
    onSelectionChange,
    onJumpToFile,
    onOpenFolder,
    onOpenSettings,
    showShortcutHelp = $bindable(false),
    showPrView = $bindable(false),
    showChecksView = $bindable(false),
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    onJumpToFile?: (filePath: string, line: number) => void;
    onOpenFolder?: () => Promise<void>;
    onOpenSettings?: () => void;
    showShortcutHelp?: boolean;
    showPrView?: boolean;
    showChecksView?: boolean;
    ref?: {
      scrollToLine: (line: number) => void;
      openSearch: () => void;
      closeSearch: () => void;
      navigateMatch: (dir: 1 | -1) => void;
      getView: () => any;
      moveCursorLine: (dir: 1 | -1) => void;
      jumpToBoundary: (boundary: "top" | "bottom") => void;
      toggleVisualSelection: (mode: "char" | "line") => void;
      clearVisualSelection: () => void;
      hasVisualSelection: () => boolean;
    } | undefined;
  } = $props();

  const editor = getEditor();
  const diff = getDiffState();
  const workspace = getWorkspace();
  const githubReview = getGitHubReviewState();

  type EditorRef = {
    scrollToLine: (line: number) => void;
    openSearch: () => void;
    closeSearch: () => void;
    navigateMatch: (dir: 1 | -1) => void;
    getView: () => any;
    moveCursorLine: (dir: 1 | -1) => void;
    jumpToBoundary: (boundary: "top" | "bottom") => void;
    toggleVisualSelection: (mode: "char" | "line") => void;
    clearVisualSelection: () => void;
    hasVisualSelection: () => boolean;
  };
  let editorRef: EditorRef | undefined = $state(undefined);
  let previewRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);
  let leftDiffEditor: { getView: () => any } | undefined = $state(undefined);
  let rightDiffEditor: {
    getView: () => any;
    moveCursorByLine?: (dir: 1 | -1) => void;
    jumpCursorToBoundary?: (boundary: "top" | "bottom") => void;
    toggleVisualMode?: (mode: "char" | "line") => void;
    clearVisualMode?: () => void;
    hasVisualMode?: () => boolean;
  } | undefined = $state(undefined);
  let unifiedDiffEditor: {
    getView: () => any;
    moveCursorByLine: (dir: 1 | -1) => void;
    jumpCursorToBoundary: (boundary: "top" | "bottom") => void;
    toggleVisualMode: (mode: "char" | "line") => void;
    clearVisualMode: () => void;
    hasVisualMode: () => boolean;
  } | undefined = $state(undefined);
  let cleanupScrollSync: (() => void) | null = null;
  let lastGitHubSessionId = $state<string | null>(null);

  $effect(() => {
    if (!githubReview.activeSession) {
      if (showPrView) showPrView = false;
      if (showChecksView) showChecksView = false;
    }
  });

  $effect(() => {
    const sessionId = githubReview.activeSession?.id ?? null;
    if (sessionId !== lastGitHubSessionId) {
      showPrView = false;
      showChecksView = false;
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
      enterDiffMode(directory, editor.currentFilePath);
    }
  }

  function handleSelectCodeView() {
    showPrView = false;
    showChecksView = false;
    if (isReviewPageOpen()) {
      closeReviewPage();
    }
  }

  function handleSelectReviewView() {
    showPrView = false;
    showChecksView = false;
    if (!isReviewPageOpen()) {
      openReviewPage("changes");
    }
  }

  function handleSelectPrView() {
    showPrView = true;
    showChecksView = false;
    if (isReviewPageOpen()) {
      closeReviewPage();
    }
  }

  function handleSelectChecksView() {
    showChecksView = true;
    showPrView = false;
    if (isReviewPageOpen()) {
      closeReviewPage();
    }
  }

  /** Cycle through Code → Review → PR → Checks (→ Code) with direction */
  export function cycleView(direction: 1 | -1) {
    const hasPr = Boolean(githubReview.activeSession);
    // Determine current tab index: 0=Code, 1=Review, 2=PR, 3=Checks
    let current = 0;
    if (isReviewPageOpen() && !showPrView && !showChecksView) current = 1;
    else if (showPrView) current = 2;
    else if (showChecksView) current = 3;

    const tabCount = hasPr ? 4 : 2;
    const next = ((current + direction) % tabCount + tabCount) % tabCount;

    if (next === 0) handleSelectCodeView();
    else if (next === 1) handleSelectReviewView();
    else if (next === 2) handleSelectPrView();
    else if (next === 3) handleSelectChecksView();
  }

  async function handleAgentReviewVerdict(verdict: ReviewVerdict) {
    if (!editor.currentFilePath) return;
    await submitReviewVerdict(editor.currentFilePath, verdict);
    clearReviewSession();
  }

  export function getShowPrView() {
    return showPrView;
  }

  export function selectCodeView() {
    handleSelectCodeView();
  }

  export function selectReviewView() {
    handleSelectReviewView();
  }

  export function selectPrView() {
    handleSelectPrView();
  }

  export function selectChecksView() {
    handleSelectChecksView();
  }

  export function enterDiff(mode: import("$lib/types").DiffMode) {
    handleEnterDiff(mode);
  }

  export function agentReviewVerdict(verdict: ReviewVerdict) {
    return handleAgentReviewVerdict(verdict);
  }

</script>

<div class="editor-pane">
  <WorkspaceContentRouter
    bind:editorRef
    bind:leftDiffEditor
    bind:previewRef
    bind:rightDiffEditor
    bind:showShortcutHelp
    bind:unifiedDiffEditor
    {showPrView}
    {showChecksView}
    {onJumpToFile}
    {onOpenFolder}
    {onOpenSettings}
    {onSelectionChange}
  />

  <CodeInputController
    bind:showShortcutHelp
    {editorRef}
    {rightDiffEditor}
    {unifiedDiffEditor}
  />
</div>

<style>
  .editor-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }
</style>
