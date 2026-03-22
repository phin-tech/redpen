<script lang="ts">
  import FileTree from "./components/FileTree.svelte";
  import Editor from "./components/Editor.svelte";
  import AnnotationSidebar from "./components/AnnotationSidebar.svelte";
  import AnnotationPopover from "./components/AnnotationPopover.svelte";
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import ResizeHandle from "./components/ResizeHandle.svelte";
  import { openFile, getEditor } from "$lib/stores/editor.svelte";
  import { loadAnnotations, addAnnotation } from "$lib/stores/annotations.svelte";
  import { addRootFolder } from "$lib/stores/workspace.svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
  import { listen } from "@tauri-apps/api/event";
  import { watch } from "@tauri-apps/plugin-fs";
  import { invoke } from "@tauri-apps/api/core";

  import { onMount, onDestroy } from "svelte";
  import { debounce } from "$lib/utils/debounce";
  const editor = getEditor();

  // Use ref pattern for Svelte 5 (not bind:this + export function)
  let editorRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);
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
            const { readDirectory } = await import("$lib/tauri");
            const entries = await readDirectory(path);
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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && showCommandPalette) {
      e.preventDefault();
      showCommandPalette = false;
      return;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      openAnnotationPopover();
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      commandPaletteMode = "default";
      showCommandPalette = true;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === ",") {
      e.preventDefault();
      showSettings = true;
    }
  }

  function handleKeyup(e: KeyboardEvent) {
    if (e.key === "Shift") {
      const now = Date.now();
      if (now - lastShiftKeyup < 300) {
        commandPaletteMode = "file";
        showCommandPalette = true;
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
      />
    </div>

    <ResizeHandle onResize={resizeLeft} />

    <div class="flex-1 min-w-[200px] overflow-hidden" style="box-shadow: var(--shadow-xs)">
      <Editor
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
    initialMode={commandPaletteMode}
    onClose={() => (showCommandPalette = false)}
    onOpenSettings={() => { showCommandPalette = false; showSettings = true; }}
    onAddAnnotation={() => { showCommandPalette = false; openAnnotationPopover(); }}
  />
</div>
