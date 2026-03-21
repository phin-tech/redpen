<script lang="ts">
  import Toolbar from "./components/Toolbar.svelte";
  import FileTree from "./components/FileTree.svelte";
  import Editor from "./components/Editor.svelte";
  import AnnotationSidebar from "./components/AnnotationSidebar.svelte";
  import AnnotationPopover from "./components/AnnotationPopover.svelte";
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import { openFile, getEditor } from "$lib/stores/editor.svelte";
  import { loadAnnotations, addAnnotation } from "$lib/stores/annotations.svelte";
  import type { AnnotationKind } from "$lib/types";

  const editor = getEditor();

  // Use ref pattern for Svelte 5 (not bind:this + export function)
  let editorRef: { scrollToLine: (line: number) => void } | undefined = $state(undefined);
  let showSettings = $state(false);

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
  }

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

  // Cmd+Return to create annotation, Cmd+Shift+Return for line note
  let initialAnnotationKind: AnnotationKind = $state("comment");

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && selection && editor.currentFilePath) {
      e.preventDefault();
      initialAnnotationKind = e.shiftKey ? "lineNote" : "comment";
      // Position popover near center of viewport
      popoverPosition = { x: window.innerWidth / 2 - 160, y: window.innerHeight / 3 };
      showPopover = true;
    }
  }

  async function handleAnnotationSubmit(
    body: string,
    labels: string[],
    kind: AnnotationKind
  ) {
    if (!selection || !editor.currentFilePath) return;
    await addAnnotation(
      editor.currentFilePath,
      kind,
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

<svelte:window onkeydown={handleKeydown} />

<div id="app">
  <Toolbar onSettingsClick={() => (showSettings = true)} />

  <div class="main-layout">
    <div class="panel-left">
      <FileTree
        onFileSelect={handleFileSelect}
        selectedPath={editor.currentFilePath}
      />
    </div>

    <div class="panel-center">
      <Editor
        bind:ref={editorRef}
        onSelectionChange={handleSelectionChange}
      />
    </div>

    <div class="panel-right">
      <AnnotationSidebar onAnnotationClick={handleAnnotationClick} />
    </div>
  </div>

  {#if showPopover}
    <AnnotationPopover
      position={popoverPosition}
      initialKind={initialAnnotationKind}
      onSubmit={handleAnnotationSubmit}
      onCancel={() => (showPopover = false)}
    />
  {/if}

  {#if showSettings}
    <SettingsDialog onClose={() => (showSettings = false)} />
  {/if}
</div>

<style>
  .main-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .panel-left {
    width: 240px;
    min-width: 180px;
    border-right: 1px solid var(--border-color);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .panel-center {
    flex: 1;
    overflow: hidden;
  }

  .panel-right {
    width: 300px;
    min-width: 200px;
    border-left: 1px solid var(--border-color);
    overflow: hidden;
  }
</style>
