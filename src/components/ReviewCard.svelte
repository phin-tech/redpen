<script lang="ts">
  import type { Annotation, AnnotationKind, DiffHunk } from "$lib/types";
  import type { FileSnippet } from "$lib/tauri";
  import ReviewCodeSnippet from "./ReviewCodeSnippet.svelte";
  import { Bot } from "lucide-svelte";

  const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

  const KIND_COLORS: Record<AnnotationKind, string> = {
    comment: "var(--kind-comment-border)",
    lineNote: "var(--kind-linenote-border)",
    explanation: "var(--kind-explanation-border)",
    label: "var(--kind-label-border)",
  };

  const KIND_LABELS: Record<AnnotationKind, string> = {
    comment: "comment",
    lineNote: "note",
    explanation: "explanation",
    label: "label",
  };

  let {
    annotation,
    replies = [],
    filePath,
    snippet,
    diffHunk,
    isActive = false,
    onReply,
    onResolve,
    onJumpToFile,
    onChoiceToggle,
  }: {
    annotation: Annotation;
    replies?: Annotation[];
    filePath: string;
    snippet: FileSnippet | null;
    diffHunk?: DiffHunk | null;
    isActive?: boolean;
    onReply: (annotationId: string, body: string) => void;
    onResolve: (annotationId: string, resolved: boolean) => void;
    onJumpToFile: (filePath: string, line: number) => void;
    onChoiceToggle: (annotationId: string, choiceIndex: number) => void;
  } = $props();

  let replyText = $state("");
  let replyInputRef: HTMLInputElement | undefined = $state(undefined);

  const kindColor = $derived(KIND_COLORS[annotation.kind]);
  const isAgent = $derived(AGENT_AUTHORS.has(annotation.author.toLowerCase()));

  function submitReply() {
    if (!replyText.trim()) return;
    onReply(annotation.id, replyText.trim());
    replyText = "";
  }

  function handleReplyKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      replyInputRef?.blur();
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submitReply();
    }
    e.stopPropagation();
  }

  export function focusReply() {
    replyInputRef?.focus();
  }


  function relativeTime(dateStr: string | null | undefined): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }
</script>

<div
  class="review-card"
  class:review-card-active={isActive}
  class:review-card-resolved={annotation.resolved}
>
  <div class="review-card-kind-strip" style:background={kindColor}></div>

  <ReviewCodeSnippet
    {filePath}
    {snippet}
    highlightLine={annotation.anchor.range.startLine}
    highlightEndLine={annotation.anchor.range.endLine}
    {diffHunk}
    kindColor={kindColor}
  />

  <div class="review-card-body">
    <div class="review-card-header">
      <span class="review-card-author">
        {#if isAgent}<Bot size={14} class="review-card-agent-icon" />{/if}
        {annotation.author}
      </span>
      <span class="review-card-badge" style:background="color-mix(in srgb, {kindColor} 15%, transparent)" style:color={kindColor}>
        {KIND_LABELS[annotation.kind]}
      </span>
      <span class="review-card-time">{relativeTime(annotation.createdAt)}</span>
    </div>

    <div class="review-card-text">{annotation.body}</div>

    {#if annotation.choices && annotation.choices.length > 0}
      <div class="review-card-choices">
        {#each annotation.choices as choice, i}
          <label
            class="review-card-choice"
            class:review-card-choice-selected={choice.selected}
          >
            <input
              type={annotation.selectionMode === "multi" ? "checkbox" : "radio"}
              name="review-choice-{annotation.id}"
              checked={choice.selected}
              onchange={() => onChoiceToggle(annotation.id, i)}
              hidden
            />
            <kbd class="review-card-choice-key">{i + 1}</kbd>
            <span>{choice.label}</span>
          </label>
        {/each}
      </div>
    {/if}

    {#if annotation.labels.length > 0}
      <div class="review-card-labels">
        {#each annotation.labels as label}
          <span class="review-card-label">{label}</span>
        {/each}
      </div>
    {/if}
  </div>

  {#each replies as reply}
    <div class="review-card-reply">
      <span class="review-card-reply-indicator">↳</span>
      <span class="review-card-author" style="font-size: 12px">
        {#if AGENT_AUTHORS.has(reply.author.toLowerCase())}<Bot size={12} class="review-card-agent-icon" />{/if}
        {reply.author}
      </span>
      <span class="review-card-reply-text">{reply.body}</span>
    </div>
  {/each}

  <div class="review-card-actions">
    <input
      bind:this={replyInputRef}
      class="review-card-reply-input"
      placeholder="Reply..."
      bind:value={replyText}
      onkeydown={handleReplyKeydown}
    />
    <button
      class="review-card-action-btn"
      class:review-card-action-resolved={annotation.resolved}
      onclick={() => onResolve(annotation.id, !annotation.resolved)}
    >
      {annotation.resolved ? "✓ Resolved" : "✓ Resolve"}
    </button>
    <button
      class="review-card-action-btn review-card-action-jump"
      onclick={() => onJumpToFile(filePath, annotation.anchor.range.startLine)}
      title="Open in editor"
    >
      Open file →
    </button>
  </div>
</div>

<style>
  .review-card {
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    margin: 8px 0;
    overflow: hidden;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .review-card:hover {
    border-color: var(--border-emphasis);
    box-shadow: var(--shadow-xs);
  }
  .review-card-active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .review-card-resolved {
    opacity: 0.5;
  }
  .review-card-kind-strip {
    height: 3px;
  }
  .review-card-body {
    padding: 12px 16px;
  }
  .review-card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .review-card-author {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  :global(.review-card-agent-icon) {
    color: var(--accent);
  }
  .review-card-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .review-card-time {
    font-size: 11px;
    color: var(--text-muted);
    margin-left: auto;
  }
  .review-card-text {
    color: var(--text-primary);
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .review-card-choices {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .review-card-choice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .review-card-choice:hover {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }
  .review-card-choice-key {
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 0 5px;
    font-family: inherit;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .review-card-choice-selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--text-primary);
  }
  .review-card-labels {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }
  .review-card-label {
    font-size: 10px;
    background: var(--surface-raised);
    color: var(--text-secondary);
    padding: 2px 8px;
    border-radius: 4px;
  }
  .review-card-reply {
    padding: 8px 16px 10px;
    border-top: 1px solid var(--border-default);
    background: color-mix(in srgb, var(--surface-panel) 88%, var(--surface-raised));
    font-size: 12px;
    color: var(--text-secondary);
    display: flex;
    align-items: baseline;
    gap: 4px;
  }
  .review-card-reply-indicator {
    color: var(--text-muted);
  }
  .review-card-reply-text {
    flex: 1;
  }
  .review-card-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-top: 1px solid var(--border-default);
    background: color-mix(in srgb, var(--surface-panel) 84%, var(--surface-raised));
  }
  .review-card-reply-input {
    flex: 1;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: 5px;
    padding: 4px 10px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
  }
  .review-card-reply-input::placeholder {
    color: var(--text-muted);
  }
  .review-card-reply-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .review-card-action-btn {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 5px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .review-card-action-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
    background: var(--surface-raised);
  }
  .review-card-action-resolved {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-success) 30%, transparent);
  }
  .review-card-action-jump {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .review-card-action-jump:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
</style>
