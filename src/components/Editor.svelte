<script lang="ts">
  import { onDestroy, tick, untrack } from "svelte";
  import { EditorView } from "@codemirror/view";
  import { Compartment, StateEffect } from "@codemirror/state";
  import { createEditor } from "$lib/codemirror/setup";
  import { setAnnotationsEffect } from "$lib/codemirror/annotations";
  import { setSearchEffect } from "$lib/codemirror/search";
  import { getEditor, getFileExtension } from "$lib/stores/editor.svelte";
  import { getDiffState } from "$lib/stores/diff.svelte";
  import { highlightsModeExtensions } from "$lib/codemirror/diff";
  import { sortedAnnotations } from "$lib/stores/annotations.svelte";

  // Svelte 5 runes mode: use $bindable ref pattern instead of `export function`
  let {
    onSelectionChange,
    ref = $bindable(undefined),
  }: {
    onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    ref?: { scrollToLine: (line: number) => void; openSearch: () => void; closeSearch: () => void; navigateMatch: (dir: 1 | -1) => void } | undefined;
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;

  // Search state
  let showSearch = $state(false);
  let searchQuery = $state("");
  let matchPositions = $state<{ from: number; to: number }[]>([]);
  let matchIndex = $state(0);
  let searchInput: HTMLInputElement;

  const editor = getEditor();
  const diff = getDiffState();
  const diffCompartment = new Compartment();

  function scrollToLine(line: number) {
    if (!view) return;
    const lineObj = view.state.doc.line(Math.min(line, view.state.doc.lines));
    view.dispatch({
      selection: { anchor: lineObj.from },
      effects: EditorView.scrollIntoView(lineObj.from, { y: "center" }),
    });
  }

  function performSearch(query: string) {
    if (!view || !query) {
      matchPositions = [];
      matchIndex = 0;
      view?.dispatch({ effects: setSearchEffect.of(null) });
      return;
    }

    const text = view.state.doc.toString();
    const lowerText = text.toLowerCase();
    const lowerQuery = query.toLowerCase();
    const matches: { from: number; to: number }[] = [];
    let pos = 0;
    while (pos < text.length) {
      const idx = lowerText.indexOf(lowerQuery, pos);
      if (idx === -1) break;
      matches.push({ from: idx, to: idx + query.length });
      pos = idx + 1;
    }

    matchPositions = matches;
    if (matches.length === 0) {
      matchIndex = 0;
      view.dispatch({ effects: setSearchEffect.of(null) });
    } else {
      matchIndex = 0;
      view.dispatch({ effects: setSearchEffect.of({ matches, currentIdx: matchIndex }) });
      scrollToMatch(matchIndex);
    }
  }

  function scrollToMatch(idx: number) {
    if (!view || !matchPositions[idx]) return;
    view.dispatch({
      effects: EditorView.scrollIntoView(matchPositions[idx].from, { y: "center" }),
    });
  }

  function navigateMatch(dir: 1 | -1) {
    if (!showSearch || matchPositions.length === 0) return;
    matchIndex = (matchIndex + dir + matchPositions.length) % matchPositions.length;
    view?.dispatch({ effects: setSearchEffect.of({ matches: matchPositions, currentIdx: matchIndex }) });
    scrollToMatch(matchIndex);
  }

  function openSearch() {
    if (!editor.currentFilePath) return;
    showSearch = true;
    tick().then(() => {
      searchInput?.focus();
      searchInput?.select();
    });
    if (searchQuery) performSearch(searchQuery);
  }

  function closeSearch() {
    showSearch = false;
    matchPositions = [];
    searchQuery = "";
    view?.dispatch({ effects: setSearchEffect.of(null) });
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closeSearch();
    } else if (e.key === "Enter") {
      e.preventDefault();
      navigateMatch(e.shiftKey ? -1 : 1);
    }
  }

  // Expose functions to parent via ref
  $effect(() => {
    ref = { scrollToLine, openSearch, closeSearch, navigateMatch };
  });

  onDestroy(() => {
    view?.destroy();
  });

  function createView() {
    view?.destroy();
    view = createEditor({
      content: editor.content,
      extension: getFileExtension(),
      parent: container,
      onSelectionChange: onSelectionChange
        ? (from, to, fromLine, fromCol, toLine, toCol) => {
            onSelectionChange!(fromLine, fromCol, toLine, toCol);
          }
        : undefined,
    });
    // Add diff compartment to the editor state (initially empty)
    view.dispatch({
      effects: StateEffect.appendConfig.of(diffCompartment.of([])),
    });
    // Re-apply search after file change without tracking these as effect dependencies
    untrack(() => {
      if (showSearch && searchQuery) {
        performSearch(searchQuery);
      }
    });
  }

  // Recreate view when file changes
  $effect(() => {
    if (container && editor.content !== undefined && editor.currentFilePath) {
      createView();
    }
  });

  // Update annotations when they change
  $effect(() => {
    const annotations = sortedAnnotations();
    if (view) {
      view.dispatch({
        effects: setAnnotationsEffect.of(annotations),
      });
    }
  });

  // Highlights mode: add/remove diff decorations
  $effect(() => {
    if (!view) return;
    const diffResult = diff.enabled && diff.mode === "highlights"
      ? diff.diffResult
      : null;
    // Reconfiguration dispatch is imperative; keep it out of dependency tracking.
    untrack(() => {
      view.dispatch({
        effects: diffCompartment.reconfigure(
          diffResult ? highlightsModeExtensions(diffResult) : []
        ),
      });
    });
  });
</script>

<div class="editor-container">
  {#if editor.loading}
    <div class="loading">Loading...</div>
  {:else if !editor.currentFilePath}
    <div class="empty">Select a file to view</div>
  {/if}

  {#if showSearch}
    <div class="search-bar">
      <input
        bind:this={searchInput}
        bind:value={searchQuery}
        oninput={(e) => performSearch((e.currentTarget as HTMLInputElement).value)}
        onkeydown={handleSearchKeydown}
        placeholder="Find in file…"
        spellcheck="false"
        autocomplete="off"
      />
      <span class="match-count">
        {#if searchQuery}
          {matchPositions.length === 0 ? "No results" : `${matchIndex + 1} / ${matchPositions.length}`}
        {/if}
      </span>
      <button
        type="button"
        class="nav-btn"
        onclick={() => navigateMatch(-1)}
        disabled={matchPositions.length === 0}
        title="Previous match (Shift+Enter)"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="18 15 12 9 6 15"></polyline>
        </svg>
      </button>
      <button
        type="button"
        class="nav-btn"
        onclick={() => navigateMatch(1)}
        disabled={matchPositions.length === 0}
        title="Next match (Enter)"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 9 12 15 18 9"></polyline>
        </svg>
      </button>
      <button type="button" class="close-btn" onclick={() => closeSearch()} title="Close (Escape)">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
  {/if}

  <div class="cm-wrapper" bind:this={container}></div>
</div>

<style>
  .editor-container {
    height: 100%;
    position: relative;
    overflow: hidden;
  }

  .cm-wrapper {
    height: 100%;
  }

  .cm-wrapper :global(.cm-editor) {
    height: 100%;
  }

  .loading, .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .search-bar {
    position: absolute;
    top: 8px;
    right: 16px;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 8px;
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .search-bar input {
    width: 200px;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 0 4px;
  }

  .search-bar input::placeholder {
    color: var(--text-muted);
  }

  .match-count {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
    min-width: 52px;
    text-align: right;
    padding-right: 4px;
    font-family: var(--font-mono);
  }

  .nav-btn, .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }

  .nav-btn:hover, .close-btn:hover {
    background: var(--surface-highlight);
    color: var(--text-primary);
  }

  .nav-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .nav-btn:disabled:hover {
    background: transparent;
    color: var(--text-secondary);
  }

  .close-btn {
    margin-left: 2px;
  }
</style>
