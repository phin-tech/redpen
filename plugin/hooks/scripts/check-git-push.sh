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
  repo_root=$(git rev-parse --show-toplevel 2>/dev/null || echo "$CLAUDE_PROJECT_DIR")
  port="${REDPEN_PORT:-8789}"

  # Primary: check if the Red Pen server reports push-approved
  server_response=$(curl -sf --max-time 2 "http://127.0.0.1:${port}/rpc/push.check" \
    -X POST -H "Content-Type: application/json" \
    -d "{\"repo_root\": \"$repo_root\"}" 2>/dev/null || echo "")

  if echo "$server_response" | jq -e '.approved == true' >/dev/null 2>&1; then
    exit 0
  fi

  # Fallback: legacy file-based signal (for environments where server isn't running)
  approval_file="$repo_root/.redpen/signals/push-approved"
  if [ -f "$approval_file" ]; then
    rm -f "$approval_file"
    exit 0
  fi

  # Direct mode: if redpen CLI is available, open changed files directly for review
  if command -v redpen >/dev/null 2>&1; then
    review_output=$(redpen open --diff-remote --wait --timeout 600 2>/dev/null) || review_exit=$?
    review_exit=${review_exit:-0}

    if [ "$review_exit" -eq 0 ]; then
      # Approved — push can proceed
      exit 0
    elif [ "$review_exit" -eq 1 ] && [ -n "$review_output" ]; then
      # Changes requested — return annotations as structured feedback
      cat <<BLOCK
{"decision": "block", "reason": "Review completed — changes requested. Annotations from the reviewer are below. Fix the issues and try pushing again.", "systemMessage": $review_output}
BLOCK
      exit 2
    elif [ "$review_exit" -eq 2 ]; then
      echo '{"decision": "block", "reason": "Review timed out. Run /review-code or try pushing again."}'
      exit 2
    elif [ "$review_exit" -eq 3 ]; then
      # Red Pen app not running — fall through to the manual prompt
      echo '{"decision": "block", "reason": "Run /review-code to review changes in Red Pen before pushing."}'
      exit 2
    fi
  fi

  echo '{"decision": "block", "reason": "Run /review-code to review changes in Red Pen before pushing."}'
  exit 2
fi

exit 0
