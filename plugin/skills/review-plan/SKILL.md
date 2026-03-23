---
name: review-plan
description: This skill should be used when the user says "/review-plan", "review the plan", "review the spec", "send plan to red pen", or wants a human to review a plan or spec document in the Red Pen desktop app before implementation begins.
allowed-tools: Bash, Read, Glob
---

# Review Plan with Red Pen

Open a plan or spec document in the Red Pen desktop app for human review before proceeding with implementation.

## Workflow

1. **Find the plan document:**
   - Check for recent files in `docs/superpowers/specs/` matching today's date or the most recent spec
   - If multiple candidates exist, pick the most recently modified one
   - If no plan files are found, inform the user and stop

2. **Open in Red Pen and wait for review:**
   ```bash
   redpen open <plan-file> --wait --timeout 600
   ```

3. **Parse and act on the verdict:**
   - **approved** — report approval. The plan is good to proceed with implementation.
   - **changes_requested** — read each annotation's `body` as feedback on the plan. The `anchor.startLine` and `anchor.lineContent` indicate which section of the plan the feedback applies to. Revise the plan document and reply to each annotation with `redpen annotate <file> --body "Updated — <summary>" --reply-to <annotation-id>`. Only ask clarifying questions if feedback is genuinely unclear.

## Finding Plan Files

Search in this order:
1. `docs/superpowers/specs/*.md` — sorted by modification time, most recent first
2. Any `.md` file in the working directory with "plan" or "spec" in the name

If an argument is provided to the skill, use that path directly instead of searching.
