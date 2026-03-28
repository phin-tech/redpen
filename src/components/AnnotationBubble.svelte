<script lang="ts">
  import type { Annotation } from "$lib/types";

  let {
    annotations,
    expanded = false,
    onToggle,
    onSelect,
    onDelete,
  }: {
    annotations: Annotation[];
    expanded?: boolean;
    onToggle: () => void;
    onSelect: (id: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  const root = $derived(annotations.find((a) => !a.replyTo) ?? annotations[0]);
  const replies = $derived(annotations.filter((a) => a.replyTo));
  const replyCount = $derived(replies.length);
  const hasOrphaned = $derived(annotations.some((a) => a.isOrphaned));

  function truncate(text: string, max: number) {
    if (text.length <= max) return text;
    return text.slice(0, max).trimEnd() + "…";
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="rp-bubble-container"
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
          <span class="rp-bubble-author">{root.author}</span>
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
            <span class="rp-bubble-author">{reply.author}</span>
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
      <span class="rp-bubble-author">{root.author}</span>
      {#if root.isOrphaned}
        <span class="rp-bubble-orphan-badge">orphaned</span>
      {/if}
      <span class="rp-bubble-body">{truncate(root.body, 80)}</span>
      {#if replyCount > 0}
        <span class="rp-bubble-reply-count">{replyCount} {replyCount === 1 ? "reply" : "replies"}</span>
      {/if}
      {#each root.labels as label}
        <span class="rp-bubble-label">{label}</span>
      {/each}
    </div>
  {/if}
</div>
