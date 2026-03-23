<script lang="ts">
  import FileTree from "./components/FileTree.svelte";
  import EditorPane from "./components/EditorPane.svelte";
  import AnnotationSidebar from "./components/AnnotationSidebar.svelte";
  import AnnotationPopover from "./components/AnnotationPopover.svelte";
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import ResizeHandle from "./components/ResizeHandle.svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { readDirectory } from "$lib/tauri";
  import { openFile, getEditor, isMarkdownFile, togglePreview } from "$lib/stores/editor.svelte";
  import { loadAnnotations, addAnnotation, clearAllAnnotations, getAnnotationsState } from "$lib/stores/annotations.svelte";
  import { addReviewFile } from "$lib/stores/review.svelte";
  import {
    addRootFolder,
    getWorkspace,
    expandAllFolders,
    collapseAllFolders,
    toggleShowChangedOnly,
  } from "$lib/stores/workspace.svelte";
  import { createCommandRegistry, findCommand } from "$lib/commands";
  import type { AppCommandContext } from "$lib/commands";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
  import { listen } from "@tauri-apps/api/event";
  import { watch } from "@tauri-apps/plugin-fs";
  import { invoke } from "@tauri-apps/api/core";

  import { onMount, onDestroy } from "svelte";
  import { debounce } from "$lib/utils/debounce";
  const editor = getEditor();
  const workspace = getWorkspace();
  const commands = createCommandRegistry();

  // Use ref pattern for Svelte 5 (not bind:this + export function)
  let editorRef: { scrollToLine: (line: number) => void; openSearch: () => void; closeSearch: () => void; navigateMatch: (dir: 1 | -1) => void } | undefined = $state(undefined);
  let showSettings = $state(false);
  let showCommandPalette = $state(false);
  let commandPaletteMode = $state<"default" | "file">("default");

  // Double-shift detection
  let lastShiftKeyup = 0;

  // Resizable panel widths
  let leftPanelWidth = $state(240);
  let rightPanelWidth = $state(300);

  function resizeLeft(delta: number) {
    leftPanelWidth = Math.max(140, Math.min(500, leftPanelWidth + delta));
  }

  function resizeRight(delta: number) {
    rightPanelWidth = Math.max(160, Math.min(600, rightPanelWidth - delta));
  }

  // File watcher cleanup
  let stopWatcher: (() => void) | null = null;

  // Selection state for annotation creation
  let selection: {
    fromLine: number;
    fromCol: number;
    toLine: number;
    toCol: number;
  } | null = $state(null);
  let showPopover = $state(false);
  let popoverPosition = $state({ x: 0, y: 0 });

  async function handleFileSelect(path: string) {
    await openFile(path);
    await loadAnnotations(path);

    // Set up file watcher for source change detection
    stopWatcher?.();
    const reloadFile = debounce(async () => {
      if (editor.currentFilePath) {
        await openFile(editor.currentFilePath);
        await loadAnnotations(editor.currentFilePath);
      }
    }, 500);
    stopWatcher = await watch(path, reloadFile, { recursive: false });
  }

  // Deep link cleanup functions
  let unlistenDeepLink: (() => void) | undefined;
  let unlistenDeepLinkEvent: (() => void) | undefined;
  let unlistenSettings: (() => void) | undefined;

  onMount(async () => {
    // Listen for settings menu event from native menu bar
    unlistenSettings = await listen("open-settings", () => {
      showSettings = true;
    });

    // Drag-and-drop handling
    const appWindow = getCurrentWebviewWindow();
    await appWindow.onDragDropEvent(async (event) => {
      if (event.payload.type === "drop") {
        for (const path of event.payload.paths) {
          try {
            await readDirectory(path);
            // If readDirectory succeeds, it's a directory — add as root
            await addRootFolder(path);
          } catch {
            // It's a file — open it directly
            // Add its parent directory as a root folder, then open the file
            const parentDir = path.substring(0, path.lastIndexOf("/"));
            if (parentDir) await addRootFolder(parentDir);
            await handleFileSelect(path);
          }
        }
      }
    });

    // Deep link handling
    async function handleDeepLinkUrl(rawUrl: string) {
      try {
        const url = new URL(rawUrl);
        const filePath = url.searchParams.get("file");
        const line = url.searchParams.get("line");

        if (filePath) {
          // Use git repo root if available, otherwise fall back to parent directory
          const gitRoot = await invoke<string | null>("get_git_root", { path: filePath });
          const rootDir = gitRoot ?? filePath.substring(0, filePath.lastIndexOf("/"));
          if (rootDir) await addRootFolder(rootDir);
          addReviewFile(filePath);
          await handleFileSelect(filePath);
          if (line) {
            setTimeout(() => editorRef?.scrollToLine(parseInt(line)), 100);
          }
        }
      } catch (e) {
        console.error("Invalid deep link URL:", rawUrl, e);
      }
    }

    // Listen for deep links while app is running (warm start)
    unlistenDeepLink = await onOpenUrl(async (urls: string[]) => {
      for (const rawUrl of urls) {
        await handleDeepLinkUrl(rawUrl);
      }
    });

    unlistenDeepLinkEvent = await listen<string>("deep-link-open", async (event) => {
      await handleDeepLinkUrl(event.payload);
    });

    // Check for cold-start deep links stored in Rust state
    const pendingUrls = await invoke<string[]>("get_pending_deep_links");
    for (const rawUrl of pendingUrls) {
      await handleDeepLinkUrl(rawUrl);
    }
  });

  onDestroy(() => {
    unlistenDeepLink?.();
    unlistenDeepLinkEvent?.();
    unlistenSettings?.();
    stopWatcher?.();
  });

  function handleSelectionChange(
    fromLine: number,
    fromCol: number,
    toLine: number,
    toCol: number
  ) {
    selection = { fromLine, fromCol, toLine, toCol };
  }

  function handleAnnotationClick(line: number) {
    editorRef?.scrollToLine(line);
  }

  function openAnnotationPopover() {
    if (selection && editor.currentFilePath) {
      popoverPosition = { x: window.innerWidth / 2 - 160, y: window.innerHeight / 3 };
      showPopover = true;
    }
  }

  function openCommandPalette(mode: "default" | "file") {
    commandPaletteMode = mode;
    showCommandPalette = true;
  }

  function openSettingsPanel() {
    showCommandPalette = false;
    showSettings = true;
  }

  function openAnnotationCommand() {
    showCommandPalette = false;
    openAnnotationPopover();
  }

  async function openFolderPicker() {
    const selected = await openDialog({ directory: true, multiple: true });
    if (!selected) return;
    for (const path of Array.isArray(selected) ? selected : [selected]) {
      if (path) await addRootFolder(path);
    }
  }

  const commandContext: AppCommandContext = {
    openCommandPalette,
    openFolder: openFolderPicker,
    openSettings: openSettingsPanel,
    openAddAnnotation: openAnnotationCommand,
    expandAllFolders,
    collapseAllFolders,
    toggleShowChangedOnly,
    hasRoots: () => workspace.rootFolders.length > 0,
    canAddAnnotation: () => Boolean(selection && editor.currentFilePath),
    hasAnnotations: () => {
      const annotationsState = getAnnotationsState();
      return Boolean(editor.currentFilePath && annotationsState.sidecar && annotationsState.sidecar.annotations.length > 0);
    },
    clearAnnotations: async () => {
      if (editor.currentFilePath) {
        await clearAllAnnotations(editor.currentFilePath);
      }
    },
    isMarkdownFile,
    toggleMarkdownPreview: togglePreview,
  };

  async function runCommand(id: string) {
    const command = findCommand(commands, id);
    if (!command) return;
    if (command.isEnabled && !command.isEnabled(commandContext)) return;
    await command.run(commandContext);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && showCommandPalette) {
      e.preventDefault();
      showCommandPalette = false;
      return;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      void runCommand("annotations.add");
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      openCommandPalette("default");
    }
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "m") {
      e.preventDefault();
      void runCommand("view.toggleMarkdownPreview");
      return;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === ",") {
      e.preventDefault();
      void runCommand("view.openSettings");
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "f") {
      e.preventDefault();
      editorRef?.openSearch();
    }
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "g") {
      e.preventDefault();
      editorRef?.navigateMatch(e.shiftKey ? -1 : 1);
    }
  }

  function handleKeyup(e: KeyboardEvent) {
    if (e.key === "Shift") {
      const now = Date.now();
      if (now - lastShiftKeyup < 300) {
        openCommandPalette("file");
        lastShiftKeyup = 0;
      } else {
        lastShiftKeyup = now;
      }
    }
  }

  async function handleAnnotationSubmit(body: string, labels: string[]) {
    if (!selection || !editor.currentFilePath) return;
    await addAnnotation(
      editor.currentFilePath,
      body,
      labels,
      selection.fromLine,
      selection.fromCol,
      selection.toLine,
      selection.toCol
    );
    showPopover = false;
    selection = null;
  }
</script>

<svelte:window onkeydown={handleKeydown} onkeyup={handleKeyup} oncontextmenu={(e) => e.preventDefault()} />

<div class="app-root">
  <div class="flex flex-1 overflow-hidden">
    <div class="shrink-0 border-r border-border-default/50 overflow-hidden" style="width: {leftPanelWidth}px; background: var(--gradient-panel), var(--surface-panel); box-shadow: inset -1px 0 0 var(--border-subtle)">
      <FileTree
        onFileSelect={handleFileSelect}
        selectedPath={editor.currentFilePath}
        onOpenFolder={openFolderPicker}
        onExpandAll={expandAllFolders}
        onCollapseAll={collapseAllFolders}
        onToggleShowChangedOnly={toggleShowChangedOnly}
      />
    </div>

    <ResizeHandle onResize={resizeLeft} />

    <div class="flex-1 min-w-[200px] overflow-hidden" style="box-shadow: var(--shadow-xs)">
      <EditorPane
        bind:ref={editorRef}
        onSelectionChange={handleSelectionChange}
      />
    </div>

    <ResizeHandle onResize={resizeRight} />

    <div class="shrink-0 border-l border-border-default/50 overflow-hidden" style="width: {rightPanelWidth}px; background: var(--gradient-panel), var(--surface-panel); box-shadow: inset 1px 0 0 var(--border-subtle)">
      <AnnotationSidebar onAnnotationClick={handleAnnotationClick} onFileSelect={handleFileSelect} />
    </div>
  </div>

  {#if showPopover}
    <AnnotationPopover
      position={popoverPosition}
      onSubmit={handleAnnotationSubmit}
      onCancel={() => (showPopover = false)}
    />
  {/if}

  {#if showSettings}
    <SettingsDialog onClose={() => (showSettings = false)} />
  {/if}

  <CommandPalette
    open={showCommandPalette}
    mode={commandPaletteMode}
    onModeChange={openCommandPalette}
    onClose={() => (showCommandPalette = false)}
    commands={commands}
    commandContext={commandContext}
    onSelectFile={handleFileSelect}
  />
</div>
