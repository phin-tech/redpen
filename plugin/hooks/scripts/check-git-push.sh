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
  echo '{"decision": "block", "reason": "Run /review-code to review changes in Red Pen before pushing."}' >&2
  exit 2
fi

exit 0
