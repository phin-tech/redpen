# Pre-Push Review Gate

Add `--diff-base` and `--pre-push` flags to `redpen open` so it can serve as a composable pre-push hook command. When invoked, it computes the changed files, opens them in the desktop app for human review, and blocks until a verdict is given.

Related: [#46](https://github.com/phin-tech/redpen/issues/46)

## Motivation

When an AI agent writes code and pushes, there's no human review gate before the code leaves the local machine. A pre-push hook that opens redpen creates a natural checkpoint: the agent pushes, the hook intercepts, the human reviews, and annotations flow back to the agent if changes are requested.

## CLI Changes

### New flags on `redpen open`

**`--diff-base <sha>`**

Computes changed files via `git diff --name-only <sha>..HEAD`. Filters to files that exist on disk (excludes deletes). Opens the resulting list in redpen.

Mutually exclusive with positional file arguments. When `--diff-base` or `--pre-push` is used, positional paths become optional (currently they are required).

Uses `git diff --diff-filter=d` to exclude deleted files. Renames are included using the new path.

**`--pre-push`**

Reads git's pre-push stdin format:

```
<local-ref> <local-sha> <remote-ref> <remote-sha>
```

Extracts `<remote-sha>` as the diff base and delegates to the `--diff-base` codepath. If multiple refs are being pushed, uses the first one.

If `<remote-sha>` is all zeros (new branch), diffs against the merge-base with the default branch.

Implies `--wait` (no need to pass `--wait` separately).

**`--no-timeout`**

Overrides the default timeout. When `--pre-push` is set, the default timeout is 600 seconds (10 minutes). Without `--pre-push`, existing behavior is unchanged (no default timeout).

Precedence: `--no-timeout` > `--timeout <N>` > `--pre-push` default (600s) > existing default (none).

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Approved — push proceeds |
| 1 | Changes requested — push blocked |
| 2 | Timeout — push blocked |
| 3 | Could not connect to redpen app (launch failed) |

Requires refining `server_client` error types to distinguish timeout from connection failure (currently both collapse to `None`).

### Stdout/stderr contract

- **Stdout**: JSON output (verdict + annotations) — preserved for machine consumption by agents
- **Stderr**: Human-readable summary on rejection (see below)

### Stderr output on rejection

When the verdict is "changes requested", the CLI prints a summary to stderr:

```
Review verdict: changes_requested
Session: <session-id>
2 annotations on src/lib/foo.ts
1 annotation on src/main.rs
Push blocked. Fix the flagged issues and push again.

Run `redpen list --session <session-id>` for full annotation details.
```

### New flag on `redpen list`

**`--session <id>`**

Lists all annotations across all files in the given review session as JSON. This gives agents and humans a single command to retrieve everything flagged in a review.

Requires a new server RPC endpoint (e.g., `GET /rpc/session.annotations?id=<session-id>`) since the current `list` command is file-based only. The server needs to track which files belong to a session — currently only `primary_file_path` is stored; multi-file sessions need a file index.

## App Auto-Launch

When the CLI can't find a running redpen server (no `server.json` or connection refused):

1. Launch the desktop app via `open -a "Red Pen"` (macOS only for now)
2. Poll for server availability: 5 attempts, 1 second apart
3. If the app comes up, proceed normally
4. If not, exit with code 3: `"Could not connect to Redpen. Is it installed?"`

Cross-platform launch (Linux, Windows) is out of scope for now.

## Annotation Feedback Loop

No new work needed. The existing flow handles this:

1. On "changes requested", annotations are preserved in the sidecar (existing behavior)
2. Annotations are posted to the local channel on port 8789 (existing behavior)
3. The agent receives annotations via MCP, sees the feedback, fixes the code, and pushes again
4. The push triggers the hook again for another review pass

## Integration with prek

The user adds a hook entry to `prek.toml`:

```toml
[[repos.hooks]]
id = "redpen-review"
name = "Red Pen review gate"
entry = "redpen open --pre-push --wait --timeout 600"
language = "system"
pass_filenames = false
stages = ["pre-push"]
```

Timeout and other flags are configured here. Redpen provides sensible defaults; prek config is where the user customizes.

## Agent Loop

```
Agent pushes
  -> prek runs pre-push hooks
    -> redpen open --pre-push --wait
      -> Desktop app opens with changed files
        -> Human reviews
          -> Approved: exit 0, push proceeds
          -> Changes requested: exit 1, push blocked
            -> Annotations posted to channel (port 8789)
            -> Agent receives annotations via MCP
            -> Agent fixes code, pushes again
            -> Loop repeats
```

## Scope

### In scope
- `--diff-base <sha>` flag on `redpen open`
- `--pre-push` flag on `redpen open` (reads git stdin, delegates to `--diff-base`)
- `--no-timeout` flag on `redpen open`
- Structured exit codes (0/1/2/3)
- Stderr summary on rejection with session ID
- `--session <id>` flag on `redpen list`
- macOS auto-launch of desktop app

### Out of scope
- Cross-platform app launch (Linux, Windows)
- Changes to plugin skills or hooks (separate issue: #47)
- prek configuration (user's responsibility)

## Edge Cases

- **Empty/malformed pre-push stdin**: exit 1 with a clear error message
- **Push deletions** (local sha all zeros): skip — nothing to review
- **Tag pushes**: skip — no file diff to review
- **New branch** (remote sha all zeros): diff against merge-base with default branch; if default branch can't be determined locally, fall back to `HEAD~10` or error
- **No changed files**: exit 0 silently (nothing to review, allow push)
