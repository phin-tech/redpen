<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'success';
    size?: 'sm' | 'md';
    disabled?: boolean;
    class?: string;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  }

  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    class: className = '',
    children,
    ...restProps
  }: Props = $props();

  const variantStyles: Record<string, string> = {
    primary: 'bg-accent text-surface-base font-semibold hover:bg-accent-hover',
    secondary:
      'bg-surface-raised text-text-secondary border border-border-subtle hover:bg-surface-highlight',
    ghost: 'text-text-secondary hover:bg-surface-highlight hover:text-text-primary',
    danger:
      'bg-[color:color-mix(in_srgb,var(--color-danger)_16%,transparent)] text-[var(--color-danger)] border border-[color:color-mix(in_srgb,var(--color-danger)_35%,transparent)] hover:bg-[color:color-mix(in_srgb,var(--color-danger)_22%,transparent)]',
    success:
      'bg-[color:color-mix(in_srgb,var(--color-success)_16%,transparent)] text-[var(--color-success)] border border-[color:color-mix(in_srgb,var(--color-success)_35%,transparent)] hover:bg-[color:color-mix(in_srgb,var(--color-success)_22%,transparent)]',
  };

  const sizeStyles: Record<string, string> = {
    sm: 'px-2.5 py-1 text-xs rounded-md',
    md: 'px-3 py-1.5 text-sm rounded-md',
  };
</script>

<button
  class="transition-colors inline-flex items-center justify-center gap-1.5 focus-visible:ring-1 focus-visible:ring-accent/30 focus-visible:outline-none {variantStyles[variant]} {sizeStyles[size]} {disabled ? 'opacity-50 pointer-events-none' : ''} {className}"
  {disabled}
  {...restProps}
>
  {@render children()}
</button>
