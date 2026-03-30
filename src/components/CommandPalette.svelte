<script lang="ts">
  import { Command, Dialog } from "bits-ui";
  import { onDestroy, tick } from "svelte";
  import { COMMAND_SECTIONS } from "$lib/commands";
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { queryWorkspaceFiles } from "$lib/tauri";
  import { formatShortcut } from "$lib/shortcuts";
  import type {
    AppCommandContext,
    AppCommandDefinition,
    CommandPaletteMode,
  } from "$lib/commands";
  import type { WorkspaceFileMatch, WorkspaceIndexStatus } from "$lib/types";

  let {
    open: isOpen,
    mode,
    onModeChange,
    onClose,
    commands,
    commandContext,
    onSelectFile,
  }: {
    open: boolean;
    mode: CommandPaletteMode;
    onModeChange: (mode: CommandPaletteMode) => void;
    onClose: () => void;
    commands: AppCommandDefinition[];
    commandContext: AppCommandContext;
    onSelectFile: (path: string) => Promise<void> | void;
  } = $props();

  const workspace = getWorkspace();

  let searchValue = $state("");
  let fileResults = $state<WorkspaceFileMatch[]>([]);
  let fileStatuses = $state<WorkspaceIndexStatus[]>([]);
  let loadingFiles = $state(false);
  let fileError = $state<string | null>(null);
  let contentEl: HTMLDivElement | undefined;
  let fileQueryVersion = 0;
  let paletteStateSignature = "";
  let pollTimer: ReturnType<typeof setTimeout> | null = null;

  let isIndexing = $derived(fileStatuses.some((status) => status.state === "indexing"));
  let isStale = $derived(fileStatuses.some((status) => status.state === "stale"));
  let isTruncated = $derived(fileStatuses.some((status) => status.truncated));
  let hasStatusError = $derived(fileStatuses.some((status) => status.state === "error"));

  $effect(() => {
    const signature = `${isOpen}:${mode}`;
    if (signature === paletteStateSignature) return;
    paletteStateSignature = signature;

    stopPolling();
    searchValue = "";
    fileResults = [];
    fileStatuses = [];
    loadingFiles = false;
    fileError = null;

    if (isOpen) {
      void focusInput();
    }
  });

  $effect(() => {
    if (!(isOpen && mode === "file")) return;

    const timeout = setTimeout(() => {
      void loadFiles();
    }, searchValue ? 120 : 0);

    return () => clearTimeout(timeout);
  });

  onDestroy(() => {
    stopPolling();
  });

  async function focusInput() {
    await tick();
    contentEl?.querySelector<HTMLInputElement>("input")?.focus();
  }

  function stopPolling() {
    if (pollTimer) {
      clearTimeout(pollTimer);
      pollTimer = null;
    }
  }

  function schedulePolling(statuses: WorkspaceIndexStatus[]) {
    stopPolling();
    if (!statuses.some((status) => status.state === "indexing" || status.state === "stale")) {
      return;
    }
    pollTimer = setTimeout(() => {
      if (isOpen && mode === "file") {
        void loadFiles();
      }
    }, 500);
  }

  async function loadFiles() {
    if (workspace.rootFolders.length === 0) {
      fileResults = [];
      fileStatuses = [];
      fileError = null;
      loadingFiles = false;
      stopPolling();
      return;
    }

    const queryVersion = ++fileQueryVersion;
    loadingFiles = true;
    fileError = null;

    try {
      const response = await queryWorkspaceFiles(searchValue, workspace.rootFolders);
      if (queryVersion !== fileQueryVersion) return;
      fileResults = response.results;
      fileStatuses = response.statuses;
      schedulePolling(response.statuses);
    } catch (error) {
      if (queryVersion !== fileQueryVersion) return;
      fileResults = [];
      fileStatuses = [];
      fileError = error instanceof Error ? error.message : String(error);
      stopPolling();
    } finally {
      if (queryVersion === fileQueryVersion) {
        loadingFiles = false;
      }
    }
  }

  function close() {
    stopPolling();
    onClose();
  }

  async function closeThen(action: () => Promise<void> | void) {
    close();
    await action();
  }

  function isCommandEnabled(command: AppCommandDefinition): boolean {
    return command.isEnabled ? command.isEnabled(commandContext) : true;
  }

  async function runCommand(command: AppCommandDefinition) {
    if (!isCommandEnabled(command)) return;
    if (command.closeOnRun === false) {
      await command.run(commandContext);
      return;
    }
    await closeThen(() => command.run(commandContext));
  }

  function rootLabel(root: string): string {
    return root.split("/").pop() ?? root;
  }
</script>

<Dialog.Root
  open={isOpen}
  onOpenChange={(value) => {
    if (!value) close();
  }}
  onOpenChangeComplete={(value) => {
    if (value) void focusInput();
  }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-[2px]" />
    <Dialog.Content
      aria-label="Command palette"
      class="fixed left-1/2 top-20 z-50 -translate-x-1/2 w-[520px] rounded-xl border border-border-emphasis overflow-hidden focus:outline-none"
      style="background: var(--surface-panel); box-shadow: var(--shadow-popover), 0 0 0 1px var(--border-subtle)"
    >
      <div bind:this={contentEl}>
        <Command.Root loop shouldFilter={mode === "default"} class="flex flex-col">
          <div class="flex items-center gap-2.5 px-3.5 py-3 border-b border-border-default/60">
            <svg class="text-text-muted shrink-0" width="14" height="14" viewBox="0 0 16 16" fill="none">
              <circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.5" />
              <path d="M11 11L14.5 14.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
            <Command.Input
              bind:value={searchValue}
              placeholder={mode === "file" ? "Search files..." : "Type a command or search..."}
              class="flex-1 bg-transparent border-none outline-none text-sm text-text-primary placeholder:text-text-muted"
            />
            {#if mode === "file"}
              <button
                onclick={() => onModeChange("default")}
                class="text-[10px] text-text-muted hover:text-text-secondary px-1.5 py-0.5 rounded border border-border-default/60 bg-surface-raised transition-colors"
              >
                back
              </button>
            {/if}
          </div>

          <Command.List class="max-h-[360px] overflow-y-auto py-1">
            {#if mode === "default"}
              {#each COMMAND_SECTIONS as section}
                {@const sectionCommands = commands.filter((command) => command.section === section)}
                {#if sectionCommands.length > 0}
                  <Command.Group>
                    <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                      {section}
                    </Command.GroupHeading>
                    <Command.GroupItems>
                      {#each sectionCommands as command (command.id)}
                        <Command.Item
                          value={command.id}
                          keywords={command.keywords}
                          disabled={!isCommandEnabled(command)}
                          onSelect={() => {
                            void runCommand(command);
                          }}
                          class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[selected]:bg-accent-subtle data-[disabled]:opacity-50"
                        >
                          <span class="flex-1 text-sm text-text-secondary">{command.title}</span>
                          {#if command.shortcut}
                            <div class="flex gap-1">
                              {#each formatShortcut(command.shortcut) as key}
                                <kbd class="inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono rounded bg-surface-raised text-text-muted border border-border-default">
                                  {key}
                                </kbd>
                              {/each}
                            </div>
                          {/if}
                        </Command.Item>
                      {/each}
                    </Command.GroupItems>
                  </Command.Group>
                {/if}
              {/each}

              <Command.Empty class="px-3.5 py-8 text-sm text-text-muted text-center">
                No commands found
              </Command.Empty>
            {:else}
              {#if fileError}
                <div class="px-3.5 py-2 text-xs text-danger">
                  {fileError}
                </div>
              {:else}
                {#if isIndexing}
                  <div class="px-3.5 py-2 text-[11px] text-text-muted">
                    Indexing workspace files...
                  </div>
                {:else if isStale}
                  <div class="px-3.5 py-2 text-[11px] text-text-muted">
                    Refreshing file index...
                  </div>
                {:else if hasStatusError}
                  <div class="px-3.5 py-2 text-[11px] text-danger">
                    Some roots failed to index.
                  </div>
                {/if}

                {#if isTruncated}
                  <div class="px-3.5 py-2 text-[11px] text-text-muted border-b border-border-default/40">
                    Some files are omitted due to indexing limits.
                  </div>
                {/if}

                <Command.Group>
                  <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                    {loadingFiles ? "Loading files..." : "Files"}
                  </Command.GroupHeading>
                  <Command.GroupItems>
                    {#each fileResults as file (file.path)}
                      <Command.Item
                        value={file.relativePath}
                        keywords={[file.name, file.path, file.root]}
                        onSelect={() => {
                          void closeThen(() => onSelectFile(file.path));
                        }}
                        class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[selected]:bg-accent-subtle"
                      >
                        <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-text-muted">
                          <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                            <path d="M2 10V2a1 1 0 011-1h4l3 3v6a1 1 0 01-1 1H3a1 1 0 01-1-1z" stroke="currentColor" stroke-width="1.2" />
                            <path d="M7 1v3h3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
                          </svg>
                        </div>
                        <div class="flex-1 min-w-0">
                          <div class="text-sm text-text-secondary truncate">{file.name}</div>
                          <div class="text-[11px] text-text-muted font-mono truncate">{file.relativePath}</div>
                        </div>
                        <span class="text-[10px] uppercase tracking-wide text-text-muted shrink-0">
                          {rootLabel(file.root)}
                        </span>
                      </Command.Item>
                    {/each}
                  </Command.GroupItems>
                </Command.Group>

                {#if workspace.rootFolders.length === 0}
                  <div class="px-3.5 py-8 text-sm text-text-muted text-center">
                    No workspace folders open
                  </div>
                {:else if !loadingFiles && fileResults.length === 0}
                  <div class="px-3.5 py-8 text-sm text-text-muted text-center">
                    No files found
                  </div>
                {/if}
              {/if}
            {/if}
          </Command.List>

          <div class="border-t border-border-default/50 px-3.5 py-2 flex items-center gap-4" style="background: rgba(0,0,0,0.15)">
            <div class="flex items-center gap-1.5 text-[11px] text-text-muted">
              <kbd class="inline-flex items-center px-1 py-0.5 text-[10px] font-mono rounded bg-surface-raised border border-border-default">↑↓</kbd>
              navigate
            </div>
            <div class="flex items-center gap-1.5 text-[11px] text-text-muted">
              <kbd class="inline-flex items-center px-1 py-0.5 text-[10px] font-mono rounded bg-surface-raised border border-border-default">↵</kbd>
              select
            </div>
            <div class="flex items-center gap-1.5 text-[11px] text-text-muted">
              <kbd class="inline-flex items-center px-1 py-0.5 text-[10px] font-mono rounded bg-surface-raised border border-border-default">esc</kbd>
              close
            </div>
          </div>
        </Command.Root>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
