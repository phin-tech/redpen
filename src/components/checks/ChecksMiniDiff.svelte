<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let {
    file,
    line,
    headSha,
    repoPath,
    onClose,
  }: {
    file: string;
    line: number;
    headSha: string;
    repoPath: string;
    onClose: () => void;
  } = $props();

  let content = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let scrollContainer: HTMLDivElement | undefined = $state();

  async function loadContent() {
    loading = true;
    error = null;
    try {
      content = await invoke<string>("get_file_content_at_ref", {
        directory: repoPath,
        filePath: file,
        gitRef: headSha,
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (file && headSha) {
      loadContent();
    }
  });

  // Scroll to target line after content loads
  $effect(() => {
    if (content && scrollContainer) {
      requestAnimationFrame(() => {
        const targetEl = scrollContainer?.querySelector(`[data-line="${line}"]`);
        if (targetEl) {
          targetEl.scrollIntoView({ block: "center" });
        }
      });
    }
  });

  const lines = $derived(content?.split("\n") ?? []);

  // Show ~15 lines of context around the target
  const contextRadius = 20;
  const visibleRange = $derived(() => {
    const start = Math.max(0, line - 1 - contextRadius);
    const end = Math.min(lines.length, line - 1 + contextRadius + 1);
    return { start, end };
  });
  const visibleLines = $derived(() => {
    const { start, end } = visibleRange();
    return lines.slice(start, end).map((text, i) => ({
      number: start + i + 1,
      text,
      isTarget: start + i + 1 === line,
    }));
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains("mini-diff-backdrop")) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="mini-diff-backdrop" onclick={handleBackdropClick}>
  <div class="mini-diff-modal" role="dialog">
    <div class="mini-diff-header">
      <span class="file-path">{file}</span>
      <span class="file-line">:{line}</span>
      <span class="header-spacer"></span>
      <button class="close-btn" onclick={onClose}>&times;</button>
    </div>
    <div class="mini-diff-body" bind:this={scrollContainer}>
      {#if loading}
        <div class="mini-diff-status">Loading...</div>
      {:else if error}
        <div class="mini-diff-status error">{error}</div>
      {:else}
        <div class="code-lines">
          {#each visibleLines() as ln (ln.number)}
            <div
              class="code-line"
              class:target={ln.isTarget}
              data-line={ln.number}
            >
              <span class="line-number">{ln.number}</span>
              <span class="line-text">{ln.text}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .mini-diff-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .mini-diff-modal {
    width: 70%;
    max-width: 900px;
    height: 60%;
    max-height: 500px;
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .mini-diff-header {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 12px;
    background: var(--surface-panel);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .file-path {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    font-weight: 600;
  }
  .file-line {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--color-danger);
  }
  .header-spacer { flex: 1; }
  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }
  .close-btn:hover { color: var(--text-primary); }
  .mini-diff-body {
    flex: 1;
    overflow: auto;
    background: #0d1117;
  }
  .mini-diff-status {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 12px;
  }
  .mini-diff-status.error {
    color: var(--color-danger);
  }
  .code-lines {
    padding: 4px 0;
  }
  .code-line {
    display: flex;
    padding: 0 12px;
    line-height: 1.6;
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .code-line.target {
    background: color-mix(in srgb, var(--color-danger) 15%, transparent);
  }
  .line-number {
    width: 50px;
    text-align: right;
    padding-right: 12px;
    color: #555;
    flex-shrink: 0;
    user-select: none;
  }
  .line-text {
    color: #ccc;
    white-space: pre;
    overflow-x: auto;
  }
</style>
