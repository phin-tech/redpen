# Diff Cache Design

**Issue**: phin-tech/redpen#40
**Date**: 2026-03-29

## Problem

When switching away from a file and back in diff mode, `enterDiff` recomputes the diff via Rust IPC even if nothing has changed. This is wasted work, especially during PR reviews where refs are fixed SHAs.

## Solution

Cache `DiffResult` in the diff store keyed on `(directory, filePath, baseRef, targetRef, algorithm)`. Prevent stale-write races with per-key request counters. Invalidate on file save, session change, and diff exit.

## Cache Key

```
${directory}::${filePath}::${baseRef}::${targetRef}::${algorithm}
```

`directory` is required because the same file path can exist in different worktrees (GitHub PR reviews use separate worktrees).

## Cache Location

TypeScript layer in `src/lib/stores/diff.svelte.ts`. Avoids IPC round-trip on cache hits. Invalidation is straightforward since all triggers (file watcher, session changes) are already on the frontend.

## Core Cache Function

Export a new `cachedInvokeDiff(directory, filePath, baseRef, targetRef, algorithm): Promise<DiffResult>` that:

1. Builds cache key from arguments
2. On cache hit: returns cached `DiffResult` immediately, no IPC
3. On cache miss: increments a per-key request counter, invokes Rust, and only caches/returns the result if that key's counter hasn't advanced (prevents stale writes from out-of-order async resolutions)
4. Does not cache error results

This function is stateless (doesn't touch `state.diffResult`) so it can be safely used by both the editor and review page.

## computeDiff Changes

`computeDiff` calls `cachedInvokeDiff` internally, then writes the result to `state.diffResult` / `state.loading` / `state.error` as before. No change to its external API.

## Invalidation

| Trigger | What to invalidate | How |
|---|---|---|
| File save (existing watcher) | All entries for that `directory::filePath` | Call `invalidateFile(directory, filePath)` from App.svelte's watcher callback |
| `exitDiff()` | Entire cache | `diffCache.clear()` |
| GitHub session change | Entire cache | Clear from GitHub review store mutation functions (`activateGitHubReviewSession`, `resyncSession`, `discardSession`, `submitReview`) or via an `$effect` watching `activeSession` in the diff store |
| `setDiffRefs` / `swapRefs` | Nothing (new key, old entries stay for potential re-navigation) | No action needed |

After invalidating the currently-displayed cache key, trigger an immediate recompute.

## Review Page Integration

`reviewPage.svelte.ts` currently calls `invoke("compute_diff", ...)` directly (line 160). Change it to import and call `cachedInvokeDiff` from the diff store. This returns a `DiffResult` per file without touching global editor diff state, so batch loading in the review page works without cross-view interference.

## What We're NOT Doing

- **No LRU cap**: Cache is session-scoped with natural file count limits. DiffResult contains full file contents but reviewer file counts are bounded.
- **No in-flight dedup**: Per-key request counters handle races simply enough.
- **No Rust-side caching**: TS cache is simpler and avoids cross-IPC invalidation signals.
- **No git ref change detection**: For PR reviews, refs are fixed SHAs (cache keys change naturally on session refresh). For local reviews, file saves are the primary trigger. Symbolic ref movement (branch switch, HEAD move) without a save is an edge case we accept for now.

## Future Consideration

If the remote PR is force-pushed/rebased while the user is reviewing, the local worktree and cached diffs remain correct for what's checked out. The cache invalidates when the session refreshes with new SHAs. Detecting remote PR changes and prompting a refresh is a separate concern (not part of this work).

## Known Limitations

- **File watcher scope**: The existing watcher only monitors the currently-selected file. Edits to non-active files (e.g., by external tools) won't invalidate their cache entries. Acceptable because the review page reloads on entry, and the editor only displays one file at a time.

## Files to Modify

1. `src/lib/stores/diff.svelte.ts` — add cache Map, per-key request counters, `cachedInvokeDiff`, invalidation functions (`invalidateFile`, `clearCache`), update `computeDiff` and `exitDiff`
2. `src/App.svelte` — call `invalidateFile` from file watcher callback
3. `src/lib/stores/githubReview.svelte.ts` — call `clearCache` from session mutation functions
4. `src/lib/stores/reviewPage.svelte.ts` — replace direct `invoke("compute_diff")` with `cachedInvokeDiff`
