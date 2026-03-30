<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";

  import AnnotationPopover from "./components/AnnotationPopover.svelte";
  import AnnotationSidebar from "./components/AnnotationSidebar.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import EditorPane from "./components/EditorPane.svelte";
  import FileTree from "./components/FileTree.svelte";
  import ResizeHandle from "./components/ResizeHandle.svelte";
  import ReviewWorkspaceHeader from "./components/ReviewWorkspaceHeader.svelte";
  import SettingsDialog from "./components/SettingsDialog.svelte";
  import { createAppShellController, type AppEditorRef } from "$lib/controllers/appShell.svelte";
  import { getDiffState } from "$lib/stores/diff.svelte";
  import { getEditor } from "$lib/stores/editor.svelte";

  const diff = getDiffState();
  const editor = getEditor();

  let editorRef: AppEditorRef | undefined = $state(undefined);
  let savedLeftPanelWidth = $state(240);
  let leftPanelWidth = $state(240);
  let rightPanelWidth = $state(300);

  const appShell = createAppShellController({
    getEditorRef: () => editorRef,
  });

  function resizeLeft(delta: number) {
    leftPanelWidth = Math.max(140, Math.min(500, leftPanelWidth + delta));
  }

  function resizeRight(delta: number) {
    rightPanelWidth = Math.max(160, Math.min(600, rightPanelWidth - delta));
  }

  onMount(async () => {
    await appShell.mount();
  });

  onDestroy(() => {
    appShell.destroy();
  });

  $effect(() => {
    if (diff.enabled && diff.mode === "split") {
      const current = untrack(() => leftPanelWidth);
      if (current > 0) {
        savedLeftPanelWidth = current;
      }
      if (current !== 0) {
        leftPanelWidth = 0;
      }
    } else {
      const saved = untrack(() => savedLeftPanelWidth);
      if (untrack(() => leftPanelWidth) === 0 && saved > 0) {
        leftPanelWidth = saved;
      }
    }
  });
</script>

<svelte:window
  onkeydown={appShell.handleKeydown}
  oncontextmenu={appShell.handleContextMenu}
/>

<div class="app-root">
  <ReviewWorkspaceHeader onOpenHelp={appShell.openReviewShortcutHelp} />

  <div class="workspace-shell">
    <section class="app-panel app-panel-nav" style={`width: ${leftPanelWidth}px;`}>
      <FileTree
        onFileSelect={appShell.handleFileSelect}
        selectedPath={editor.currentFilePath}
        onOpenFolder={appShell.openFolderPicker}
        onExpandAll={appShell.commandContext.expandAllFolders}
        onCollapseAll={appShell.commandContext.collapseAllFolders}
        onToggleShowChangedOnly={appShell.commandContext.toggleShowChangedOnly}
      />
    </section>

    <ResizeHandle onResize={resizeLeft} />

    <section class="app-panel app-panel-workspace">
      <EditorPane
        bind:ref={editorRef}
        bind:showShortcutHelp={appShell.state.showReviewShortcutHelp}
        onSelectionChange={appShell.handleSelectionChange}
        onOpenFolder={appShell.openFolderPicker}
        onJumpToFile={appShell.handleJumpToFile}
      />
    </section>

    <ResizeHandle onResize={resizeRight} />

    <section class="app-panel app-panel-sidebar" style={`width: ${rightPanelWidth}px;`}>
      <AnnotationSidebar
        onAnnotationClick={appShell.handleAnnotationClick}
        onFileSelect={appShell.handleFileSelect}
      />
    </section>
  </div>

  {#if appShell.state.showPopover}
    <AnnotationPopover
      position={appShell.state.popoverPosition}
      onSubmit={appShell.handleAnnotationSubmit}
      onCancel={appShell.closeAnnotationPopover}
    />
  {/if}

  {#if appShell.state.showSettings}
    <SettingsDialog onClose={appShell.closeSettingsPanel} />
  {/if}

  <CommandPalette
    open={appShell.state.showCommandPalette}
    mode={appShell.state.commandPaletteMode}
    onModeChange={appShell.openCommandPalette}
    onClose={appShell.closeCommandPalette}
    commands={appShell.commands}
    commandContext={appShell.commandContext}
    onSelectFile={appShell.handleFileSelect}
  />
</div>

<style>
  .app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background:
      radial-gradient(circle at top, color-mix(in srgb, var(--accent) 10%, transparent), transparent 45%),
      var(--surface-base);
  }
  .workspace-shell {
    flex: 1;
    display: flex;
    min-height: 0;
    padding: 10px;
    gap: 0;
  }
  .app-panel {
    min-height: 0;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--border-default) 75%, transparent);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--surface-panel) 96%, white 4%), transparent 100%),
      var(--gradient-panel),
      var(--surface-panel);
    box-shadow: var(--shadow-xs);
  }
  .app-panel-nav,
  .app-panel-sidebar {
    flex: 0 0 auto;
  }
  .app-panel-workspace {
    flex: 1;
    min-width: 200px;
    border-left: 0;
    border-right: 0;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--surface-panel) 94%, white 6%), transparent 100%),
      var(--surface-base);
  }
</style>
