#!/bin/bash
set -euo pipefail

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty')

if [ -z "$file_path" ]; then
  exit 0
fi

# Check if this is a plan/spec markdown file
case "$file_path" in
  *docs/superpowers/specs/*.md)
    echo '{"systemMessage": "A plan/spec document was just written. Consider running /review-plan to get human review in Red Pen before proceeding with implementation."}'
    exit 0
    ;;
esac

# Check filename for plan/spec pattern
basename=$(basename "$file_path")
case "$basename" in
  *plan*.md|*spec*.md|*Plan*.md|*Spec*.md)
    echo '{"systemMessage": "A plan/spec document was just written. Consider running /review-plan to get human review in Red Pen before proceeding with implementation."}'
    exit 0
    ;;
esac

# Not a plan/spec file — no output
exit 0
