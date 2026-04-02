<script lang="ts">
  import DiffEditor from "../DiffEditor.svelte";
  import Editor from "../Editor.svelte";
  import GitHubInbox from "../GitHubInbox.svelte";
  import MarkdownPreview from "../MarkdownPreview.svelte";
  import PullRequestView from "../PullRequestView.svelte";
  import ReviewPage from "../ReviewPage.svelte";
  import { buildSplitDecorations, buildUnifiedDocument } from "$lib/codemirror/diff";
  import { sortedAnnotations } from "$lib/stores/annotations.svelte";
  import { getDiffState, exitDiff, fetchFileContentAtRef } from "$lib/stores/diff.svelte";
  import { getEditor, getFileExtension, getShowPreview, isMarkdownFile } from "$lib/stores/editor.svelte";
  import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
  import { isReviewPageOpen } from "$lib/stores/reviewPage.svelte";
  import { getWorkspace } from "$lib/stores/workspace.svelte";

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

  type PreviewRef = { scrollToLine: (line: number) => void };
  type SplitDiffRef = { getView: () => any };
  type ActiveDiffRef = {
    getView: () => any;
    moveCursorByLine: (dir: 1 | -1) => void;
    jumpCursorToBoundary: (boundary: "top" | "bottom") => void;
    toggleVisualMode: (mode: "char" | "line") => void;
    clearVisualMode: () => void;
    hasVisualMode: () => boolean;
  };
  type SplitActiveDiffRef = {
    getView: () => any;
    moveCursorByLine?: (dir: 1 | -1) => void;
    jumpCursorToBoundary?: (boundary: "top" | "bottom") => void;
    toggleVisualMode?: (mode: "char" | "line") => void;
    clearVisualMode?: () => void;
    hasVisualMode?: () => boolean;
  };

  let {
    onJumpToFile,
    onOpenFolder,
    onSelectionChange,
    previewRef = $bindable(undefined),
    editorRef = $bindable(undefined),
    leftDiffEditor = $bindable(undefined),
    rightDiffEditor = $bindable(undefined),
    showShortcutHelp = $bindable(false),
    showPrView,
    unifiedDiffEditor = $bindable(undefined),
  }: {
    onJumpToFile?: (filePath: string, line: number) => void;
    onOpenFolder?: () => Promise<void>;
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    previewRef?: PreviewRef | undefined;
    editorRef?: EditorRef | undefined;
    leftDiffEditor?: SplitDiffRef | undefined;
    rightDiffEditor?: SplitActiveDiffRef | undefined;
    showShortcutHelp?: boolean;
    showPrView: boolean;
    unifiedDiffEditor?: ActiveDiffRef | undefined;
  } = $props();

  const editor = getEditor();
  const diff = getDiffState();
  const workspace = getWorkspace();
  const githubReview = getGitHubReviewState();

  let splitOldContent = $state<string>("");
  let splitNewContent = $state<string>("");
  let splitContentLoading = $state(false);

  $effect(() => {
    if (diff.enabled && diff.mode === "split" && diff.diffResult && editor.currentFilePath && workspace.rootFolders.length > 0) {
      const directory = workspace.rootFolders[0];
      const filePath = editor.currentFilePath;
      const baseRef = diff.baseRef;
      const targetRef = diff.targetRef;

      splitContentLoading = true;
      Promise.all([
        fetchFileContentAtRef(directory, filePath, baseRef).catch(() => ""),
        fetchFileContentAtRef(directory, filePath, targetRef).catch(() => ""),
      ]).then(([oldContent, newContent]) => {
        splitOldContent = oldContent;
        splitNewContent = newContent;
        splitContentLoading = false;
      });
    }
  });

  function createUnifiedSelectionHandler(lineMap: Map<number, number>) {
    return (fromLine: number, fromCol: number, toLine: number, toCol: number) => {
      for (let line = fromLine; line <= toLine; line += 1) {
        if (!lineMap.has(line)) {
          return;
        }
      }

      const mappedFromLine = lineMap.get(fromLine);
      const mappedToLine = lineMap.get(toLine);
      if (!mappedFromLine || !mappedToLine) return;

      onSelectionChange?.(mappedFromLine, fromCol, mappedToLine, toCol);
    };
  }
</script>

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
      {#if splitContentLoading}
        <div class="diff-status">Loading split view...</div>
      {:else}
        {@const splitDeco = buildSplitDecorations(diff.diffResult)}
        <div class="split-diff">
          <DiffEditor
            bind:this={leftDiffEditor}
            content={splitOldContent}
            fileExtension={getFileExtension()}
            diffExtensions={splitDeco.oldExtensions}
          />
          <div class="split-divider"></div>
          <DiffEditor
            bind:this={rightDiffEditor}
            content={splitNewContent}
            fileExtension={getFileExtension()}
            diffExtensions={splitDeco.newExtensions}
          />
        </div>
      {/if}
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

<style>
  .pane-content {
    flex: 1;
    overflow: hidden;
  }
  .split-diff {
    display: flex;
    height: 100%;
  }
  .split-diff > :global(*) {
    flex: 1;
    min-width: 0;
  }
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
