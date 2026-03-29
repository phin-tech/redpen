<script lang="ts">
    import { getDiffState, getRefs, setDiffRefs, swapRefs, loadRefs } from "$lib/stores/diff.svelte";

    interface Props {
        directory: string;
        filePath: string;
    }

    let { directory, filePath }: Props = $props();

    const diff = getDiffState();
    let showBaseDropdown = $state(false);
    let showTargetDropdown = $state(false);

    $effect(() => {
        if (diff.enabled && directory) {
            loadRefs(directory);
        }
    });

    function selectBase(ref: string) {
        showBaseDropdown = false;
        setDiffRefs(directory, filePath, ref, diff.targetRef);
    }

    function selectTarget(ref: string) {
        showTargetDropdown = false;
        setDiffRefs(directory, filePath, diff.baseRef, ref);
    }

    function handleSwap() {
        swapRefs(directory, filePath);
    }

    function closeDropdowns(e: MouseEvent) {
        const target = e.target as HTMLElement;
        if (!target.closest(".ref-dropdown")) {
            showBaseDropdown = false;
            showTargetDropdown = false;
        }
    }
</script>

<svelte:window onclick={closeDropdowns} />

{#if diff.enabled}
    {@const refList = getRefs()}
    <div class="ref-picker">
        <div class="ref-dropdown">
            <button class="ref-btn base" onclick={(e) => { e.stopPropagation(); showBaseDropdown = !showBaseDropdown; showTargetDropdown = false; }}>
                {diff.baseRef}
            </button>
            {#if showBaseDropdown && refList}
                <div class="dropdown-menu">
                    <button class="dropdown-item" onclick={() => selectBase("HEAD")}>HEAD</button>
                    <button class="dropdown-item" onclick={() => selectBase("working-tree")}>working tree</button>
                    {#each refList.branches as branch}
                        <button class="dropdown-item" onclick={() => selectBase(branch.name)}>
                            {branch.name}{branch.isCurrent ? " *" : ""}
                        </button>
                    {/each}
                    {#each refList.tags as tag}
                        <button class="dropdown-item" onclick={() => selectBase(tag)}>{tag}</button>
                    {/each}
                    {#each refList.recentCommits as commit}
                        <button class="dropdown-item" onclick={() => selectBase(commit.sha)}>
                            <span class="sha">{commit.sha}</span> {commit.shortMessage}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>

        <button class="swap-btn" onclick={handleSwap} title="Swap base and target">←→</button>

        <div class="ref-dropdown">
            <button class="ref-btn target" onclick={(e) => { e.stopPropagation(); showTargetDropdown = !showTargetDropdown; showBaseDropdown = false; }}>
                {diff.targetRef}
            </button>
            {#if showTargetDropdown && refList}
                <div class="dropdown-menu">
                    <button class="dropdown-item" onclick={() => selectTarget("HEAD")}>HEAD</button>
                    <button class="dropdown-item" onclick={() => selectTarget("working-tree")}>working tree</button>
                    {#each refList.branches as branch}
                        <button class="dropdown-item" onclick={() => selectTarget(branch.name)}>
                            {branch.name}{branch.isCurrent ? " *" : ""}
                        </button>
                    {/each}
                    {#each refList.tags as tag}
                        <button class="dropdown-item" onclick={() => selectTarget(tag)}>{tag}</button>
                    {/each}
                    {#each refList.recentCommits as commit}
                        <button class="dropdown-item" onclick={() => selectTarget(commit.sha)}>
                            <span class="sha">{commit.sha}</span> {commit.shortMessage}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    .ref-picker {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 11px;
    }
    .ref-dropdown { position: relative; }
    .ref-btn {
        min-height: 30px;
        padding: 4px 10px;
        background: var(--surface-raised);
        border: 1px solid var(--border-default);
        border-radius: 4px;
        cursor: pointer;
        font-size: 12px;
        font-family: inherit;
        transition: border-color 0.15s, background 0.15s, color 0.15s;
    }
    .ref-btn:hover {
        border-color: var(--border-emphasis);
        background: color-mix(in srgb, var(--surface-highlight) 55%, transparent);
    }
    .ref-btn.base { color: var(--accent-purple); }
    .ref-btn.target { color: var(--color-success); }
    .swap-btn {
        min-height: 30px;
        padding: 4px 8px;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 4px;
        color: var(--text-muted);
        cursor: pointer;
        font-size: 12px;
        transition: border-color 0.15s, background 0.15s, color 0.15s;
    }
    .swap-btn:hover {
        color: var(--text-primary);
        background: var(--surface-raised);
        border-color: var(--border-default);
    }
    .dropdown-menu {
        position: absolute;
        top: 100%;
        left: 0;
        margin-top: 4px;
        background: var(--surface-panel);
        border: 1px solid var(--border-default);
        border-radius: 6px;
        min-width: 200px;
        max-height: 300px;
        overflow-y: auto;
        z-index: 100;
        padding: 4px 0;
    }
    .dropdown-item {
        display: block;
        width: 100%;
        padding: 6px 12px;
        background: none;
        border: none;
        color: var(--text-primary);
        cursor: pointer;
        font-size: 11px;
        text-align: left;
        font-family: inherit;
    }
    .dropdown-item:hover { background: var(--surface-raised); }
    .sha { color: var(--text-muted); font-family: monospace; }
</style>
