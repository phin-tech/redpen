#!/usr/bin/env bash
# Exercise the fake GitHub server via the real `gh` CLI.
#
# Required env: GH_HOST (host:port), GH_TOKEN, SSL_CERT_FILE (PEM trust),
# PARENT_ID (numeric id of the seeded comment).
#
# Exits 0 on success.

set -euo pipefail

: "${GH_HOST:?required}"
: "${GH_TOKEN:?required}"
: "${SSL_CERT_FILE:?required}"
: "${PARENT_ID:?required}"

# 1. List PRs by branch.
prs_json="$(gh api "repos/octocat/hello/pulls?head=octocat:feature/hello&state=open")"
pr_number="$(echo "$prs_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["number"])')"
if [[ "$pr_number" != "42" ]]; then
  echo "gh: expected PR 42, got $pr_number" >&2
  exit 1
fi
echo "gh: list pulls OK (number=$pr_number)"

# 2. List review comments.
comments_json="$(gh api "repos/octocat/hello/pulls/42/comments")"
ids="$(echo "$comments_json" | python3 -c 'import json,sys; print(",".join(str(c["id"]) for c in json.load(sys.stdin)))')"
if [[ "$ids" != "$PARENT_ID" ]]; then
  echo "gh: expected ids=$PARENT_ID got $ids" >&2
  exit 1
fi
echo "gh: list comments OK (id=$ids)"

# 3. Reply via gh api.
reply_json="$(gh api -X POST \
  "repos/octocat/hello/pulls/42/comments/${PARENT_ID}/replies" \
  -f body='fixed via gh CLI')"
reply_parent="$(echo "$reply_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["in_reply_to_id"])')"
if [[ "$reply_parent" != "$PARENT_ID" ]]; then
  echo "gh: reply parent mismatch (got $reply_parent, expected $PARENT_ID)" >&2
  exit 1
fi
echo "gh: posted reply OK (in_reply_to_id=$reply_parent)"

# 4. GraphQL resolveReviewThread (using gh api graphql).
thread_id="$(gh api graphql \
  -f query='query($owner:String!,$repo:String!,$pr:Int!){repository(owner:$owner,name:$repo){pullRequest(number:$pr){reviewThreads(first:100){nodes{id isResolved}}}}}' \
  -F owner=octocat -F repo=hello -F pr=42 \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"][0]["id"])')"
echo "gh: graphql found thread $thread_id"

resolved="$(gh api graphql \
  -f query='mutation($threadId:ID!){resolveReviewThread(input:{threadId:$threadId}){thread{id isResolved}}}' \
  -F threadId="$thread_id" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["resolveReviewThread"]["thread"]["isResolved"])')"
if [[ "$resolved" != "True" ]]; then
  echo "gh: thread not resolved (got $resolved)" >&2
  exit 1
fi
echo "gh: resolved thread OK"
