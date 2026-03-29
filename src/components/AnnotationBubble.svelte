<script lang="ts">
  import type { Annotation } from "$lib/types";
  import { Bot } from "lucide-svelte";

  const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

  function isAgent(author: string): boolean {
    return AGENT_AUTHORS.has(author.toLowerCase());
  }

  let {
    annotations,
    expanded = false,
    onToggle,
    onSelect,
    onDelete,
    onChoiceToggle,
  }: {
    annotations: Annotation[];
    expanded?: boolean;
    onToggle: () => void;
    onSelect: (id: string) => void;
    onDelete: (id: string) => void;
    onChoiceToggle: (annotationId: string, choiceIndex: number) => void;
  } = $props();

  const root = $derived(annotations.find((a) => !a.replyTo) ?? annotations[0]);
  const replies = $derived(annotations.filter((a) => a.replyTo));
  const replyCount = $derived(replies.length);
  const hasOrphaned = $derived(annotations.some((a) => a.isOrphaned));

  const kindCssClass = $derived(
    root.kind === "explanation" ? "rp-bubble-kind-explanation"
    : root.kind === "lineNote" ? "rp-bubble-kind-linenote"
    : root.kind === "label" ? "rp-bubble-kind-label"
    : ""
  );

  const kindLabel = $derived(
    root.kind === "explanation" ? "explanation"
    : root.kind === "lineNote" ? "note"
    : root.kind === "label" ? "label"
    : null
  );

  function truncate(text: string, max: number) {
    if (text.length <= max) return text;
    return text.slice(0, max).trimEnd() + "…";
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="rp-bubble-container {kindCssClass}"
  class:rp-bubble-collapsed={!expanded}
  class:rp-bubble-orphaned={hasOrphaned}
  onclick={(e) => { e.stopPropagation(); onToggle(); }}
>
  <div class="rp-bubble-notch"></div>

  {#if expanded}
    <!-- Expanded: full thread -->
    <div class="rp-bubble-thread">
      <div
        class="rp-bubble-item"
        onclick={(e) => { e.stopPropagation(); onSelect(root.id); }}
      >
        <div class="rp-bubble-header">
          <span class="rp-bubble-author">
            {#if isAgent(root.author)}<Bot size={14} class="rp-bubble-agent-icon" />{/if}
            {root.author}
          </span>
          {#if kindLabel}
            <span class="rp-bubble-orphan-badge" style:color="inherit">{kindLabel}</span>
          {/if}
          {#if root.isOrphaned}
            <span class="rp-bubble-orphan-badge">orphaned</span>
          {/if}
          <button
            class="rp-bubble-delete"
            onclick={(e) => { e.stopPropagation(); onDelete(root.id); }}
            title="Delete"
          >×</button>
        </div>
        <div class="rp-bubble-body">{root.body}</div>
        {#if root.choices && root.choices.length > 0}
          <div class="rp-bubble-choices">
            {#each root.choices as choice, i}
              <label
                class="rp-bubble-choice"
                class:rp-bubble-choice-selected={choice.selected}
                onclick={(e) => e.stopPropagation()}
              >
                <input
                  type={root.selectionMode === "multi" ? "checkbox" : "radio"}
                  name="choice-{root.id}"
                  checked={choice.selected}
                  onchange={() => onChoiceToggle(root.id, i)}
                />
                <span>{choice.label}</span>
              </label>
            {/each}
          </div>
        {/if}
        {#if root.labels.length > 0}
          <div class="rp-bubble-labels">
            {#each root.labels as label}
              <span class="rp-bubble-label">{label}</span>
            {/each}
          </div>
        {/if}
      </div>

      {#each replies as reply}
        <div
          class="rp-bubble-item rp-bubble-reply"
          onclick={(e) => { e.stopPropagation(); onSelect(reply.id); }}
        >
          <div class="rp-bubble-header">
            <span class="rp-bubble-reply-indicator">↳</span>
            <span class="rp-bubble-author">
              {#if isAgent(reply.author)}<Bot size={14} class="rp-bubble-agent-icon" />{/if}
              {reply.author}
            </span>
            {#if reply.isOrphaned}
              <span class="rp-bubble-orphan-badge">orphaned</span>
            {/if}
            <button
              class="rp-bubble-delete"
              onclick={(e) => { e.stopPropagation(); onDelete(reply.id); }}
              title="Delete"
            >×</button>
          </div>
          <div class="rp-bubble-body">{reply.body}</div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- Collapsed: single line summary -->
    <div class="rp-bubble-summary" onclick={(e) => { e.stopPropagation(); onSelect(root.id); }}>
      <span class="rp-bubble-author">
        {#if isAgent(root.author)}<Bot size={14} class="rp-bubble-agent-icon" />{/if}
        {root.author}
      </span>
      {#if kindLabel}
        <span class="rp-bubble-orphan-badge" style:color="inherit">{kindLabel}</span>
      {/if}
      {#if root.isOrphaned}
        <span class="rp-bubble-orphan-badge">orphaned</span>
      {/if}
      <span class="rp-bubble-body">{truncate(root.body, 80)}</span>
      {#if root.choices && root.choices.length > 0}
        {@const selected = root.choices.filter((c) => c.selected)}
        <span class="rp-bubble-reply-count">
          {selected.length}/{root.choices.length} picked
        </span>
      {/if}
      {#if replyCount > 0}
        <span class="rp-bubble-reply-count">{replyCount} {replyCount === 1 ? "reply" : "replies"}</span>
      {/if}
      {#each root.labels as label}
        <span class="rp-bubble-label">{label}</span>
      {/each}
    </div>
  {/if}
</div>
