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
        background: var(--surface-secondary, #0d1117);
        border-radius: 6px;
        border: 1px solid var(--border-default, #30363d);
        overflow: hidden;
    }
    .toggle-btn {
        padding: 3px 10px;
        font-size: 11px;
        color: var(--text-muted);
        background: transparent;
        border: none;
        cursor: pointer;
        transition: background 0.15s, color 0.15s;
    }
    .toggle-btn:hover { color: var(--text-secondary); }
    .toggle-btn.active {
        background: var(--accent);
        color: var(--surface-primary, #0d1117);
    }
</style>
