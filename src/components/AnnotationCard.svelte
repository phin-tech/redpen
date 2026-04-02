<script lang="ts">
  import type { Annotation, AnnotationKind } from "$lib/types";
  import { Bot } from "lucide-svelte";

  const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

  let {
    annotation,
    isReply = false,
    isSelected = false,
    onClick,
    onDoubleClick,
    onDelete,
    onChoiceToggle,
  }: {
    annotation: Annotation;
    isReply?: boolean;
    isSelected?: boolean;
    onClick: () => void;
    onDoubleClick: () => void;
    onDelete: () => void;
    onChoiceToggle?: (choiceIndex: number) => void;
  } = $props();

  const kindColorMap: Record<AnnotationKind, string> = {
    comment: "var(--kind-comment-border)",
    explanation: "var(--kind-explanation-border)",
    lineNote: "var(--kind-linenote-border)",
    label: "var(--kind-label-border)",
    question: "var(--kind-question-border)",
  };

  const kindBorderColor = $derived(
    annotation.isOrphaned ? undefined : kindColorMap[annotation.kind]
  );

  function syncBadge(syncState: string | null | undefined): { label: string; className: string } | null {
    switch (syncState) {
      case "pendingPublish":
        return { label: "pending", className: "annotation-sync-pending" };
      case "published":
      case "imported":
        return { label: "submitted", className: "annotation-sync-submitted" };
      case "localOnly":
        return { label: "local only", className: "annotation-sync-local" };
      case "conflict":
        return { label: "conflict", className: "annotation-sync-conflict" };
      default:
        return null;
    }
  }

  const githubSyncBadge = $derived(syncBadge(annotation.github?.syncState));

  function formatTimestamp(dateStr: string | null | undefined): string {
    if (!dateStr) return "";
    const date = new Date(dateStr);
    if (Number.isNaN(date.getTime())) return "";
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    }).format(date);
  }
</script>

<div
  class="group px-3 py-2.5 my-1 cursor-pointer transition-all duration-150 border-l-[3px] rounded-md
    {isReply ? 'ml-4' : ''}
    {isSelected ? 'bg-surface-selection annotation-card-selected' : 'hover:bg-surface-highlight/70 annotation-card'}
    {annotation.isOrphaned ? 'border-l-danger opacity-60' : ''}"
  style:border-left-color={kindBorderColor}
  onclick={onClick}
  ondblclick={onDoubleClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && onClick()}
>
  <div class="flex items-center gap-2 mb-1.5">
    {#if isReply}
      <span class="text-xs text-text-muted">↳ reply</span>
    {:else}
      <span class="text-xs text-text-secondary font-mono">
        L{annotation.anchor.range.startLine}
      </span>
    {/if}
    {#if annotation.kind !== "comment"}
      <span class="kind-badge" style:color={kindColorMap[annotation.kind]}>
        {annotation.kind === "explanation" ? "explanation" : annotation.kind === "lineNote" ? "note" : annotation.kind === "question" ? "question" : "label"}
      </span>
    {/if}
    {#if annotation.kind === "question" && annotation.blocking}
      <span class="kind-badge" style:color="var(--kind-question-border)">must-answer</span>
    {/if}
    {#if annotation.isOrphaned}
      <span class="text-xs text-danger font-semibold uppercase tracking-wide">orphaned</span>
    {/if}
    {#if githubSyncBadge}
      <span class={`annotation-sync-badge ${githubSyncBadge.className}`}>{githubSyncBadge.label}</span>
    {/if}
    <button
      class="ml-auto text-base text-text-secondary opacity-0 group-hover:opacity-100 transition-opacity hover:text-danger leading-none"
      onclick={(e) => { e.stopPropagation(); onDelete(); }}
      title="Delete"
    >
      ×
    </button>
  </div>

  <div class="text-base leading-relaxed mb-1.5 whitespace-pre-wrap break-words text-text-primary">
    {annotation.body}
  </div>

  {#if annotation.choices && annotation.choices.length > 0}
    <div class="flex flex-col gap-1 mb-2">
      {#each annotation.choices as choice, i}
        <label
          class="choice-option flex items-center gap-2 px-2.5 py-1.5 rounded-md cursor-pointer text-sm border transition-all"
          class:choice-selected={choice.selected}
          onclick={(e) => e.stopPropagation()}
        >
          <input
            type={annotation.selectionMode === "multi" ? "checkbox" : "radio"}
            name="card-choice-{annotation.id}"
            checked={choice.selected}
            onchange={() => onChoiceToggle?.(i)}
            class="accent-accent"
          />
          <span>{choice.label}</span>
        </label>
      {/each}
    </div>
  {/if}

  {#if annotation.labels.length > 0}
    <div class="flex flex-wrap gap-1 mb-1.5">
      {#each annotation.labels as label}
        <span class="text-xs px-2 py-0.5 rounded-full bg-surface-panel/80 text-text-primary border border-border-default/50" style="box-shadow: var(--shadow-xs)">{label}</span>
      {/each}
    </div>
  {/if}

  <div class="text-xs text-text-secondary flex items-center gap-1 flex-wrap">
    {#if AGENT_AUTHORS.has(annotation.author.toLowerCase())}
      <Bot size={12} class="text-accent-blue" />
    {/if}
    {annotation.author}
    {#if formatTimestamp(annotation.createdAt)}
      <span class="annotation-card-meta-sep">·</span>
      <span>{formatTimestamp(annotation.createdAt)}</span>
    {/if}
  </div>
</div>

<style>
  .annotation-card {
    box-shadow: var(--shadow-card);
    background: var(--surface-panel);
  }
  .annotation-card:hover {
    box-shadow: var(--shadow-card-hover);
    transform: translateY(-0.5px);
  }
  .annotation-card-selected {
    box-shadow: var(--shadow-card-hover), 0 0 0 1px rgba(227, 154, 45, 0.15);
    background: var(--surface-selection);
  }
  .kind-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .annotation-sync-badge {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 999px;
    border: 1px solid var(--border-default);
  }
  .annotation-sync-pending {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--accent) 30%, var(--border-default));
  }
  .annotation-sync-submitted {
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-success) 30%, var(--border-default));
  }
  .annotation-sync-local {
    color: var(--text-secondary);
    background: var(--surface-raised);
  }
  .annotation-sync-conflict {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 30%, var(--border-default));
  }
  .annotation-card-meta-sep {
    color: var(--text-muted);
  }
  .choice-option {
    background: var(--surface-panel);
    border-color: var(--border-default);
    color: var(--text-secondary);
  }
  .choice-option:hover {
    background: var(--surface-highlight);
    border-color: var(--border-emphasis);
  }
  .choice-selected {
    background: var(--accent-subtle);
    border-color: var(--accent);
    color: var(--text-primary);
  }
</style>
