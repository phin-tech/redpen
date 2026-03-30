<script lang="ts">
  import { getDiffState } from "$lib/stores/diff.svelte";
  import { isReviewPageOpen } from "$lib/stores/reviewPage.svelte";

  type EditorRef = {
    moveCursorLine?: (dir: 1 | -1) => void;
    jumpToBoundary?: (boundary: "top" | "bottom") => void;
    toggleVisualSelection?: (mode: "char" | "line") => void;
    clearVisualSelection?: () => void;
    hasVisualSelection?: () => boolean;
  };

  type DiffRef = {
    moveCursorByLine?: (dir: 1 | -1) => void;
    jumpCursorToBoundary?: (boundary: "top" | "bottom") => void;
    toggleVisualMode?: (mode: "char" | "line") => void;
    clearVisualMode?: () => void;
    hasVisualMode?: () => boolean;
  };

  interface ShortcutHelpSection {
    title: string;
    items: { keys: string[]; label: string }[];
  }

  let {
    editorRef,
    rightDiffEditor,
    showShortcutHelp = $bindable(false),
    unifiedDiffEditor,
  }: {
    editorRef?: EditorRef | undefined;
    rightDiffEditor?: DiffRef | undefined;
    showShortcutHelp?: boolean;
    unifiedDiffEditor?: DiffRef | undefined;
  } = $props();

  const diff = getDiffState();

  const codeShortcutHelpSections: ShortcutHelpSection[] = [
    {
      title: "Global",
      items: [
        { keys: ["Mod", "K"], label: "open command palette" },
        { keys: ["Mod", "P"], label: "go to file" },
        { keys: ["[", "]"], label: "switch code / review view" },
      ],
    },
    {
      title: "Editing",
      items: [
        { keys: ["j", "k"], label: "move cursor up / down one line" },
        { keys: ["gg", "G"], label: "jump to top / bottom" },
        { keys: ["v", "V"], label: "toggle visual / line selection" },
        { keys: ["Esc"], label: "clear visual selection" },
        { keys: ["Mod", "Enter"], label: "add annotation" },
        { keys: ["Mod", "F"], label: "find in file" },
        { keys: ["Mod", "G"], label: "next search match" },
        { keys: ["Mod", "Shift", "G"], label: "previous search match" },
      ],
    },
    {
      title: "UI",
      items: [
        { keys: ["Mod", "Shift", "R"], label: "open or close review" },
        { keys: ["Mod", "Shift", "M"], label: "toggle markdown preview" },
        { keys: ["Mod", ","], label: "open settings" },
      ],
    },
  ];

  let pendingCodeG = $state(false);

  function moveActiveCodeLine(dir: 1 | -1) {
    if (diff.enabled && diff.mode === "split") {
      rightDiffEditor?.moveCursorByLine?.(dir);
      return;
    }

    if (diff.enabled && diff.mode === "unified") {
      unifiedDiffEditor?.moveCursorByLine?.(dir);
      return;
    }

    editorRef?.moveCursorLine?.(dir);
  }

  function jumpActiveCodeBoundary(boundary: "top" | "bottom") {
    if (diff.enabled && diff.mode === "split") {
      rightDiffEditor?.jumpCursorToBoundary?.(boundary);
      return;
    }

    if (diff.enabled && diff.mode === "unified") {
      unifiedDiffEditor?.jumpCursorToBoundary?.(boundary);
      return;
    }

    editorRef?.jumpToBoundary?.(boundary);
  }

  function toggleActiveCodeVisual(mode: "char" | "line") {
    if (diff.enabled && diff.mode === "split") {
      rightDiffEditor?.toggleVisualMode?.(mode);
      return;
    }

    if (diff.enabled && diff.mode === "unified") {
      unifiedDiffEditor?.toggleVisualMode?.(mode);
      return;
    }

    editorRef?.toggleVisualSelection?.(mode);
  }

  function clearActiveCodeVisual(): boolean {
    if (diff.enabled && diff.mode === "split") {
      const hadVisual = rightDiffEditor?.hasVisualMode?.() ?? false;
      if (hadVisual) rightDiffEditor?.clearVisualMode?.();
      return hadVisual;
    }

    if (diff.enabled && diff.mode === "unified") {
      const hadVisual = unifiedDiffEditor?.hasVisualMode?.() ?? false;
      if (hadVisual) unifiedDiffEditor?.clearVisualMode?.();
      return hadVisual;
    }

    const hadVisual = editorRef?.hasVisualSelection?.() ?? false;
    if (hadVisual) editorRef?.clearVisualSelection?.();
    return hadVisual;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (isReviewPageOpen()) return;

    const target = event.target;
    const isInputTarget = target instanceof HTMLElement
      && !target.closest(".cm-editor")
      && !!target.closest("input, textarea, select, [contenteditable='true']");

    if (isInputTarget) return;

    if (showShortcutHelp) {
      if (event.key === "Escape" || event.key === "?") {
        event.preventDefault();
        event.stopPropagation();
        showShortcutHelp = false;
      }
      return;
    }

    if (event.metaKey || event.ctrlKey || event.altKey) return;

    if (event.key === "?") {
      event.preventDefault();
      event.stopPropagation();
      showShortcutHelp = true;
      pendingCodeG = false;
      return;
    }

    if (event.key === "g") {
      event.preventDefault();
      event.stopPropagation();
      if (pendingCodeG) {
        jumpActiveCodeBoundary("top");
        pendingCodeG = false;
      } else {
        pendingCodeG = true;
        setTimeout(() => {
          pendingCodeG = false;
        }, 500);
      }
      return;
    }

    pendingCodeG = false;

    if (event.key === "j") {
      event.preventDefault();
      event.stopPropagation();
      moveActiveCodeLine(1);
    } else if (event.key === "k") {
      event.preventDefault();
      event.stopPropagation();
      moveActiveCodeLine(-1);
    } else if (event.key === "G") {
      event.preventDefault();
      event.stopPropagation();
      jumpActiveCodeBoundary("bottom");
    } else if (event.key === "v") {
      event.preventDefault();
      event.stopPropagation();
      toggleActiveCodeVisual("char");
    } else if (event.key === "V") {
      event.preventDefault();
      event.stopPropagation();
      toggleActiveCodeVisual("line");
    } else if (event.key === "Escape" && clearActiveCodeVisual()) {
      event.preventDefault();
      event.stopPropagation();
    }
  }
</script>

<svelte:window onkeydowncapture={handleWindowKeydown} />

{#if showShortcutHelp && !isReviewPageOpen()}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="shortcut-help-overlay" onclick={() => (showShortcutHelp = false)}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="shortcut-help-modal"
      role="dialog"
      aria-modal="true"
      aria-label="Code keyboard shortcuts"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
    >
      <div class="shortcut-help-header">
        <div>
          <h2>Code shortcuts</h2>
          <p>Global and editor shortcuts for the code view.</p>
        </div>
        <button
          class="shortcut-help-close"
          onclick={() => (showShortcutHelp = false)}
          aria-label="Close keyboard shortcut help"
        >
          ×
        </button>
      </div>

      <div class="shortcut-help-grid">
        {#each codeShortcutHelpSections as section}
          <section class="shortcut-help-section">
            <h3>{section.title}</h3>
            <div class="shortcut-help-list">
              {#each section.items as item}
                <div class="shortcut-help-row">
                  <div class="shortcut-help-keys">
                    {#each item.keys as key}
                      <kbd>{key}</kbd>
                    {/each}
                  </div>
                  <span>{item.label}</span>
                </div>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .shortcut-help-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(6, 7, 10, 0.76);
    backdrop-filter: blur(2px);
    z-index: 30;
    padding: 24px;
  }
  .shortcut-help-modal {
    width: min(760px, 100%);
    max-height: min(680px, calc(100vh - 48px));
    overflow: auto;
    background: var(--surface-panel);
    border: 1px solid var(--border-emphasis);
    border-radius: 10px;
    box-shadow: var(--shadow-popover);
    padding: 20px;
  }
  .shortcut-help-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 18px;
  }
  .shortcut-help-header h2 {
    margin: 0;
    font-size: 18px;
    color: var(--text-primary);
  }
  .shortcut-help-header p {
    margin: 6px 0 0;
    color: var(--text-muted);
    font-size: 13px;
  }
  .shortcut-help-close {
    border: 1px solid var(--border-default);
    background: var(--surface-raised);
    color: var(--text-muted);
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
  }
  .shortcut-help-close:hover {
    color: var(--text-secondary);
    border-color: var(--border-emphasis);
  }
  .shortcut-help-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 16px;
  }
  .shortcut-help-section {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    padding: 14px;
  }
  .shortcut-help-section h3 {
    margin: 0 0 10px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .shortcut-help-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .shortcut-help-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .shortcut-help-keys {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }
  .shortcut-help-keys kbd {
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }
</style>
