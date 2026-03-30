import { invoke } from "@tauri-apps/api/core";
import type { DiffResult, DiffMode, RefList } from "$lib/types";

interface DiffState {
    enabled: boolean;
    mode: DiffMode;
    baseRef: string;
    targetRef: string;
    baseLabel: string;
    targetLabel: string;
    diffResult: DiffResult | null;
    algorithm: "patience" | "myers";
    loading: boolean;
    error: string | null;
}

let state = $state<DiffState>({
    enabled: false,
    mode: "highlights",
    baseRef: "HEAD",
    targetRef: "working-tree",
    baseLabel: "HEAD",
    targetLabel: "working-tree",
    diffResult: null,
    algorithm: "patience",
    loading: false,
    error: null,
});

let refs = $state<RefList | null>(null);

// --- Diff cache ---
const diffCache = new Map<string, DiffResult>();
const requestCounters = new Map<string, number>();

function cacheKey(
    directory: string,
    filePath: string,
    baseRef: string,
    targetRef: string,
    algorithm: string,
): string {
    return `${directory}::${filePath}::${baseRef}::${targetRef}::${algorithm}`;
}

export async function cachedInvokeDiff(
    directory: string,
    filePath: string,
    baseRef: string,
    targetRef: string,
    algorithm: "patience" | "myers",
): Promise<DiffResult> {
    const key = cacheKey(directory, filePath, baseRef, targetRef, algorithm);

    const cached = diffCache.get(key);
    if (cached) return cached;

    const counter = (requestCounters.get(key) ?? 0) + 1;
    requestCounters.set(key, counter);

    const result = await invoke<DiffResult>("compute_diff", {
        directory,
        filePath,
        baseRef,
        targetRef,
        algorithm,
    });

    if (requestCounters.get(key) === counter) {
        diffCache.set(key, result);
    }

    return result;
}

export function invalidateFile(directory: string, filePath: string) {
    const prefix = `${directory}::${filePath}::`;
    for (const key of diffCache.keys()) {
        if (key.startsWith(prefix)) {
            diffCache.delete(key);
            requestCounters.delete(key);
        }
    }
}

export function clearDiffCache() {
    diffCache.clear();
    requestCounters.clear();
}

export function getDiffState() {
    return state;
}

export function getRefs(): RefList | null {
    return refs;
}

export async function enterDiff(directory: string, filePath: string) {
    state.enabled = true;
    state.error = null;
    await loadRefs(directory);
    await computeDiff(directory, filePath);
}

export function exitDiff() {
    state.enabled = false;
    state.diffResult = null;
    state.error = null;
    state.loading = false;
    clearDiffCache();
}

export function setDiffMode(mode: DiffMode) {
    state.mode = mode;
}

export async function setDiffRefs(
    directory: string,
    filePath: string,
    base: string,
    target: string,
    baseLabel?: string,
    targetLabel?: string,
) {
    state.baseRef = base;
    state.targetRef = target;
    state.baseLabel = baseLabel ?? base;
    state.targetLabel = targetLabel ?? target;
    await computeDiff(directory, filePath);
}

export async function swapRefs(directory: string, filePath: string) {
    const temp = state.baseRef;
    const tempLabel = state.baseLabel;
    state.baseRef = state.targetRef;
    state.targetRef = temp;
    state.baseLabel = state.targetLabel;
    state.targetLabel = tempLabel;
    await computeDiff(directory, filePath);
}

export function setDiffDefaults(
    base: string,
    target: string,
    baseLabel?: string,
    targetLabel?: string,
) {
    state.baseRef = base;
    state.targetRef = target;
    state.baseLabel = baseLabel ?? base;
    state.targetLabel = targetLabel ?? target;
}

export function resetDiffDefaults() {
    setDiffDefaults("HEAD", "working-tree");
}

export async function computeDiff(directory: string, filePath: string) {
    state.loading = true;
    state.error = null;
    try {
        const result = await cachedInvokeDiff(
            directory,
            filePath,
            state.baseRef,
            state.targetRef,
            state.algorithm,
        );
        state.diffResult = result;
    } catch (e) {
        state.error = e instanceof Error ? e.message : String(e);
        state.diffResult = null;
    } finally {
        state.loading = false;
    }
}

export async function loadRefs(directory: string) {
    try {
        refs = await invoke<RefList>("list_refs", { directory });
    } catch {
        refs = null;
    }
}

export function setAlgorithm(algo: "patience" | "myers") {
    state.algorithm = algo;
}

export function resetDiffForTests() {
    state.enabled = false;
    state.mode = "highlights";
    state.baseRef = "HEAD";
    state.targetRef = "working-tree";
    state.baseLabel = "HEAD";
    state.targetLabel = "working-tree";
    state.diffResult = null;
    state.algorithm = "patience";
    state.loading = false;
    state.error = null;
    refs = null;
    clearDiffCache();
}
