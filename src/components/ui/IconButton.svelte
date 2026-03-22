<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    label: string;
    class?: string;
    active?: boolean;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }

  let {
    label,
    class: className = '',
    active = false,
    children,
    ...restProps
  }: Props = $props();
</script>

<button
  title={label}
  aria-label={label}
  class="inline-flex items-center justify-center p-1 rounded hover:text-text-primary hover:bg-surface-highlight transition-colors {active ? 'text-accent' : 'text-text-secondary'} {className}"
  {...restProps}
>
  {@render children()}
</button>
