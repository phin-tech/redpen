// Exercise the fake GitHub server end-to-end from Node.
// Prefers @octokit/rest (real GitHub library) if installed in this dir;
// always also runs a built-in fetch round-trip for the GraphQL bits.
//
// Reads BASE_URL and PARENT_ID from env. Exits 0 on success.

"use strict";

const path = require("path");

const BASE = process.env.BASE_URL;
const PARENT_ID = Number(process.env.PARENT_ID);
const TOKEN = process.env.GH_FAKE_TOKEN || "node-agent";

if (!BASE || !PARENT_ID) {
  console.error("BASE_URL and PARENT_ID must be set");
  process.exit(2);
}

async function runWithOctokit() {
  let Octokit;
  // @octokit/rest v21+ ships ESM-only — use dynamic import from this CJS file.
  try {
    const mod = await import(
      path.join(__dirname, "node_modules", "@octokit", "rest", "dist-src", "index.js")
    );
    Octokit = mod.Octokit;
  } catch (e) {
    console.error("@octokit/rest not loadable in tests/clients; skipping octokit path:", e.message);
    return false;
  }

  const octokit = new Octokit({ auth: TOKEN, baseUrl: BASE });

  const { data: prs } = await octokit.pulls.list({
    owner: "octocat",
    repo: "hello",
    head: "octocat:feature/hello",
    state: "open",
  });
  if (prs.length !== 1 || prs[0].number !== 42) {
    throw new Error(`octokit: expected PR 42, got ${JSON.stringify(prs)}`);
  }
  console.log("octokit: list pulls OK");

  const { data: comments } = await octokit.pulls.listReviewComments({
    owner: "octocat",
    repo: "hello",
    pull_number: 42,
  });
  if (comments.length < 1 || comments[0].id !== PARENT_ID) {
    throw new Error(`octokit: comment id mismatch: ${JSON.stringify(comments)}`);
  }
  console.log(`octokit: list comments OK (id=${comments[0].id})`);

  const { data: reply } = await octokit.pulls.createReplyForReviewComment({
    owner: "octocat",
    repo: "hello",
    pull_number: 42,
    comment_id: PARENT_ID,
    body: "fixed (via octokit)",
  });
  if (reply.in_reply_to_id !== PARENT_ID) {
    throw new Error(`octokit: in_reply_to_id mismatch: ${JSON.stringify(reply)}`);
  }
  console.log(`octokit: posted reply id=${reply.id}`);
  return true;
}

async function runWithFetch() {
  const headers = {
    Authorization: `Bearer ${TOKEN}`,
    "Content-Type": "application/json",
    Accept: "application/vnd.github.v3+json",
  };

  // GraphQL: find thread.
  const findQuery = `
    query($owner:String!,$repo:String!,$pr:Int!) {
      repository(owner:$owner,name:$repo) {
        pullRequest(number:$pr) {
          reviewThreads(first:100) {
            nodes { id isResolved comments(first:1) { nodes { databaseId } } }
          }
        }
      }
    }`;
  let r = await fetch(`${BASE}/graphql`, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query: findQuery,
      variables: { owner: "octocat", repo: "hello", pr: 42 },
    }),
  });
  if (!r.ok) throw new Error(`graphql find: ${r.status}`);
  const findData = await r.json();
  const nodes = findData.data.repository.pullRequest.reviewThreads.nodes;
  if (nodes.length !== 1) {
    throw new Error(`graphql: expected 1 thread, got ${nodes.length}`);
  }
  const threadId = nodes[0].id;
  console.log(`fetch: graphql found thread ${threadId}`);

  // GraphQL: resolve.
  const resolveMutation = `
    mutation($threadId: ID!) {
      resolveReviewThread(input:{threadId:$threadId}) { thread { id isResolved } }
    }`;
  r = await fetch(`${BASE}/graphql`, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query: resolveMutation,
      variables: { threadId },
    }),
  });
  if (!r.ok) throw new Error(`graphql resolve: ${r.status}`);
  const resolveData = await r.json();
  if (!resolveData.data.resolveReviewThread.thread.isResolved) {
    throw new Error(`graphql: thread not marked resolved`);
  }
  console.log(`fetch: resolved thread ${threadId}`);
}

(async () => {
  await runWithOctokit();
  await runWithFetch();
})().catch((e) => {
  console.error("FAIL:", e.message);
  process.exit(1);
});
