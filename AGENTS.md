# Redpen

A Tauri desktop app for code review, built with Svelte 5 (runes) and Rust.

## Two Review Paths

The app has two distinct review workflows that share the diff store and annotation system but differ in how they source refs, store annotations, and submit results.

### 1. GitHub PR Review

- **Entry**: `activateGitHubReviewSession()` via deep link or inbox queue
- **Store**: `src/lib/stores/githubReview.svelte.ts`
- **Diff refs**: Fixed to PR's `baseSha`/`headSha` from the GitHub API
- **Annotations**: Stored in session-specific sidecars under `~/.redpen/sessions/`, include `GitHubAnnotationMetadata` with sync state (`PendingPublish`/`Published`)
- **Submission**: Publishes annotations as GitHub PR review comments via `gh api`
- **Key Rust commands**: `src-tauri/src/commands/github_review.rs`
- **Key components**: `GitHubInbox.svelte`, `ReviewWorkspaceHeader.svelte`, `PullRequestView.svelte`

### 2. Local Redpen Review

- **Entry**: Manual file selection or `addReviewFile()`
- **Store**: `src/lib/stores/review.svelte.ts`
- **Diff refs**: Defaults to `HEAD`/`working-tree`, user can change via `DiffRefPicker`
- **Annotations**: Stored in standard project-level sidecars via `AnnotationService`
- **Submission**: Writes verdict to `.redpen/signals/review.session`, posts to local channel (port 8789)
- **Key Rust commands**: `src-tauri/src/commands/annotations.rs`
- **Key components**: `AnnotationSidebar.svelte`, `DiffRefPicker.svelte`, `ReviewPage.svelte`

### Shared Infrastructure

- **Diff store**: `src/lib/stores/diff.svelte.ts` — both paths use `computeDiff` and `enterDiff`
- **Annotations store**: `src/lib/stores/annotations.svelte.ts` — annotation CRUD routes through Rust commands that check for an active GitHub session first, falling back to the local service
- **Orchestration**: `EditorPane.svelte` detects which path is active and sets diff defaults accordingly

## Tech Stack

- **Frontend**: Svelte 5 (runes, `$state`, `$effect`), TypeScript
- **Backend**: Rust (Tauri v2), ts-rs for type generation
- **Styling**: CSS (no framework)
- **Types**: Generated from Rust structs via ts-rs into `src/lib/bindings/`

## Build & Dev

```
npm run dev        # Start dev server
npm run build      # Production build
cargo tauri dev    # Full Tauri dev (frontend + backend)
cargo tauri build  # Production Tauri build
```
