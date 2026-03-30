<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";

  import AnnotationPopover from "./components/AnnotationPopover.svelte";
  import AnnotationSidebar from "./components/AnnotationSidebar.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import EditorPane from "./components/EditorPane.svelte";
  import FileTree from "./components/FileTree.svelte";
  import ResizeHandle from "./components/ResizeHandle.svelte";
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
  let savedRightPanelWidth = $state(300);

  function toggleLeftPanel() {
    if (leftPanelWidth > 0) {
      savedLeftPanelWidth = leftPanelWidth;
      leftPanelWidth = 0;
    } else {
      leftPanelWidth = savedLeftPanelWidth > 0 ? savedLeftPanelWidth : 240;
    }
  }

  function toggleRightPanel() {
    if (rightPanelWidth > 0) {
      savedRightPanelWidth = rightPanelWidth;
      rightPanelWidth = 0;
    } else {
      rightPanelWidth = savedRightPanelWidth > 0 ? savedRightPanelWidth : 300;
    }
  }

  const appShell = createAppShellController({
    getEditorRef: () => editorRef,
    onToggleLeftPanel: () => toggleLeftPanel(),
    onToggleRightPanel: () => toggleRightPanel(),
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
  <div class="workspace-shell">
    {#if leftPanelWidth === 0}
      <button class="panel-rail panel-rail-left" onclick={toggleLeftPanel} title="Expand file tree (⌘B)">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 18l6-6-6-6" />
        </svg>
      </button>
    {:else}
      <section class="app-panel app-panel-nav" style={`width: ${leftPanelWidth}px;`}>
        <FileTree
          onFileSelect={appShell.handleFileSelect}
          selectedPath={editor.currentFilePath}
          onOpenFolder={appShell.openFolderPicker}
          onExpandAll={appShell.commandContext.expandAllFolders}
          onCollapseAll={appShell.commandContext.collapseAllFolders}
          onToggleShowChangedOnly={appShell.commandContext.toggleShowChangedOnly}
          onCollapse={toggleLeftPanel}
        />
      </section>
      <ResizeHandle onResize={resizeLeft} />
    {/if}

    <section class="app-panel app-panel-workspace">
      <EditorPane
        bind:ref={editorRef}
        bind:showShortcutHelp={appShell.state.showReviewShortcutHelp}
        onSelectionChange={appShell.handleSelectionChange}
        onOpenFolder={appShell.openFolderPicker}
        onJumpToFile={appShell.handleJumpToFile}
      />
    </section>

    {#if rightPanelWidth === 0}
      <button class="panel-rail panel-rail-right" onclick={toggleRightPanel} title="Expand annotations (⌘⇧B)">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M15 18l-6-6 6-6" />
        </svg>
      </button>
    {:else}
      <ResizeHandle onResize={resizeRight} />
      <section class="app-panel app-panel-sidebar" style={`width: ${rightPanelWidth}px;`}>
        <AnnotationSidebar
          onAnnotationClick={appShell.handleAnnotationClick}
          onFileSelect={appShell.handleFileSelect}
          onCollapse={toggleRightPanel}
        />
      </section>
    {/if}
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
    background: var(--surface-base);
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
    background: var(--surface-panel);
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
    background: var(--surface-base);
  }
  .panel-rail {
    width: 24px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 8px;
    background: var(--surface-panel);
    border: 1px solid color-mix(in srgb, var(--border-default) 75%, transparent);
    cursor: pointer;
    color: var(--text-ghost);
  }
  .panel-rail:hover {
    color: var(--text-muted);
    background: var(--surface-raised);
  }
</style>
