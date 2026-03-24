#!/bin/bash
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

if [ -z "$command" ]; then
  exit 0
fi

# Check if the command starts with or contains "git push" as an actual command
# (not just in a string/message). Look for git push at the start or after && ; |
if echo "$command" | grep -qE '(^|&&|\|\||;)\s*git push'; then
  # Check if a review approval exists — use git toplevel so this works in worktrees too
  repo_root=$(git rev-parse --show-toplevel 2>/dev/null || echo "$CLAUDE_PROJECT_DIR")
  approval_file="$repo_root/.redpen/signals/push-approved"
  if [ -f "$approval_file" ]; then
    # Consume the approval (one-time use)
    rm -f "$approval_file"
    exit 0
  fi

  echo '{"decision": "block", "reason": "Run /review-code to review changes in Red Pen before pushing."}'
  exit 2
fi

exit 0
