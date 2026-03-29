<script lang="ts">
    import { getDiffState, setDiffMode, exitDiff } from "$lib/stores/diff.svelte";
    import type { DiffMode } from "$lib/types";

    interface Props {
        onEnterDiff?: (mode: DiffMode) => void;
    }

    let { onEnterDiff }: Props = $props();

    const diff = getDiffState();

    const modes: { value: DiffMode; label: string }[] = [
        { value: "split", label: "Split" },
        { value: "unified", label: "Unified" },
        { value: "highlights", label: "Highlights" },
    ];

    function handleClick(mode: DiffMode) {
        if (diff.enabled && diff.mode === mode) {
            // Clicking active mode exits diff
            exitDiff();
        } else if (diff.enabled) {
            setDiffMode(mode);
        } else {
            // Not in diff mode — enter it
            onEnterDiff?.(mode);
        }
    }
</script>

<div class="diff-mode-toggle">
    {#each modes as mode}
        <button
            class="toggle-btn"
            class:active={diff.enabled && diff.mode === mode.value}
            onclick={() => handleClick(mode.value)}
        >
            {mode.label}
        </button>
    {/each}
</div>

<style>
    .diff-mode-toggle {
        display: flex;
        background: var(--surface-raised);
        border-radius: 4px;
        border: 1px solid var(--border-default);
        overflow: hidden;
    }
    .toggle-btn {
        min-height: 30px;
        padding: 4px 12px;
        font-size: 12px;
        color: var(--text-muted);
        background: transparent;
        border: none;
        cursor: pointer;
        transition: background 0.15s, color 0.15s;
    }
    .toggle-btn:hover {
        color: var(--text-secondary);
        background: color-mix(in srgb, var(--surface-highlight) 55%, transparent);
    }
    .toggle-btn.active {
        background: var(--view-active-subtle);
        color: var(--view-active);
        box-shadow: inset 0 0 0 1px var(--view-active-border);
    }
</style>
