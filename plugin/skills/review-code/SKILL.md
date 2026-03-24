---
name: review-code
description: This skill should be used when the user says "/review-code", "review the code", "review before push", "code review in red pen", or wants a human to review code changes in the Red Pen desktop app before pushing to remote.
allowed-tools: Bash, Read, Glob
---

# Review Code with Red Pen

Open all changed code files in the Red Pen desktop app for human review, typically before pushing to remote.

## Workflow

1. **Identify changed files:**
   ```bash
   git diff --name-only HEAD
   ```
   Also include staged changes:
   ```bash
   git diff --name-only --staged
   ```
   Combine and deduplicate the lists. If no changed files, inform the user and stop.

2. **Open all changed files in Red Pen and wait for review:**
   ```bash
   redpen open <file1> <file2> ... --wait --timeout 600
   ```

3. **Parse and act on the verdict:**
   - **approved** — create the push approval signal so the pre-push hook allows the next push:
     ```bash
     repo_root=$(git rev-parse --show-toplevel)
     mkdir -p "$repo_root/.redpen/signals" && echo "approved" > "$repo_root/.redpen/signals/push-approved"
     ```
     Report approval. Code is ready to push.
   - **changes_requested** — for each file's annotations, read the `body` as reviewer feedback on that specific line (`anchor.range.startLine`, `anchor.lineContent`). Implement the requested changes and reply to each annotation with `redpen annotate <file> --body "Done — <summary>" --reply-to <annotation-id>`. Commit the fixes. Only ask clarifying questions if feedback is genuinely unclear.

## Output Format

`redpen wait` outputs JSON with verdict and per-file annotations:
```json
{
  "verdict": "approved" | "changes_requested",
  "files": [
    {
      "file": "relative/path/to/file",
      "annotations": [
        {
          "id": "annotation-uuid",
          "body": "reviewer comment",
          "anchor": {
            "range": { "startLine": 42 },
            "lineContent": "the exact line"
          }
        }
      ]
    }
  ]
}
```

## Important

- Only include actual source files in the review, not build artifacts or lock files
- After implementing changes, create a new commit with the fixes before offering another review pass
