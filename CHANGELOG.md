## [0.1.8] - 2026-04-08

### Bug Fixes

- Delete sidecar file when all annotations are removed (#16)
Empty sidecar JSON files were left behind after deleting all annotations,
  causing the GUI to show files as annotated when they had no annotations.
- Resolve push approval signal path in git worktrees (#18)
Use `git rev-parse --show-toplevel` instead of `$CLAUDE_PROJECT_DIR` to
  locate the push-approved signal file. This ensures the hook and the
  review-code skill agree on the signal path regardless of whether the
  push originates from the main repo or a worktree.
- Remove panic-prone unwrap() from non-test runtime paths (#31)
Replace unwrap() calls with recoverable error handling:
  - Mutex locks: unwrap_or_else(|e| e.into_inner()) or map_err for Tauri commands
  - Path ops (workdir, file_name): fallbacks or ok_or() with descriptive errors
  - Deep link handlers: if-let/let-else patterns
- Add ts-rs type overrides for datetime fields in Annotation

### Documentation

- Add event/service boundary design spec (#36)
- Add event/service boundary implementation plan (#36)
- Add review page design spec
Spec for a full-screen feed view with two modes: Review Changes
  (diff + inline annotations) and Agent Feedback (annotation-only).
- Add review page implementation plan
10-task plan covering Rust resolved field, read_file_lines command,
  review page store, ReviewCodeSnippet/ReviewCard/ReviewPage components,
  EditorPane wiring, command palette integration, and keyboard shortcuts.
- Add workspace launch redesign spec
Agent status contract (POST /agent/status → SQLite → Tauri event),
  PID crash detection, 60s heartbeat staleness, LOCAL/INBOX layout
  separation, Recent Sessions dedup, and × button git-refresh behavior.

### Features

- Add Homebrew tap support (#19)
* feat: add Homebrew tap support for CLI and desktop app

  Add release:upload CLI tarball packaging and release:brew task that
  updates phin-tech/homebrew-tap with the correct Formula and Cask
  for each release.
- OS-level notifications for annotation replies (#22)
* feat: add tauri-plugin-notification and url dependencies
- Add three-mode diff view (#35)
* docs: add diff view design spec

  Spec for three-mode diff viewing (split, unified, highlights) with
  CodeMirror 6 integration, Rust-side diff computation via similar crate,
  and ref picker UI.
- Generate shared TS types from Rust with ts-rs (#29)
Replace hand-maintained TypeScript type definitions with types generated
  from Rust structs via ts-rs. Also fixes missing `similar` crate
  dependency from the diff view merge.
- Extract annotation service behind EventBus trait (#36)
Introduces `redpen-runtime` crate with framework-agnostic AnnotationService<E: EventBus>
  that encapsulates annotation CRUD logic. Tauri commands become thin wrappers delegating
  to the service via TauriEventBus adapter. Runtime tests run without Tauri bootstrapping.
- Add resolved field to Annotation struct
Adds a `resolved: bool` field to Annotation with serde default/skip attributes
  for backward compatibility with existing sidecar JSON files. Updates both
  `Annotation::new()` and `Annotation::new_reply()` constructors. Regenerates
  TypeScript bindings to include `resolved?: boolean`.
- Add read_file_lines command and resolve support to update_annotation
- Add FileSnippet struct and read_file_lines Tauri command for code snippet retrieval
  - Add resolved param to update_annotation command and update_annotation_full service method
  - Add readFileLines and updateAnnotation (with resolved) TypeScript wrappers
  - Add resolveAnnotation helper to annotations store
- Add review page store with changes and feedback modes
- Add ReviewCodeSnippet component for review page
- Add ReviewCard component for review page
- Add ReviewPage component with feed layout and keyboard nav
- Wire ReviewPage into EditorPane conditional rendering
- Add review page commands and Cmd+Shift+R shortcut
- Add Review button to annotation sidebar
- Enhance review page with scope toggle, keyboard nav, and diff-only file support
Add session/all-changes scope toggle (s key), vim-style navigation (gg/G top/bottom,
  h/l expand/contract snippets, H/L contract, R reset), choice selection via number keys
  (1-9), and display of diff-only files in all-changes mode. Fix self-referential $derived
  bug that caused Loading... state, fix Svelte $state naming conflict, fix Escape key
  handling in reply input. Add dev:clean task to Taskfile.
- Add CLI choices/replies, bubble kind filter, codemirror theme updates, and docs
- Add Lezer-based syntax highlighting to review page code snippets
Use highlightTree + classHighlighter for lightweight tokenization instead
  of full CodeMirror instances. Deleted diff lines are excluded from parsing
  to keep the syntax tree coherent. Map .svelte to JS/TS parser for better
  script block highlighting.
- Add delete confirmation, escape handling, color consistency, and shortcut system
- Add lightbox delete confirmation (dd) with Escape to dismiss
  - Fix Escape in delete lightbox closing entire review page
  - Standardize color tokens (--color-success, --color-danger) across all components
  - Extract shortcut matching into shared shortcuts.ts utility
  - Update design tokens and codemirror theme for consistency
- Integrate github review workflow
- Pre-push review gate (#46) (#48)
* docs: add pre-push review gate design spec for issue #46
- Add Approve/Request Changes to local review header, move Help right (#49)
- Add Approve and Request Changes buttons to ReviewWorkspaceHeader for
    local review sessions (matching the GitHub PR review action buttons)
  - Move Help button to the right side of the header
  - Wire buttons to submitReviewVerdict + clearReviewSession
  - Add success/danger button style variants to the header
- Comprehensive native menu bar with submenus and icons (#50)
* feat: comprehensive native menu bar with submenus and icons

  Replaces the minimal 4-item menu with a full macOS menu bar:
  - File: Open Folder (⌘O), Go to File (⌘P), Export Annotations
  - Edit: standard items + Find (⌘F), Find Next (⌘G), Find Prev (⌘⇧G)
  - Annotations: Add (⌘↩), Reload, Clear — with native icons
  - View: Command Palette (⌘K), Markdown Preview (⌘⇧M), Diff submenu
  - Review: Review Changes (⌘⇧R), Agent Feedback, Approve/Request Changes with status icons
  - Window: standard Minimize/Close

  All shortcuts align with existing in-app vim-style keybindings. Menu events
  route through Tauri emit → App.svelte listeners → runCommand/editorRef.
- Comprehensive UI overhaul — theme, header, bubbles, sidebars, review page, launch screen (#54)
* feat: comprehensive native menu bar with submenus and icons

  Replaces the minimal 4-item menu with a full macOS menu bar:
  - File: Open Folder (⌘O), Go to File (⌘P), Export Annotations
  - Edit: standard items + Find (⌘F), Find Next (⌘G), Find Prev (⌘⇧G)
  - Annotations: Add (⌘↩), Reload, Clear — with native icons
  - View: Command Palette (⌘K), Markdown Preview (⌘⇧M), Diff submenu
  - Review: Review Changes (⌘⇧R), Agent Feedback, Approve/Request Changes with status icons
  - Window: standard Minimize/Close

  All shortcuts align with existing in-app vim-style keybindings. Menu events
  route through Tauri emit → App.svelte listeners → runCommand/editorRef.
- PR description annotations with paragraph-level targeting (#55)
* feat: add PR description annotations with block-level hover affordances

  - Create splitMarkdownBlocks utility for parsing PR body into
    annotatable blocks with line numbers
  - Create PrBodyAnnotatable component with hover gutter, numbered
    dots, annotation bubbles, and popover for creating annotations
  - Wire into PullRequestView replacing raw HTML rendering
  - Uses virtual file path (__redpen__/pr-body.md) to reuse
    existing annotation store and sidecar infrastructure
- Redesign settings panel — sidebar navigation, pill tags, toggle switches, repo cards (#56)
* docs: settings redesign spec

  * feat: add TagInput and ToggleSwitch reusable UI components
- Automate changelog generation with git-cliff (#57)
- Migrate push-approved check from file-based to server-based (#58)
Add /rpc/push.check endpoint to the redpen server that checks all
  tracked sessions for an "approved" verdict. Update the git push hook
  to query the server first, falling back to the legacy file signal
  for environments where the server isn't running.
- Workspace launch redesign — inbox categories, agent status, LOCAL/INBOX sections
Adds configurable GitHub inbox categories (ReviewRequested, Assigned,
  Authored, Mentioned) via global `gh search prs`, with org/repo exclude
  lists in Settings. Introduces agent status tracking: POST /agent/status
  and /agent/log server routes, SQLite persistence (migration 2), PID
  crash detection on startup, heartbeat staleness (60s), and frontend
  agent dots. Restructures launch screen into INBOX (GitHub PRs) and
  LOCAL (dirty repos) sections with unified row density, GitHub/computer
  icons, sort pills, slim search bar with autofocus. Removes Recent
  Sessions — if it's not dirty or unreviewed, it's off the dashboard.
- Review mode UX improvements — diff folding, pending comments sidebar, inline replies
- Collapse unchanged regions in diff view with expandable fold widgets
  - Trim large diff hunks to context window around annotations in Review tab
  - Show sync status badges (pending/submitted) on annotations and replies
  - Add pending comments sidebar grouped by file during GitHub review
  - Inline reply display with edit-on-double-click and delete for pending replies
  - Active line highlight (accent border) when selecting cards in Review tab
  - Smart scroll: only scroll when context block is off-screen, center on scroll
  - h/l expand keybinds now work with diff hunks in review mode
  - Jump-to-annotation from sidebar with fallback to closest root by line
- MacOS signing workflow and bundled CLI installation
Adds a complete build-sign-notarize-staple pipeline via Taskfile,
  bundles the redpen CLI into the .app and auto-installs it to
  ~/.local/bin on launch, and documents the macOS signing setup.

### Build

- Vendor libgit2 and openssl for portable git2 builds

