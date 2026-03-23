---
name: review
description: This skill should be used when the user says "/review", "review this", "review these files", "open in red pen", "send to red pen", or wants to review specific files or all changed files in the Red Pen desktop app. Accepts an optional file or directory path argument.
---

# Review with Red Pen

Open files in the Red Pen desktop app for human review, block until the reviewer submits a verdict (approved or changes requested), then act on any annotations.

## Usage

- `/review` — review all git-changed files in the repo
- `/review src/main.rs` — review a specific file
- `/review src/` — review all files in a directory

## Workflow

1. **Determine files to review:**
   - If a path argument is provided, use it directly
   - If no argument, get all changed files via `git diff --name-only HEAD` and `git diff --name-only --staged`
   - If no changed files exist, inform the user and stop

2. **Open files in Red Pen and wait for review:**
   ```bash
   redpen open <file1> <file2> ... --wait --timeout 600
   ```
   This opens all files and blocks until the reviewer clicks "Approve" or "Request Changes".

3. **Parse the output** — the command outputs JSON:
   ```json
   {
     "verdict": "approved" | "changes_requested",
     "files": [
       { "file": "path/to/file", "annotations": [...] }
     ]
   }
   ```

4. **Act on the verdict:**
   - **approved** (or no annotations) — report approval and proceed
   - **changes_requested** — for each annotation, read the `body` field as reviewer feedback and the `anchor.lineContent` / `anchor.range.startLine` to locate the relevant code. Implement the requested changes. After making all changes, ask the user if they want another review pass.

## Annotation Format

Each annotation in the output has:
- `id` — unique annotation ID (use with `--reply-to` to reply)
- `body` — the reviewer's comment (treat as an instruction)
- `anchor.range.startLine` — line number in the file
- `anchor.lineContent` — the exact line of code commented on
- `labels` — any tags the reviewer applied

## Replying to Annotations

After implementing changes for an annotation, reply to acknowledge what was done:
```bash
redpen annotate <file> --body "Done — <brief summary>" --reply-to <annotation-id>
```
This creates a threaded reply visible in the Red Pen sidebar.

## Important

- Always use `--wait --timeout 600` to avoid blocking indefinitely (10 minute timeout)
- If the reviewer has no annotations and clicks Approve, the `files` array will be empty — this means approved
- After implementing changes from a "changes_requested" verdict, reply to each annotation with what was done. Only ask clarifying questions if the feedback is genuinely unclear.
