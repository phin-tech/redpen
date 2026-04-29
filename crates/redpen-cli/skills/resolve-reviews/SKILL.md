---
name: resolve-reviews
description: Resolve all PR review comments (human and bot) on the active review session. Reads, fixes, replies, and resolves through the redpen CLI in two phases — fix everything that's open, then watch until quiet. Works against the local Redpen server (drafts) by default, or real GitHub with `--remote`.
license: MIT
allowed-tools: Bash(redpen *) Bash(git add *) Bash(git commit *) Bash(git push *) Bash(git status *) Bash(git diff *) Bash(git log *) Bash(git config *)
metadata:
  scope: redpen
---

Resolve all review comments on a PR in two phases: fix everything that's open, then watch for new comments until quiet.

By default, this skill operates against the **local Redpen server** — the desktop app's gh-fake HTTP endpoint. Replies become `PendingPublish` drafts in Redpen sidecars; the human reviewer publishes to GitHub by clicking "Submit review" in the Redpen UI. To operate directly against api.github.com instead (no Redpen mediation), pass `--remote` to every `redpen review` invocation.

## Prerequisites

1. **Local mode (default):** the Redpen desktop app must be running. Run `redpen url` to confirm. If it errors with "redpen server is not running", ask the user to start the app and retry. A GitHub PR review session must be loaded; check with `redpen sessions`.
2. **Remote mode (`--remote`):** a GitHub token via `gh auth login`, `GITHUB_TOKEN`, or `GH_TOKEN`. Cwd should be a clone of the target repo, or pass `--repo owner/name`.

If multiple Redpen sessions are loaded in local mode, the CLI uses the active session automatically when there's exactly one; if there are several, pass `--pr <number>` and `--repo owner/name` explicitly.

## Phase 1: FETCH & FIX (synchronous)

### Step 1: Fetch unanswered comments

```bash
redpen review fetch --unanswered --expanded
```

Add `--remote` to operate against real GitHub instead of the local server. Add `--pr <n>` and `--repo owner/name` if the active context can't be auto-detected.

This returns each unanswered comment with its id, kind (CODE / ISSUE / REVIEW), author, file/line, full body, diff hunk, and any prior replies. The id in `[brackets]` is what you'll use to reply.

If the command prints "Found 0 comments", skip to Phase 2.

### Step 2: Process each unanswered comment

For each comment, classify and decide.

#### Bot comments (Copilot, CodeRabbit, Cursor, Sourcery, etc.)

1. **TRUE POSITIVE** — A real issue worth fixing
2. **FALSE POSITIVE** — Not a real bug (intentional pattern, bot misread, framework-specific)
3. **UNCERTAIN** — Ask the user

Bots have a high false-positive rate. Verify before fixing.

#### Human comments

1. **ACTIONABLE** — Reviewer identified a real issue or requested a concrete change
2. **DISCUSSION** — Valid point, right approach unclear → ask the user
3. **ALREADY ADDRESSED** — Concern is no longer relevant

Human reviewers usually have context you don't. When unsure, defer to the author.

#### Act on the evaluation

- **TRUE POSITIVE / ACTIONABLE** — Fix the code. Track `(comment_id, brief description)`.
- **FALSE POSITIVE** — Do not change code. Track `(comment_id, reason)`.
- **DISCUSSION** — Ask the user, apply their decision, track outcome.
- **ALREADY ADDRESSED** — Track with explanation.
- **UNCERTAIN** — Ask the user; if they say skip, track as skipped.

Do NOT reply yet. Replies happen after the commit (Step 4).

### Step 3: Commit and push

After evaluating and fixing every unanswered comment:

1. Run the project's lint / type-check / tests as appropriate.
2. Stage, commit, push:

    ```bash
    git add -A
    git commit -m "fix: address PR review findings

    {bullet list of changes, grouped by reviewer or bot}"
    git push
    ```

3. Capture the short commit hash from the output.

### Step 4: Reply to every processed comment

Now post replies. The `--resolve` flag marks the review thread resolved; in local mode this updates the sidecar, in remote mode this calls GraphQL `resolveReviewThread` against api.github.com.

For each TRUE POSITIVE / ACTIONABLE:

```bash
redpen review reply <comment_id> "Fixed in {hash}. {brief description}" --resolve
```

For each FALSE POSITIVE:

```bash
redpen review reply <comment_id> "Won't fix: {reason}. {explanation}" --resolve
```

For DISCUSSION (after user decision):

```bash
redpen review reply <comment_id> "{outcome}. {explanation}" --resolve
```

For ALREADY ADDRESSED:

```bash
redpen review reply <comment_id> "Already addressed. {when/how}" --resolve
```

For SKIPPED:

```bash
redpen review reply <comment_id> "Skipped per user request" --resolve
```

Add `--remote` to every reply if you've been operating in remote mode. The CLI auto-detects PR/repo from the active session (local) or git remote (remote); pass `--pr <n>` and `--repo owner/name` if needed.

**Local-mode reminder:** every reply you post is a `PendingPublish` annotation in the Redpen sidecar under your bearer-token identity. Nothing flies to api.github.com until the human submits the Redpen review. The user can edit or delete drafts at any point.

**Do NOT start Phase 2 until all replies are posted.**

## Phase 2: POLL FOR NEW COMMENTS (loop until quiet)

The watcher exits as soon as new comments appear (after a short grace period to catch batch posts). Run it in a loop until it times out cleanly.

### Step 5: Watcher loop

Repeat until the watcher reports `WATCH COMPLETE`:

1. Launch:

    ```bash
    redpen watch
    ```

    Add `--idle-timeout <s>` and `--poll-interval <s>` to override defaults of 600s and 30s.

2. Wait for it to complete.

3. Read the output:
    - **`EXITING WITH NEW COMMENTS`**: list new ids, fetch each via `redpen review detail <id>`, process them as in Phase 1 (Steps 2-4), then go back to step 1.
    - **`WATCH COMPLETE`**: stop the loop and produce the Summary Report.

Note: in local mode, new comments include both Redpen drafts the human added AND comments imported from a fresh PR sync. In remote mode, they're whatever appeared on api.github.com since the last poll. Treat them the same.

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
- {description} — Fixed in {commit}
- {description} — Won't fix: {reason}

### Status
All findings addressed. Watch completed.
Mode: local (drafts in Redpen, awaiting human submit)  |  remote (published to GitHub)
Commit: {hash}
```

## Important Notes

### Local vs remote mode

| Concern | Local (default) | Remote (`--remote`) |
|---|---|---|
| Source of comments | Redpen session sidecars (imported once when the session opened) | Live api.github.com |
| Effect of replies | PendingPublish drafts; human submits to push upstream | Posted directly to GitHub |
| Effect of `--resolve` | Updates sidecar locally; resolution propagates on submit | Real GraphQL `resolveReviewThread` mutation |
| Auth | Loopback only; bearer token is identity | `gh auth token` or `GITHUB_TOKEN`/`GH_TOKEN` |

Pick local for sandboxed work where the human wants final say; pick remote when you've verified the workflow and want to commit to upstream changes immediately.

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
- Keep fixes minimal and focused; don't refactor unrelated code.
- Run lint and type-check before committing.
- Group related fixes into a single commit.
- Copilot `suggestion` blocks usually contain ready-to-apply fixes.
- Prefer the human reviewer's specific code suggestion over your own paraphrase unless it introduces issues.
