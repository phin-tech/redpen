<script lang="ts">
  import { getWorkspace } from "$lib/stores/workspace.svelte";
  import { Button } from "flowbite-svelte";

  let { onSettingsClick }: { onSettingsClick: () => void } = $props();

  const workspace = getWorkspace();

  let workspaceName = $derived(
    workspace.rootFolders.length > 0
      ? workspace.rootFolders[0].split("/").pop() ?? "Red Pen"
      : "Red Pen"
  );
</script>

<div class="flex items-center justify-between h-10 pl-[78px] pr-3 bg-[var(--bg-primary)] border-b border-graphite-700" style="-webkit-app-region: drag">
  <div class="flex items-center gap-2 flex-1" style="-webkit-app-region: no-drag">
    <span class="text-xs font-semibold text-graphite-200 tracking-wide">{workspaceName}</span>
  </div>
  <div class="flex justify-end" style="-webkit-app-region: no-drag">
    <Button size="xs" color="alternative" onclick={onSettingsClick} class="!p-1.5">
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <path d="M8 10a2 2 0 100-4 2 2 0 000 4z" stroke="currentColor" stroke-width="1.5"/>
        <path d="M13.5 8a5.5 5.5 0 01-.3 1.8l1.3 1-1 1.7-1.5-.6a5.5 5.5 0 01-1.5.9L10 14H8l-.5-1.6a5.5 5.5 0 01-1.5-.9l-1.5.6-1-1.7 1.3-1A5.5 5.5 0 014.5 8c0-.6.1-1.2.3-1.8l-1.3-1 1-1.7 1.5.6a5.5 5.5 0 011.5-.9L8 2h2l.5 1.6a5.5 5.5 0 011.5.9l1.5-.6 1 1.7-1.3 1c.2.6.3 1.2.3 1.8z" stroke="currentColor" stroke-width="1.2"/>
      </svg>
    </Button>
  </div>
</div>
