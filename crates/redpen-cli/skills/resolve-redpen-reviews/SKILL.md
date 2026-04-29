---
name: resolve-redpen-reviews
description: Resolve all PR review comments (human and bot) on the active Redpen review session. Reads comments via the local Redpen server, fixes real issues, dismisses false positives, and posts replies as drafts inside Redpen for the human to publish.
license: MIT
allowed-tools: Bash(redpen *) Bash(git add *) Bash(git commit *) Bash(git push *) Bash(git status *) Bash(git diff *) Bash(git log *)
metadata:
  scope: redpen
  semantics: drafts-only
---

Resolve all review comments on the active Redpen GitHub review session, in two phases: fix everything that's open, then watch for new comments until quiet.

**This skill operates against the Redpen desktop app's local server, not api.github.com.** Replies you post become PendingPublish drafts inside Redpen sidecars; the human reviewer publishes them upstream by clicking "Submit review" in the Redpen UI. You never touch real GitHub.

## Prerequisites

1. **Redpen desktop app must be running.** Run `redpen url` to confirm. If it errors with "redpen server is not running", ask the user to start the desktop app (`cargo tauri dev` or open Redpen normally) and retry.
2. **A GitHub PR review session must be loaded.** Run `redpen sessions` to list active sessions. If none are loaded, ask the user to open one in Redpen (`redpen review-pr <owner/repo#number>` or via the GitHub Inbox in the app), then retry.
3. **`agent-reviews` binary on PATH.** The Redpen review loop delegates to it for the comment-management UX. Install with:

    ```bash
    npm i -g agent-reviews
    # or, until the GITHUB_API_URL upstream PR merges:
    npm i -g github:sam-phinizy/agent-reviews#add-github-api-url-env
    ```

    Or set `REDPEN_REVIEW_BIN=/path/to/agent-reviews` to override.

If multiple sessions are active, this skill operates on the first one returned by `redpen sessions` unless the user specifies a different `--pr <number>` explicitly.

## Phase 1: FETCH & FIX (synchronous)

### Step 1: Pick the active session and fetch comments

Run `redpen sessions --format=json` and pick a target. If exactly one session is active, use its `number`. If multiple, ask the user which.

Then fetch all unanswered comments with full detail:

```bash
redpen review --pr <number> --unanswered --expanded
```

`redpen review` is a thin wrapper that execs `agent-reviews` with `GITHUB_API_URL` pointed at the local Redpen server, so the comments come from the active session's sidecars (originally imported from real GitHub when the session was opened). Every comment in the listing is a real review left by a human or bot on the underlying PR.

If zero comments are returned, print "No unanswered comments found" and skip to Phase 2.

### Step 2: Process Each Unanswered Comment

For each comment, evaluate:

#### For Bot Comments (Copilot, CodeRabbit, Cursor, Sourcery, etc.)

Read the referenced code and classify:

1. **TRUE POSITIVE** — A real issue that needs fixing
2. **FALSE POSITIVE** — Not a real bug (intentional pattern, bot misread, framework-specific)
3. **UNCERTAIN** — Ask the user

Bots have a high false-positive rate. Verify before fixing.

#### For Human Comments

1. **ACTIONABLE** — Reviewer identified a real issue or requested a concrete change
2. **DISCUSSION** — Valid point, right approach unclear → ask the user
3. **ALREADY ADDRESSED** — Concern is no longer relevant

Human reviewers usually have context you don't. When unsure, defer to the author.

#### Act on the Evaluation

- **TRUE POSITIVE / ACTIONABLE** — Fix the code. Track `(comment_id, brief description of fix)`.
- **FALSE POSITIVE** — Do not change code. Track `(comment_id, reason)`.
- **DISCUSSION** — Ask the user, apply their decision, track outcome.
- **ALREADY ADDRESSED** — Track with the explanation.
- **UNCERTAIN** — Ask the user; if they say skip, track as skipped.

Do NOT reply to comments yet. Replies happen after the commit (Step 4).

### Step 3: Commit and Push

After evaluating and fixing every unanswered comment:

1. Run the project's lint / type-check / tests as appropriate.
2. Stage, commit, and push:

    ```bash
    git add -A
    git commit -m "fix: address PR review findings

    {bullet list of changes, grouped by reviewer or bot}"
    git push
    ```

3. Capture the short commit hash from the output.

### Step 4: Reply to Every Processed Comment

Now that the commit hash exists, reply to each comment using `redpen review`. The `--resolve` flag marks the thread resolved inside Redpen; once the human submits the Redpen review, the thread resolution propagates to GitHub along with the replies.

For each TRUE POSITIVE / ACTIONABLE:

```bash
redpen review --pr <number> --reply <comment_id> "Fixed in {hash}. {brief description}" --resolve
```

For each FALSE POSITIVE:

```bash
redpen review --pr <number> --reply <comment_id> "Won't fix: {reason}. {explanation}" --resolve
```

For DISCUSSION (after user decision):

```bash
redpen review --pr <number> --reply <comment_id> "{outcome}. {explanation}" --resolve
```

For ALREADY ADDRESSED:

```bash
redpen review --pr <number> --reply <comment_id> "Already addressed. {when/how}" --resolve
```

For SKIPPED:

```bash
redpen review --pr <number> --reply <comment_id> "Skipped per user request" --resolve
```

**Drafts-only reminder:** every reply you post lands as a `PendingPublish` annotation in the Redpen sidecar under your bearer-token identity. Nothing is sent to api.github.com until the human reviewer clicks "Submit review" in the Redpen UI. If you change your mind about a reply before the human submits, the user can edit or delete the draft directly in Redpen.

**Do NOT start Phase 2 until all replies are posted.**

## Phase 2: POLL FOR NEW COMMENTS (loop until quiet)

The watcher exits as soon as new comments arrive (after a short grace period to catch batch posts). Run it in a loop until it times out cleanly.

### Step 5: Watcher Loop

Repeat until the watcher reports `WATCH COMPLETE`:

1. Launch the watcher as a background task. Use the native Redpen poller — it has no Node dependency and is the recommended path:

    ```bash
    redpen watch
    ```

    Both watchers emit the same exit markers (`EXITING WITH NEW COMMENTS` / `WATCH COMPLETE`) so the rest of this loop is identical either way. `redpen review --pr <number> --watch` (forwards to `agent-reviews --watch`) is also available if you have a reason to use it.
2. Wait for the watcher to complete.
3. Read the output:
    - **`EXITING WITH NEW COMMENTS`**: pull each new comment with `redpen review --pr <number> --detail <id>`, process them as in Phase 1 (Steps 2-4), then go back to step 1.
    - **`WATCH COMPLETE`**: stop the loop and produce the Summary Report.

Note: new comments here can include comments the human added to Redpen drafts AND new comments imported from GitHub if Redpen re-syncs the PR. Either way, treat them the same.

## Summary Report

```text
## PR Review Resolution Summary

### Results
- Fixed: X issues
- Already addressed: X
- Won't fix (false positives): X
- Discussion resolved: X
- Skipped per user: X

### By Reviewer/Bot
#### {reviewer or bot}
- {description} -- Fixed in {commit}
- {description} -- Won't fix: {reason}

### Status
All findings addressed. Watch completed.
Commit: {hash} (drafts in Redpen; awaiting human submit)
```

## Important Notes

### Drafts-only architecture
- `redpen review` routes through the local Redpen HTTP server, not api.github.com.
- Replies are local drafts under your bearer-token identity, not real GitHub comments.
- The human reviewer publishes by clicking "Submit review" in the Redpen UI.
- You can fearlessly retry, edit, or discard your work at any point during the loop.

### Response policy
- Every comment gets a reply. No silent ignores.
- For bots: replies stop them re-raising the same finding next sync.
- For humans: replies surface in Redpen and (after submit) on GitHub.

### When uncertain, ask the user
- Architectural changes
- Subjective tradeoffs
- Multiple valid interpretations
- Fixes with non-obvious side effects

### Best practices
- Verify findings before fixing — bots have false positives, humans rarely do.
- Keep fixes minimal and focused; do not refactor unrelated code.
- Run lint and type-check before committing.
- Group related fixes into a single commit.
- Copilot `suggestion` blocks usually contain ready-to-apply fixes.
- Prefer the human reviewer's specific code suggestion over your own paraphrase unless it introduces issues.
