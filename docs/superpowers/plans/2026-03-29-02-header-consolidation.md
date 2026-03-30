# Header Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge the two-row header (ReviewWorkspaceHeader banner + WorkspaceToolbar) into a single consolidated row with three zones: PR context left, view switcher center, actions right.

**Architecture:** Move review context (PR title, repo link) and review actions (Resync, Discard, Submit) from `ReviewWorkspaceHeader` into `WorkspaceToolbar`. The toolbar becomes the single source of truth for the header row. `ReviewWorkspaceHeader` is removed from `App.svelte`. The toolbar gets new props for review context and actions.

**Tech Stack:** Svelte 5 (runes), CSS

---

### Task 1: Merge Review Context Into WorkspaceToolbar

**Files:**
- Modify: `src/components/editor-pane/WorkspaceToolbar.svelte`
- Modify: `src/components/EditorPane.svelte` (pass new props)
- Modify: `src/App.svelte` (remove ReviewWorkspaceHeader, pass context down)

- [ ] **Step 1: Add review context props to WorkspaceToolbar**

Add these props to `WorkspaceToolbar.svelte`:

```typescript
let {
  showPrView,
  onAgentReviewVerdict,
  onEnterDiff,
  onSelectCodeView,
  onSelectPrView,
  onSelectReviewView,
  // New review context props
  reviewContext,
  onResync,
  onDiscardPending,
  onSubmitReview,
  onOpenPullRequest,
  onOpenHelp,
  onLocalApprove,
  onLocalRequestChanges,
}: {
  showPrView: boolean;
  onAgentReviewVerdict: (verdict: "approved" | "changes_requested") => Promise<void>;
  onEnterDiff: (mode: import("$lib/types").DiffMode) => void;
  onSelectCodeView: () => void;
  onSelectPrView: () => void;
  onSelectReviewView: () => void;
  // New
  reviewContext: {
    hasContext: boolean;
    isGitHub: boolean;
    isSelfAuthored: boolean;
    repoLink: string;       // e.g. "phin-tech/test-repo #16"
    title: string;           // e.g. "feat: search functionality [WIP]"
    meta: string;            // e.g. "3 files to review"
    isLocalReview: boolean;
  } | null;
  onResync?: () => Promise<void>;
  onDiscardPending?: () => Promise<void>;
  onSubmitReview?: (action: "comment" | "approve" | "requestChanges", message?: string) => Promise<import("$lib/types").SubmitGitHubReviewResult | null>;
  onOpenPullRequest?: () => Promise<void>;
  onOpenHelp?: () => void;
  onLocalApprove?: () => Promise<void>;
  onLocalRequestChanges?: () => Promise<void>;
} = $props();
```

- [ ] **Step 2: Restructure toolbar template to three-zone layout**

Replace the existing toolbar template with the consolidated single-row layout:

```svelte
{#if editor.currentFilePath}
  <div class="workspace-toolbar">
    <!-- LEFT: Review context (truncatable) -->
    {#if reviewContext?.hasContext}
      <div class="toolbar-context">
        {#if reviewContext.isGitHub}
          <button class="toolbar-context-link" onclick={() => onOpenPullRequest?.()}>
            {reviewContext.repoLink}
          </button>
          <span class="toolbar-context-sep">·</span>
          <span class="toolbar-context-title">{reviewContext.title}</span>
        {:else}
          <span class="toolbar-context-text">{reviewContext.meta}</span>
        {/if}
      </div>
    {/if}

    <!-- CENTER: View switcher -->
    <div class="view-tabs">
      <button
        class="view-tab"
        class:active={!isReviewPageOpen() && !showPrView}
        onclick={onSelectCodeView}
      >
        Code
        {#if !isReviewPageOpen() && !showPrView}
          <span class="view-tab-underline"></span>
        {/if}
      </button>
      <button
        class="view-tab"
        class:active={isReviewPageOpen() && !showPrView}
        onclick={onSelectReviewView}
      >
        Review
        <span class="view-tab-badge" class:view-tab-badge-always={reviewAnnotationCount > 0}>
          {reviewAnnotationCount > 0 ? reviewAnnotationCount : ""}
        </span>
        {#if isReviewPageOpen() && !showPrView}
          <span class="view-tab-underline"></span>
        {/if}
      </button>
      {#if githubReview.activeSession}
        <button
          class="view-tab"
          class:active={showPrView}
          onclick={onSelectPrView}
        >
          PR
          {#if showPrView}
            <span class="view-tab-underline"></span>
          {/if}
        </button>
      {/if}
    </div>

    <!-- Moat -->
    <div class="toolbar-moat"></div>

    <!-- RIGHT: Actions (varies by mode) -->
    <div class="toolbar-actions">
      {#if reviewContext?.isGitHub}
        <!-- GitHub PR actions: icon buttons + submit -->
        <button class="toolbar-icon-btn" onclick={() => onResync?.()} title="Resync from GitHub">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M23 4v6h-6"/><path d="M1 20v-6h6"/>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
          </svg>
        </button>
        <button class="toolbar-icon-btn" onclick={() => onDiscardPending?.()} title="Revert pending changes">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 10h10a5 5 0 0 1 0 10H9"/><path d="M3 10l4-4"/><path d="M3 10l4 4"/>
          </svg>
        </button>
        <ReviewSubmitControl isSelfAuthoredPr={reviewContext.isSelfAuthored} {onSubmitReview} />
      {:else if reviewContext?.isLocalReview}
        <!-- Local review: ghost approve/request changes -->
        <button class="toolbar-ghost-btn toolbar-ghost-success" onclick={() => onLocalApprove?.()}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>
          Approve
        </button>
        <button class="toolbar-ghost-btn toolbar-ghost-danger" onclick={() => onLocalRequestChanges?.()}>
          Request changes
        </button>
      {:else}
        <!-- Code view: diff tools -->
        <DiffModeToggle onEnterDiff={onEnterDiff} />
        {#if diff.enabled}
          <DiffRefPicker {directory} filePath={editor.currentFilePath ?? ""} />
        {/if}
        <button
          class="toggle-btn"
          class:active={getBubblesEnabled()}
          onclick={() => toggleBubbles()}
          title="Toggle inline annotations"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
          </svg>
          Inline
        </button>
        {#if getBubblesEnabled()}
          <BubbleKindFilter />
        {/if}
      {/if}
    </div>
  </div>
{/if}
```

- [ ] **Step 3: Add new CSS classes for the consolidated layout**

Replace the `.workspace-toolbar` styles:

```css
.workspace-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  min-height: 52px;
  box-sizing: border-box;
  background: var(--surface-panel);
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
}
.toolbar-context {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
  overflow: hidden;
}
.toolbar-context-link {
  color: var(--accent);
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
  flex-shrink: 0;
  cursor: pointer;
  border: none;
  background: none;
  padding: 0;
  font-family: inherit;
}
.toolbar-context-link:hover {
  color: var(--accent-hover);
}
.toolbar-context-sep {
  color: var(--text-ghost);
  flex-shrink: 0;
}
.toolbar-context-title {
  color: var(--text-muted);
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.toolbar-context-text {
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
}
.view-tabs {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  background: var(--surface-raised);
  border-radius: 8px;
  flex-shrink: 0;
}
.view-tab {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-height: 30px;
  padding: 4px 12px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 12px;
  border-radius: 6px;
  cursor: pointer;
  font-family: inherit;
  white-space: nowrap;
}
.view-tab:hover {
  color: var(--text-secondary);
}
.view-tab.active {
  color: var(--view-active);
}
.view-tab-underline {
  position: absolute;
  bottom: 0;
  left: 8px;
  right: 8px;
  height: 2px;
  background: var(--accent-active);
  border-radius: 1px;
}
.view-tab-badge {
  font-size: 10px;
  font-family: var(--font-mono);
  color: var(--accent-badge-text);
  border: 1px solid var(--accent-badge-border);
  padding: 0 5px;
  border-radius: 999px;
  background: transparent;
  visibility: hidden;
}
.view-tab-badge-always {
  visibility: visible;
}
.toolbar-moat {
  width: 12px;
  flex-shrink: 0;
}
.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.toolbar-icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-default);
  background: transparent;
  color: var(--text-muted);
  border-radius: 6px;
  cursor: pointer;
}
.toolbar-icon-btn:hover {
  color: var(--text-secondary);
  border-color: var(--border-emphasis);
}
.toolbar-ghost-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 12px;
  border: none;
  background: transparent;
  font-size: 11px;
  font-weight: 500;
  font-family: inherit;
  border-radius: 6px;
  cursor: pointer;
}
.toolbar-ghost-success {
  color: var(--color-success);
}
.toolbar-ghost-success:hover {
  background: color-mix(in srgb, var(--color-success) 10%, transparent);
}
.toolbar-ghost-danger {
  color: color-mix(in srgb, var(--color-danger) 60%, transparent);
}
.toolbar-ghost-danger:hover {
  color: var(--color-danger);
  background: color-mix(in srgb, var(--color-danger) 10%, transparent);
}
```

- [ ] **Step 4: Fix pluralization in review metadata**

In `WorkspaceToolbar.svelte`, wherever file/annotation counts are shown, use proper pluralization:

```typescript
const fileCountLabel = $derived(
  `${reviewState.files.length} file${reviewState.files.length === 1 ? '' : 's'}`
);
```

- [ ] **Step 5: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 6: Commit**

```bash
git add src/components/editor-pane/WorkspaceToolbar.svelte
git commit -m "feat: restructure WorkspaceToolbar to consolidated three-zone layout"
```

---

### Task 2: Wire Props Through EditorPane

**Files:**
- Modify: `src/components/EditorPane.svelte` (accept and pass new props)

- [ ] **Step 1: Add new props to EditorPane**

EditorPane currently passes `showPrView`, `onAgentReviewVerdict`, `onEnterDiff`, and the view select callbacks to WorkspaceToolbar. Add the new review context props:

```typescript
let {
  // ...existing props...
  reviewContext,
  onResync,
  onDiscardPending,
  onSubmitReview,
  onOpenPullRequest,
  onOpenHelp,
  onLocalApprove,
  onLocalRequestChanges,
}: {
  // ...existing types...
  reviewContext: WorkspaceToolbar's reviewContext type | null;
  onResync?: () => Promise<void>;
  onDiscardPending?: () => Promise<void>;
  onSubmitReview?: (...) => Promise<...>;
  onOpenPullRequest?: () => Promise<void>;
  onOpenHelp?: () => void;
  onLocalApprove?: () => Promise<void>;
  onLocalRequestChanges?: () => Promise<void>;
} = $props();
```

Pass all new props through to `<WorkspaceToolbar>`.

- [ ] **Step 2: Build and verify**

Run: `npm run build`
Expected: Build succeeds (props passed through but not yet wired from App.svelte).

- [ ] **Step 3: Commit**

```bash
git add src/components/EditorPane.svelte
git commit -m "feat: forward review context props through EditorPane to WorkspaceToolbar"
```

---

### Task 3: Remove ReviewWorkspaceHeader and Wire App.svelte

**Files:**
- Modify: `src/App.svelte` (remove ReviewWorkspaceHeader, build reviewContext, pass to EditorPane)
- Modify: `src/lib/controllers/appShell.svelte.ts` (move review context derivations here)

- [ ] **Step 1: Remove ReviewWorkspaceHeader from App.svelte template**

In `App.svelte`, remove:
```svelte
<ReviewWorkspaceHeader onOpenHelp={appShell.openReviewShortcutHelp} />
```

Remove the import for `ReviewWorkspaceHeader`.

- [ ] **Step 2: Build reviewContext in appShell.svelte.ts**

Add derived state for the review context object that WorkspaceToolbar needs. This consolidates the logic that was in ReviewWorkspaceHeader:

```typescript
const reviewContext = $derived.by(() => {
  const gh = githubReview.activeSession;
  const local = reviewSession.active;
  if (!gh && !local) return null;
  return {
    hasContext: true,
    isGitHub: Boolean(gh),
    isSelfAuthored: Boolean(
      gh?.authorLogin && gh?.viewerLogin
      && gh.authorLogin.toLowerCase() === gh.viewerLogin.toLowerCase()
    ),
    repoLink: gh ? `${gh.repo} #${gh.number}` : "",
    title: gh ? gh.title : "Agent change review",
    meta: local && !gh
      ? `${reviewSession.files.length} file${reviewSession.files.length === 1 ? '' : 's'} to review`
      : `Into ${gh?.baseRef} from ${gh?.headRef}`,
    isLocalReview: local && !gh,
  };
});
```

Expose `reviewContext` and the action handlers (`onResync`, `onDiscardPending`, `onSubmitReview`, `onOpenPullRequest`, `onLocalApprove`, `onLocalRequestChanges`) from appShell so App.svelte can pass them to EditorPane.

- [ ] **Step 3: Pass reviewContext and action props to EditorPane in App.svelte**

```svelte
<EditorPane
  bind:ref={editorRef}
  bind:showShortcutHelp={appShell.state.showReviewShortcutHelp}
  onSelectionChange={appShell.handleSelectionChange}
  onOpenFolder={appShell.openFolderPicker}
  onJumpToFile={appShell.handleJumpToFile}
  reviewContext={appShell.reviewContext}
  onResync={appShell.resyncReview}
  onDiscardPending={appShell.discardPendingReview}
  onSubmitReview={appShell.submitReview}
  onOpenPullRequest={appShell.openPullRequest}
  onOpenHelp={appShell.openReviewShortcutHelp}
  onLocalApprove={appShell.localApprove}
  onLocalRequestChanges={appShell.localRequestChanges}
/>
```

- [ ] **Step 4: Import ReviewSubmitControl in WorkspaceToolbar**

The submit review button with dropdown is in `src/components/review-header/ReviewSubmitControl.svelte`. Import it in WorkspaceToolbar:

```typescript
import ReviewSubmitControl from "../review-header/ReviewSubmitControl.svelte";
```

- [ ] **Step 5: Remove redundant toolbar labels and shortcut hints**

Remove from WorkspaceToolbar:
- `<div class="toolbar-label">Workspace</div>`
- `<div class="toolbar-label">Review</div>`
- `<div class="toolbar-label">Code View</div>`
- `<kbd>s</kbd>` inline hint on the scope toggle button
- The `{reviewState.files.length} files · {reviewAnnotationCount} annotations` text span (count now in badge)

- [ ] **Step 6: Build and verify**

Run: `npm run build`
Expected: Build succeeds. The app shows a single header row instead of two.

- [ ] **Step 7: Commit**

```bash
git add src/App.svelte src/lib/controllers/appShell.svelte.ts src/components/EditorPane.svelte src/components/editor-pane/WorkspaceToolbar.svelte
git commit -m "feat: merge ReviewWorkspaceHeader into WorkspaceToolbar — single consolidated header row"
```

---

### Task 4: Clean Up Dead Code

**Files:**
- Potentially delete: `src/components/ReviewWorkspaceHeader.svelte` (if no longer imported anywhere)
- Potentially delete: `src/components/review-header/ReviewContextBanner.svelte` (context now inline in toolbar)
- Potentially delete: `src/components/review-header/ReviewActionsGroup.svelte` (actions now inline in toolbar)
- Modify: `src/components/ReviewWorkspaceHeader.svelte` CSS (move any needed styles to WorkspaceToolbar)

- [ ] **Step 1: Check for remaining imports of removed components**

Search for imports of `ReviewWorkspaceHeader`, `ReviewContextBanner`, `ReviewActionsGroup` across the codebase. If none exist, delete the files.

- [ ] **Step 2: Move any CSS from ReviewWorkspaceHeader that's still needed**

The ReviewSubmitControl component uses `:global(.review-summary-btn)` styles that were defined in ReviewWorkspaceHeader's `<style>` block. These need to move to either WorkspaceToolbar or ReviewSubmitControl itself.

Check which `:global()` styles from ReviewWorkspaceHeader.svelte are still needed:
- `.review-summary-btn` and variants
- `.review-summary-dropdown` and children
- `.review-submit-*` styles

Move them to `WorkspaceToolbar.svelte`'s `<style>` block or into `ReviewSubmitControl.svelte` if they're scoped to that component.

- [ ] **Step 3: Delete unused files**

```bash
git rm src/components/ReviewWorkspaceHeader.svelte
git rm src/components/review-header/ReviewContextBanner.svelte
git rm src/components/review-header/ReviewActionsGroup.svelte
```

(Only if confirmed no remaining imports.)

- [ ] **Step 4: Build and verify**

Run: `npm run build`
Expected: Build succeeds with no missing import errors.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove ReviewWorkspaceHeader and subcomponents — logic merged into WorkspaceToolbar"
```

---

### Task 5: Visual Polish and Edge Cases

**Files:**
- Modify: `src/components/editor-pane/WorkspaceToolbar.svelte`

- [ ] **Step 1: Handle the "no review context, no file open" state**

When `editor.currentFilePath` is null, the toolbar is hidden entirely (existing behavior). Verify this still works.

- [ ] **Step 2: Handle code view mode with active review context**

When in Code view but a review session is active, the right zone should show diff tools (not review actions). The review actions only appear in the right zone when the Review tab is active OR in the top-level context. Verify the conditional logic is correct:

- Code tab active + review context → right shows diff tools, review badge visible on Review tab
- Review tab active + GitHub context → right shows Resync/Discard/Submit
- Review tab active + local context → right shows ghost Approve/Request Changes
- PR tab active → right shows "PR overview" info or is empty

- [ ] **Step 3: Verify markdown preview toggle still works**

The Source/Preview toggle for markdown files was in the Code View section of the old toolbar. Verify it's still present in the new layout when a markdown file is open.

- [ ] **Step 4: Build and verify**

Run: `npm run build`
Expected: Build succeeds.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: header edge cases — markdown toggle, code-view-with-context, empty state"
```
