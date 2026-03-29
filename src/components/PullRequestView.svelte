<script lang="ts">
  import type { GitHubPrSession } from "$lib/types";
  import { renderMarkdown } from "$lib/markdown/render";
  import "$lib/markdown/markdown.css";

  let { session }: { session: GitHubPrSession } = $props();

  const bodyHtml = $derived.by(() => {
    const body = session.body?.trim();
    return renderMarkdown(body && body.length > 0 ? body : "_No description provided._");
  });
</script>

<div class="pr-view">
  <div class="pr-view-card">
    <div class="pr-view-card-header">
      <div class="pr-view-card-title">Pull request</div>
      <div class="pr-view-card-meta">{session.repo} #{session.number}</div>
    </div>
    <div class="pr-view-card-body markdown-body">
      {@html bodyHtml}
    </div>
  </div>
</div>

<style>
  .pr-view {
    height: 100%;
    overflow: auto;
    background: var(--surface-base);
    padding: 18px;
  }
  .pr-view-card {
    max-width: 980px;
    margin: 0 auto;
    border: 1px solid var(--border-default);
    border-radius: 10px;
    background: var(--surface-panel);
    box-shadow: var(--shadow-xs);
    overflow: hidden;
  }
  .pr-view-card-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-default);
    background: var(--surface-raised);
  }
  .pr-view-card-title {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
  }
  .pr-view-card-meta {
    color: var(--text-muted);
    font-size: 12px;
  }
  .pr-view-card-body {
    padding: 16px;
  }
</style>
