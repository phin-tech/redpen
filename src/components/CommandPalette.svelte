<script lang="ts">
  import { Command, Dialog } from "bits-ui";
  import { tick } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    getWorkspace,
    addRootFolder,
    expandAllFolders,
    collapseAllFolders,
    toggleShowChangedOnly,
  } from "$lib/stores/workspace.svelte";
  import { openFile } from "$lib/stores/editor.svelte";
  import type { FileEntry } from "$lib/types";

  let {
    open: isOpen,
    onClose,
    initialMode = "default",
    onOpenSettings,
    onAddAnnotation,
  }: {
    open: boolean;
    onClose: () => void;
    initialMode?: "default" | "file";
    onOpenSettings: () => void;
    onAddAnnotation: () => void;
  } = $props();

  const workspace = getWorkspace();

  let mode = $state<"default" | "file">("default");
  let loadingFiles = $state(false);
  let fileList = $state<FileEntry[]>([]);
  let searchValue = $state("");
  let contentEl: HTMLDivElement | undefined;

  $effect(() => {
    if (isOpen) {
      mode = initialMode;
      searchValue = "";
      fileList = [];
      if (initialMode === "file") {
        (async () => await loadFiles())();
      }
    }
  });

  async function focusInput() {
    await tick();
    contentEl?.querySelector<HTMLInputElement>("input")?.focus();
  }

  async function loadFiles() {
    loadingFiles = true;
    await expandAllFolders();
    const files: FileEntry[] = [];
    for (const entries of workspace.fileTree.values()) {
      for (const entry of entries) {
        if (!entry.isDir) files.push(entry);
      }
    }
    fileList = files;
    loadingFiles = false;
  }

  async function enterFileMode() {
    mode = "file";
    searchValue = "";
    await loadFiles();
    await focusInput();
  }

  function close() {
    onClose();
  }

  async function handleOpenFolder() {
    close();
    const paths = await open({ directory: true, multiple: true });
    if (paths) {
      for (const path of Array.isArray(paths) ? paths : [paths]) {
        await addRootFolder(path);
      }
    }
  }

  function shortPath(path: string): string {
    return path.replace(/^\/Users\/[^/]+/, "~");
  }
</script>

<Dialog.Root
  open={isOpen}
  onOpenChange={(v) => { if (!v) close(); }}
  onOpenChangeComplete={(v) => { if (v) focusInput(); }}
>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60 backdrop-blur-[2px]" />
    <Dialog.Content
      aria-label="Command palette"
      class="fixed left-1/2 top-20 z-50 -translate-x-1/2 w-[520px] rounded-xl border border-border-emphasis overflow-hidden focus:outline-none"
      style="background: linear-gradient(180deg, rgba(255,255,255,0.02) 0%, transparent 100%), var(--surface-panel); box-shadow: var(--shadow-popover), 0 0 0 1px var(--border-subtle)"
    >
      <div bind:this={contentEl}>
        <Command.Root loop class="flex flex-col">
          <!-- Search input row -->
          <div class="flex items-center gap-2.5 px-3.5 py-3 border-b border-border-default/60">
            <svg class="text-text-muted shrink-0" width="14" height="14" viewBox="0 0 16 16" fill="none">
              <circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.5" />
              <path d="M11 11L14.5 14.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
            <Command.Input
              bind:value={searchValue}
              placeholder={mode === "file" ? "Search files…" : "Type a command or search…"}
              class="flex-1 bg-transparent border-none outline-none text-sm text-text-primary placeholder:text-text-muted"
            />
            {#if mode === "file"}
              <button
                onclick={() => { mode = "default"; searchValue = ""; focusInput(); }}
                class="text-[10px] text-text-muted hover:text-text-secondary px-1.5 py-0.5 rounded border border-border-default/60 bg-surface-raised transition-colors"
              >
                back
              </button>
            {/if}
          </div>

          <!-- Command list -->
          <Command.List class="max-h-[360px] overflow-y-auto py-1">
            {#if mode === "default"}
              <!-- Navigation -->
              <Command.Group>
                <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                  Navigation
                </Command.GroupHeading>
                <Command.GroupItems>
                  <Command.Item
                    value="go to file"
                    onSelect={() => { (async () => await enterFileMode())(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-accent-blue">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M2 10V3a1 1 0 011-1h4l3 3v5a1 1 0 01-1 1H3a1 1 0 01-1-1z" stroke="currentColor" stroke-width="1.2" />
                        <path d="M7 2v3h3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">Go to file…</span>
                  </Command.Item>
                  <Command.Item
                    value="open folder"
                    onSelect={() => { (async () => await handleOpenFolder())(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-accent-teal">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M1 9V4.5a1 1 0 011-1h2.5l1 1H10a1 1 0 011 1V9a1 1 0 01-1 1H2a1 1 0 01-1-1z" stroke="currentColor" stroke-width="1.2" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">Open folder…</span>
                  </Command.Item>
                </Command.GroupItems>
              </Command.Group>

              <!-- Workspace -->
              <Command.Group>
                <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                  Workspace
                </Command.GroupHeading>
                <Command.GroupItems>
                  <Command.Item
                    value="expand all folders"
                    onSelect={() => { (async () => { await expandAllFolders(); close(); })(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-text-muted">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M2 4l4 4 4-4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">Expand all folders</span>
                  </Command.Item>
                  <Command.Item
                    value="collapse all folders"
                    onSelect={() => { collapseAllFolders(); close(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-text-muted">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M2 8l4-4 4 4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">Collapse all folders</span>
                  </Command.Item>
                  <Command.Item
                    value="toggle changed files only show changed"
                    onSelect={() => { toggleShowChangedOnly(); close(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60" style="color: var(--color-warning)">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <circle cx="6" cy="6" r="2" fill="currentColor" />
                        <circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.2" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">
                      {workspace.showChangedOnly ? "Show all files" : "Show changed files only"}
                    </span>
                  </Command.Item>
                </Command.GroupItems>
              </Command.Group>

              <!-- Annotations -->
              <Command.Group>
                <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                  Annotations
                </Command.GroupHeading>
                <Command.GroupItems>
                  <Command.Item
                    value="add annotation comment"
                    onSelect={() => { close(); onAddAnnotation(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0" style="background: rgba(244,122,99,0.1); border: 1px solid rgba(244,122,99,0.25); color: var(--accent)">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M6 1v10M1 6h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">Add annotation</span>
                    <div class="flex gap-1">
                      <kbd class="inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono rounded bg-surface-raised text-text-muted border border-border-default">⌘</kbd>
                      <kbd class="inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono rounded bg-surface-raised text-text-muted border border-border-default">↵</kbd>
                    </div>
                  </Command.Item>
                </Command.GroupItems>
              </Command.Group>

              <!-- View -->
              <Command.Group>
                <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                  View
                </Command.GroupHeading>
                <Command.GroupItems>
                  <Command.Item
                    value="open settings preferences"
                    onSelect={() => { close(); onOpenSettings(); }}
                    class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                  >
                    <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-accent-purple">
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <circle cx="6" cy="6" r="2" stroke="currentColor" stroke-width="1.2" />
                        <path d="M6 1v1.5M6 9.5V11M1 6h1.5M9.5 6H11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
                      </svg>
                    </div>
                    <span class="flex-1 text-sm text-text-secondary">Open settings</span>
                    <div class="flex gap-1">
                      <kbd class="inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono rounded bg-surface-raised text-text-muted border border-border-default">⌘</kbd>
                      <kbd class="inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono rounded bg-surface-raised text-text-muted border border-border-default">,</kbd>
                    </div>
                  </Command.Item>
                </Command.GroupItems>
              </Command.Group>

              <Command.Empty class="px-3.5 py-8 text-sm text-text-muted text-center">
                No commands found
              </Command.Empty>

            {:else}
              <!-- File mode -->
              <Command.Group>
                <Command.GroupHeading class="px-3.5 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest text-text-muted">
                  {loadingFiles ? "Loading files…" : "Files"}
                </Command.GroupHeading>
                <Command.GroupItems>
                  {#each fileList as file (file.path)}
                    <Command.Item
                      value={file.path}
                      keywords={[file.name]}
                      onSelect={() => { openFile(file.path); close(); }}
                      class="flex items-center gap-2.5 px-3.5 py-[7px] cursor-pointer data-[highlighted]:bg-accent-subtle"
                    >
                      <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 bg-surface-raised border border-border-default/60 text-text-muted">
                        <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                          <path d="M2 10V2a1 1 0 011-1h4l3 3v6a1 1 0 01-1 1H3a1 1 0 01-1-1z" stroke="currentColor" stroke-width="1.2" />
                          <path d="M7 1v3h3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
                        </svg>
                      </div>
                      <span class="flex-1 text-sm text-text-secondary truncate">{file.name}</span>
                      <span class="text-[11px] text-text-muted font-mono truncate max-w-[200px]">{shortPath(file.path)}</span>
                    </Command.Item>
                  {/each}
                </Command.GroupItems>
              </Command.Group>

              <Command.Empty class="px-3.5 py-8 text-sm text-text-muted text-center">
                {loadingFiles ? "Loading…" : "No files found"}
              </Command.Empty>
            {/if}
          </Command.List>

          <!-- Footer -->
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
