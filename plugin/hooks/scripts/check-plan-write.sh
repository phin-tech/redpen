#!/bin/bash
set -euo pipefail

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty')

if [ -z "$file_path" ]; then
  exit 0
fi

is_plan=false

# Check if this is a plan/spec markdown file by path
case "$file_path" in
  *docs/superpowers/specs/*.md|*/.claude/plans/*.md)
    is_plan=true
    ;;
esac

# Check filename for plan/spec pattern
if [ "$is_plan" = false ]; then
  basename=$(basename "$file_path")
  case "$basename" in
    *plan*.md|*spec*.md|*Plan*.md|*Spec*.md)
      is_plan=true
      ;;
  esac
fi

if [ "$is_plan" = true ]; then
  # Check if review has already been approved for this file
  approval_file="$CLAUDE_PROJECT_DIR/.redpen/signals/plan-reviewed"
  if [ -f "$approval_file" ]; then
    rm -f "$approval_file"
    exit 0
  fi

  echo '{"decision": "block", "reason": "A plan/spec document was just written. Run /review-plan to get human review in Red Pen before proceeding with implementation."}'
  exit 2
fi

exit 0
