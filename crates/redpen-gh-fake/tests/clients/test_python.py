#!/usr/bin/env python3
"""Exercise the fake GitHub server end-to-end from Python.

Prefers PyGithub (real GitHub library) if installed; falls back to stdlib
urllib for environments without it. Reads BASE_URL and PARENT_ID from env.

Exit code 0 on full success; non-zero with a diagnostic on any failure.
"""
import json
import os
import sys
import urllib.request

BASE = os.environ["BASE_URL"]
PARENT_ID = int(os.environ["PARENT_ID"])
TOKEN = os.environ.get("GH_FAKE_TOKEN", "py-agent")


def _req(method, path, body=None):
    url = f"{BASE}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={
            "Authorization": f"Bearer {TOKEN}",
            "Accept": "application/vnd.github.v3+json",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req) as r:
        return r.status, json.loads(r.read() or b"null")


def run_with_pygithub():
    from github import Github, Auth  # type: ignore

    gh = Github(base_url=BASE, auth=Auth.Token(TOKEN))
    repo = gh.get_repo("octocat/hello")
    pr = repo.get_pull(42)

    # List review comments via the library.
    comments = list(pr.get_review_comments())
    if not comments:
        raise SystemExit("PyGithub: expected >=1 seeded comment")
    print(f"pygithub: listed {len(comments)} comment(s)")

    # PyGithub doesn't have a clean reply API in older versions; use the raw
    # API via the lib's requester to keep this test independent of version.
    headers, data = pr._requester.requestJsonAndCheck(
        "POST",
        f"{BASE}/repos/octocat/hello/pulls/42/comments/{PARENT_ID}/replies",
        input={"body": "fixed (via PyGithub)"},
    )
    print(f"pygithub: posted reply id={data.get('id')}")
    if not data.get("id"):
        raise SystemExit("PyGithub: reply missing id")
    return True


def run_with_stdlib():
    # 1. List PRs by branch.
    status, prs = _req("GET", "/repos/octocat/hello/pulls?head=octocat:feature/hello&state=open")
    assert status == 200, f"list pulls: {status}"
    assert len(prs) == 1, f"expected 1 PR, got {len(prs)}"
    assert prs[0]["number"] == 42, prs
    print(f"stdlib: list pulls OK (number={prs[0]['number']})")

    # 2. List review comments.
    status, comments = _req("GET", "/repos/octocat/hello/pulls/42/comments")
    assert status == 200
    assert len(comments) >= 1
    parent = comments[0]
    assert parent["id"] == PARENT_ID, f"expected id {PARENT_ID}, got {parent['id']}"
    print(f"stdlib: list comments OK ({len(comments)} comment(s))")

    # 3. Reply.
    status, reply = _req(
        "POST",
        f"/repos/octocat/hello/pulls/42/comments/{PARENT_ID}/replies",
        body={"body": "fixed via python stdlib"},
    )
    assert status == 201, f"reply: {status}"
    assert reply["in_reply_to_id"] == PARENT_ID
    assert reply["user"]["login"] == TOKEN
    print(f"stdlib: posted reply id={reply['id']}")

    # 4. GraphQL: find thread.
    query = """
        query($owner:String!,$repo:String!,$pr:Int!) {
          repository(owner:$owner,name:$repo) {
            pullRequest(number:$pr) {
              reviewThreads(first:100) {
                nodes { id isResolved comments(first:1) { nodes { databaseId } } }
              }
            }
          }
        }
    """
    status, gql = _req(
        "POST",
        "/graphql",
        body={"query": query, "variables": {"owner": "octocat", "repo": "hello", "pr": 42}},
    )
    assert status == 200
    nodes = gql["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"]
    assert len(nodes) == 1, nodes
    thread_id = nodes[0]["id"]
    print(f"stdlib: graphql found thread {thread_id}")

    # 5. GraphQL: resolve.
    mutation = """
        mutation($threadId: ID!) {
          resolveReviewThread(input:{threadId:$threadId}) { thread { id isResolved } }
        }
    """
    status, gql = _req(
        "POST",
        "/graphql",
        body={"query": mutation, "variables": {"threadId": thread_id}},
    )
    assert status == 200
    assert gql["data"]["resolveReviewThread"]["thread"]["isResolved"] is True
    print(f"stdlib: resolved thread {thread_id}")
    return True


def main():
    try:
        import github  # noqa: F401
    except ImportError:
        print("PyGithub not installed; using stdlib client.", file=sys.stderr)
        return run_with_stdlib()
    return run_with_pygithub() and run_with_stdlib()


if __name__ == "__main__":
    main()
