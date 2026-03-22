<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    class?: string;
    children: Snippet;
  }

  let { class: className = '', children }: Props = $props();
</script>

<kbd
  class="inline-flex items-center justify-center px-1.5 py-0.5 text-xs font-mono rounded bg-surface-raised text-text-secondary border border-border-default {className}"
>
  {@render children()}
</kbd>
