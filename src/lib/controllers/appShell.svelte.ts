import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { watch, writeTextFile } from "@tauri-apps/plugin-fs";

import { createCommandRegistry, findCommand, type AppCommandContext, type CommandPaletteMode } from "$lib/commands";
import { debounce } from "$lib/utils/debounce";
import { readDirectory, sendNotification, getSettings } from "$lib/tauri";
import { getEditor, openFile, isMarkdownFile, togglePreview } from "$lib/stores/editor.svelte";
import {
  addAnnotation,
  clearAllAnnotations,
  getAnnotationsState,
  loadAnnotations,
  selectAnnotation,
  sortedAnnotations,
} from "$lib/stores/annotations.svelte";
import { getAnnotatedLines, getFocusedBubbleLine, setFocusedBubbleEffect } from "$lib/codemirror/bubbles";
import { EditorView } from "@codemirror/view";
import { activateReviewSession, addReviewFile } from "$lib/stores/review.svelte";
import { closeReviewPage, isReviewPageOpen, openReviewPage } from "$lib/stores/reviewPage.svelte";
import {
  addRootFolder,
  collapseAllFolders,
  expandAllFolders,
  getWorkspace,
  toggleShowChangedOnly,
} from "$lib/stores/workspace.svelte";
import { computeDiff, enterDiff, exitDiff, getDiffState, invalidateFile, setDiffMode } from "$lib/stores/diff.svelte";
import { activateGitHubReviewSession, openGitHubPullRequest } from "$lib/stores/githubReview.svelte";
import { submitReviewVerdict } from "$lib/review";
import { isShortcutInputTarget, matchesShortcut } from "$lib/shortcuts";
import type { GitHubPrSession } from "$lib/types";

export interface AppEditorRef {
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
}

interface AnnotationSelection {
  fromLine: number;
  fromCol: number;
  toLine: number;
  toCol: number;
}

interface AppShellControllerOptions {
  getEditorRef?: () => AppEditorRef | undefined;
  jumpToLineDelayMs?: number;
  onToggleLeftPanel?: () => void;
  onToggleRightPanel?: () => void;
}

export function createAppShellController(
  { getEditorRef, jumpToLineDelayMs = 100, onToggleLeftPanel, onToggleRightPanel }: AppShellControllerOptions = {},
) {
  const editor = getEditor();
  const workspace = getWorkspace();
  const diff = getDiffState();
  const commands = createCommandRegistry();

  const state = $state({
    showSettings: false,
    showCommandPalette: false,
    commandPaletteMode: "default" as CommandPaletteMode,
    showReviewShortcutHelp: false,
    selection: null as AnnotationSelection | null,
    showPopover: false,
    popoverPosition: { x: 0, y: 0 },
  });

  let stopWatcher: (() => void) | null = null;
  let unlistenDeepLink: (() => void) | undefined;
  let unlistenDeepLinkEvent: (() => void) | undefined;
  let unlistenSettings: (() => void) | undefined;
  let unlistenGitHubReviewSession: (() => void) | undefined;
  let unlistenDragDrop: (() => void) | undefined;
  let unlistenMenuItems: (() => void)[] = [];

  function editorRef() {
    return getEditorRef?.();
  }

  async function startSelectedFileWatcher(path: string) {
    stopWatcher?.();

    let lastAnnotationIds = new Set(
      getAnnotationsState().sidecar?.annotations.map((annotation) => annotation.id) ?? [],
    );

    const reloadFile = debounce(async () => {
      if (!editor.currentFilePath) return;

      const directory = workspace.rootFolders[0];
      if (directory) {
        invalidateFile(directory, editor.currentFilePath);
        if (diff.enabled) {
          void computeDiff(directory, editor.currentFilePath);
        }
      }

      await openFile(editor.currentFilePath);
      await loadAnnotations(editor.currentFilePath);

      const annotationsState = getAnnotationsState();
      const currentSettings = await getSettings();
      const currentAuthor = currentSettings.author;
      const newAnnotations = (annotationsState.sidecar?.annotations ?? []).filter(
        (annotation) => !lastAnnotationIds.has(annotation.id) && annotation.author !== currentAuthor,
      );

      lastAnnotationIds = new Set(
        annotationsState.sidecar?.annotations.map((annotation) => annotation.id) ?? [],
      );

      if (newAnnotations.length === 0) return;

      const fileName = editor.currentFilePath.split("/").pop() ?? "unknown";
      for (const annotation of newAnnotations) {
        const kind = annotation.replyTo ? "annotation_reply" : "new_annotation";
        const line = annotation.anchor?.range?.startLine;
        sendNotification(kind, fileName, line).catch(() => {});
      }
    }, 500);

    stopWatcher = await watch(path, reloadFile, { recursive: false });
  }

  async function handleFileSelect(path: string) {
    await openFile(path);
    await loadAnnotations(path);

    const directory = workspace.rootFolders[0];
    if (diff.enabled && directory) {
      await computeDiff(directory, path);
    }

    await startSelectedFileWatcher(path);
  }

  async function handleJumpToFile(path: string, line: number) {
    await handleFileSelect(path);
    setTimeout(() => editorRef()?.scrollToLine(line), jumpToLineDelayMs);
  }

  function handleSelectionChange(
    fromLine: number,
    fromCol: number,
    toLine: number,
    toCol: number,
  ) {
    state.selection = { fromLine, fromCol, toLine, toCol };
  }

  function handleAnnotationClick(line: number) {
    editorRef()?.scrollToLine(line);
  }

  function openCommandPalette(mode: CommandPaletteMode) {
    state.commandPaletteMode = mode;
    state.showCommandPalette = true;
  }

  function closeCommandPalette() {
    state.showCommandPalette = false;
  }

  function openSettingsPanel() {
    state.showCommandPalette = false;
    state.showSettings = true;
  }

  function closeSettingsPanel() {
    state.showSettings = false;
  }

  function openAnnotationPopover() {
    if (!state.selection || !editor.currentFilePath) return;

    state.popoverPosition = {
      x: window.innerWidth / 2 - 160,
      y: window.innerHeight / 3,
    };
    state.showPopover = true;
  }

  function closeAnnotationPopover() {
    state.showPopover = false;
  }

  function openReviewShortcutHelp() {
    state.showReviewShortcutHelp = true;
  }

  function closeReviewShortcutHelp() {
    state.showReviewShortcutHelp = false;
  }

  async function handleAnnotationSubmit(body: string, labels: string[]) {
    if (!state.selection || !editor.currentFilePath) return;

    await addAnnotation(
      editor.currentFilePath,
      body,
      labels,
      state.selection.fromLine,
      state.selection.fromCol,
      state.selection.toLine,
      state.selection.toCol,
    );

    state.showPopover = false;
    state.selection = null;
  }

  async function submitCurrentReviewVerdict(verdict: "approved" | "changes_requested") {
    if (!editor.currentFilePath) return;
    await submitReviewVerdict(editor.currentFilePath, verdict);
  }

  async function openFolderPicker() {
    const selected = await openDialog({ directory: true, multiple: true });
    if (!selected) return;

    for (const path of Array.isArray(selected) ? selected : [selected]) {
      if (path) {
        await addRootFolder(path);
      }
    }
  }

  function navigateAnnotation(direction: 1 | -1) {
    const view = editorRef()?.getView();
    if (!view) return;

    const lines = getAnnotatedLines(view.state);
    if (lines.length === 0) return;

    const currentFocus = getFocusedBubbleLine(view.state);
    let nextIndex: number;

    if (currentFocus === null) {
      nextIndex = direction === 1 ? 0 : lines.length - 1;
    } else {
      const currentIndex = lines.indexOf(currentFocus);
      if (currentIndex === -1) {
        nextIndex = direction === 1 ? 0 : lines.length - 1;
      } else {
        nextIndex = (currentIndex + direction + lines.length) % lines.length;
      }
    }

    const targetLine = lines[nextIndex];
    const lineObj = view.state.doc.line(Math.min(targetLine, view.state.doc.lines));

    view.dispatch({
      effects: [
        setFocusedBubbleEffect.of(targetLine),
        EditorView.scrollIntoView(lineObj.from, { y: "center" }),
      ],
    });

    // Sync sidebar selection with the root annotation on the target line
    const annotations = sortedAnnotations();
    const rootOnLine = annotations.find((a) => !a.replyTo && a.anchor.range.startLine === targetLine);
    if (rootOnLine) {
      selectAnnotation(rootOnLine.id);
    }
  }

  const commandContext: AppCommandContext = {
    openCommandPalette,
    openFolder: openFolderPicker,
    openSettings: openSettingsPanel,
    openAddAnnotation: () => {
      state.showCommandPalette = false;
      openAnnotationPopover();
    },
    expandAllFolders,
    collapseAllFolders,
    toggleShowChangedOnly,
    hasRoots: () => workspace.rootFolders.length > 0,
    canAddAnnotation: () => Boolean(state.selection && editor.currentFilePath),
    hasAnnotations: () => {
      const annotationsState = getAnnotationsState();
      return Boolean(
        editor.currentFilePath
        && annotationsState.sidecar
        && annotationsState.sidecar.annotations.length > 0,
      );
    },
    reloadAnnotations: async () => {
      if (editor.currentFilePath) {
        await loadAnnotations(editor.currentFilePath);
      }
    },
    clearAnnotations: async () => {
      if (editor.currentFilePath) {
        await clearAllAnnotations(editor.currentFilePath);
      }
    },
    isMarkdownFile,
    toggleMarkdownPreview: togglePreview,
    enterDiffMode: (mode) => {
      if (!editor.currentFilePath || workspace.rootFolders.length === 0) return;

      setDiffMode(mode);
      if (!diff.enabled) {
        enterDiff(workspace.rootFolders[0], editor.currentFilePath);
      }
    },
    exitDiffMode: () => exitDiff(),
    hasDiffMode: () => diff.enabled,
    hasOpenFile: () => Boolean(editor.currentFilePath),
    openReviewChanges: () => openReviewPage("changes"),
    openAgentFeedback: () => openReviewPage("feedback"),
    isReviewPageOpen: () => isReviewPageOpen(),
    canSubmitReviewVerdict: () => Boolean(editor.currentFilePath),
    approveReview: () => submitCurrentReviewVerdict("approved"),
    requestReviewChanges: () => submitCurrentReviewVerdict("changes_requested"),
    navigateAnnotation,
    toggleLeftPanel: () => onToggleLeftPanel?.(),
    toggleRightPanel: () => onToggleRightPanel?.(),
  };

  async function runCommand(id: string) {
    const command = findCommand(commands, id);
    if (!command) return;
    if (command.isEnabled && !command.isEnabled(commandContext)) return;
    await command.run(commandContext);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing) return;

    const ignoreGlobalShortcuts = isShortcutInputTarget(event.target);

    if (event.key === "Escape") {
      if (state.showCommandPalette) {
        event.preventDefault();
        state.showCommandPalette = false;
        return;
      }

      if (isReviewPageOpen()) {
        event.preventDefault();
        closeReviewPage();
        return;
      }
    }

    if (!ignoreGlobalShortcuts) {
      if (event.key === "n" && !event.metaKey && !event.ctrlKey && !event.altKey) {
        if (event.shiftKey) {
          navigateAnnotation(-1);
        } else {
          navigateAnnotation(1);
        }
        event.preventDefault();
        return;
      }
    }

    if (ignoreGlobalShortcuts) return;

    const isEditorTarget = event.target instanceof HTMLElement && event.target.closest(".cm-editor");

    if (!isEditorTarget && !event.metaKey && !event.ctrlKey && !event.altKey) {
      if (event.key === "[") {
        if (isReviewPageOpen()) {
          event.preventDefault();
          closeReviewPage();
        }
        return;
      }

      if (event.key === "]") {
        if (!isReviewPageOpen()) {
          event.preventDefault();
          void runCommand("review.changes");
        }
        return;
      }
    }

    if (matchesShortcut(event, ["Mod", "Enter"])) {
      event.preventDefault();
      void runCommand("annotations.add");
      return;
    }
    if (matchesShortcut(event, ["Mod", "K"])) {
      event.preventDefault();
      openCommandPalette("default");
      return;
    }
    if (matchesShortcut(event, ["Mod", "P"])) {
      event.preventDefault();
      openCommandPalette("file");
      return;
    }
    if (matchesShortcut(event, ["Mod", "Shift", "R"])) {
      event.preventDefault();
      if (isReviewPageOpen()) {
        closeReviewPage();
      } else {
        void runCommand("review.changes");
      }
      return;
    }
    if (matchesShortcut(event, ["Mod", "Shift", "M"])) {
      event.preventDefault();
      void runCommand("view.toggleMarkdownPreview");
      return;
    }
    if (matchesShortcut(event, ["Mod", ","])) {
      event.preventDefault();
      void runCommand("view.openSettings");
      return;
    }
    if (matchesShortcut(event, ["Mod", "F"])) {
      event.preventDefault();
      editorRef()?.openSearch();
      return;
    }
    if (matchesShortcut(event, ["Mod", "G"])) {
      event.preventDefault();
      editorRef()?.navigateMatch(1);
      return;
    }
    if (matchesShortcut(event, ["Mod", "Shift", "G"])) {
      event.preventDefault();
      editorRef()?.navigateMatch(-1);
      return;
    }
    if (matchesShortcut(event, ["Mod", "B"])) {
      event.preventDefault();
      onToggleLeftPanel?.();
      return;
    }
    if (matchesShortcut(event, ["Mod", "Shift", "B"])) {
      event.preventDefault();
      onToggleRightPanel?.();
    }
  }

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  async function handleDroppedPath(path: string) {
    try {
      await readDirectory(path);
      await addRootFolder(path);
    } catch {
      const parentDir = path.substring(0, path.lastIndexOf("/"));
      if (parentDir) {
        await addRootFolder(parentDir);
      }
      await handleFileSelect(path);
    }
  }

  async function handleDeepLinkUrl(rawUrl: string) {
    try {
      const url = new URL(rawUrl);
      const action = url.hostname || "open";
      const filePath = url.searchParams.get("file");
      const prRef = url.searchParams.get("pr");
      const localPath = url.searchParams.get("localPath");
      const reviewSession = url.searchParams.get("reviewSession");

      if (action === "review-pr" && prRef) {
        await openGitHubPullRequest(prRef, localPath ?? undefined);
        return;
      }

      if (action === "refresh" && filePath) {
        if (editor.currentFilePath === filePath) {
          await loadAnnotations(filePath);
        }
        return;
      }

      const line = url.searchParams.get("line");
      if (!filePath) return;

      const gitRoot = await invoke<string | null>("get_git_root", { path: filePath });
      const rootDir = gitRoot ?? filePath.substring(0, filePath.lastIndexOf("/"));
      if (rootDir) {
        await addRootFolder(rootDir);
      }

      if (reviewSession) {
        activateReviewSession(reviewSession, [filePath]);
      } else {
        addReviewFile(filePath);
      }

      await handleFileSelect(filePath);

      if (line) {
        setTimeout(() => editorRef()?.scrollToLine(parseInt(line, 10)), jumpToLineDelayMs);
      }
    } catch (error) {
      console.error("Invalid deep link URL:", rawUrl, error);
    }
  }

  async function mount() {
    unlistenSettings = await listen("open-settings", () => {
      state.showSettings = true;
    });

    unlistenMenuItems = await Promise.all([
      listen("menu-open-folder", () => void openFolderPicker()),
      listen("menu-go-to-file", () => openCommandPalette("file")),
      listen("menu-command-palette", () => openCommandPalette("default")),
      listen("menu-export-annotations", async () => {
        try {
          if (!editor.currentFilePath) return;

          const markdown = await invoke<string>("export_annotations", {
            filePath: editor.currentFilePath,
          });
          if (!markdown) return;

          const destinationPath = await saveDialog({
            defaultPath: "annotations.md",
            filters: [{ name: "Markdown", extensions: ["md"] }],
          });
          if (destinationPath) {
            await writeTextFile(destinationPath, markdown);
          }
        } catch (error) {
          console.error("Failed to export annotations:", error);
        }
      }),
      listen("menu-add-annotation", () => void runCommand("annotations.add")),
      listen("menu-reload-annotations", () => void runCommand("annotations.reload")),
      listen("menu-clear-annotations", () => void runCommand("annotations.clear")),
      listen("menu-toggle-markdown-preview", () => void runCommand("view.toggleMarkdownPreview")),
      listen("menu-diff-split", () => void runCommand("diff.split")),
      listen("menu-diff-unified", () => void runCommand("diff.unified")),
      listen("menu-diff-highlights", () => void runCommand("diff.highlights")),
      listen("menu-diff-exit", () => void runCommand("diff.exit")),
      listen("menu-review-changes", () => void runCommand("review.changes")),
      listen("menu-agent-feedback", () => void runCommand("review.feedback")),
      listen("menu-approve-review", () => void runCommand("review.approve")),
      listen("menu-request-changes", () => void runCommand("review.requestChanges")),
      listen("menu-find", () => editorRef()?.openSearch()),
      listen("menu-find-next", () => editorRef()?.navigateMatch(1)),
      listen("menu-find-previous", () => editorRef()?.navigateMatch(-1)),
    ]);

    const appWindow = getCurrentWebviewWindow();
    unlistenDragDrop = await appWindow.onDragDropEvent(async (event) => {
      if (event.payload.type !== "drop") return;

      for (const path of event.payload.paths) {
        await handleDroppedPath(path);
      }
    });

    unlistenDeepLink = await onOpenUrl(async (urls: string[]) => {
      for (const rawUrl of urls) {
        await handleDeepLinkUrl(rawUrl);
      }
    });

    unlistenDeepLinkEvent = await listen<string>("deep-link-open", async (event) => {
      await handleDeepLinkUrl(event.payload);
    });

    unlistenGitHubReviewSession = await listen<GitHubPrSession>(
      "open-github-review-session",
      async (event) => {
        await activateGitHubReviewSession(event.payload, {
          refreshQueue: true,
          replaceRoots: true,
        });
      },
    );

    const pendingUrls = await invoke<string[]>("get_pending_deep_links");
    for (const rawUrl of pendingUrls) {
      await handleDeepLinkUrl(rawUrl);
    }
  }

  function destroy() {
    unlistenDeepLink?.();
    unlistenDeepLinkEvent?.();
    unlistenSettings?.();
    unlistenGitHubReviewSession?.();
    unlistenDragDrop?.();
    stopWatcher?.();
    unlistenMenuItems.forEach((unlisten) => unlisten());
  }

  return {
    state,
    commands,
    commandContext,
    closeAnnotationPopover,
    closeCommandPalette,
    closeReviewShortcutHelp,
    closeSettingsPanel,
    destroy,
    handleAnnotationClick,
    handleAnnotationSubmit,
    handleContextMenu,
    handleFileSelect,
    handleJumpToFile,
    handleKeydown,
    handleSelectionChange,
    mount,
    openCommandPalette,
    openFolderPicker,
    openReviewShortcutHelp,
    openSettingsPanel,
    runCommand,
  };
}
