//! End-to-end tests using `reqwest` against a spawned server.
//! Covers every endpoint agent-reviews touches.

use std::sync::Arc;

use redpen_gh_fake::test_backend::TestBackend;
use redpen_gh_fake::SessionRef;
use serde_json::{json, Value};

async fn spawn() -> (String, TestBackend) {
    let backend = TestBackend::new();
    let (addr, _handle) = redpen_gh_fake::bind(Arc::new(backend.clone()), 0)
        .await
        .expect("bind");
    (format!("http://{}", addr), backend)
}

fn seed_pr(backend: &TestBackend) -> SessionRef {
    let session = SessionRef {
        id: "session-1".into(),
        owner: "octocat".into(),
        repo: "hello".into(),
        number: 42,
        title: "Add greeting".into(),
        body: "Adds a hello message.".into(),
        head_ref: "feature/hello".into(),
        head_sha: "deadbeef".into(),
        base_ref: "main".into(),
        base_sha: "cafebabe".into(),
        author_login: "octocat".into(),
        viewer_login: "reviewer".into(),
        html_url: "redpen://session/session-1".into(),
    };
    backend.add_session(session.clone());
    session
}

#[tokio::test]
async fn list_pulls_by_branch_returns_session() {
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);

    let url = format!(
        "{}/repos/{}/{}/pulls?head={}:{}&state=open",
        base, session.owner, session.repo, session.owner, session.head_ref
    );
    let resp: Vec<Value> = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0]["number"], 42);
    assert_eq!(resp[0]["head"]["ref"], "feature/hello");
    assert_eq!(resp[0]["base"]["ref"], "main");
}

#[tokio::test]
async fn list_pulls_unknown_branch_returns_empty() {
    let (base, backend) = spawn().await;
    seed_pr(&backend);

    let url = format!(
        "{}/repos/octocat/hello/pulls?head=octocat:nope&state=open",
        base
    );
    let resp: Vec<Value> = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert!(resp.is_empty());
}

#[tokio::test]
async fn list_review_comments_returns_seeded() {
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);
    let seeded = backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let url = format!("{}/repos/octocat/hello/pulls/42/comments", base);
    let resp: Vec<Value> = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0]["id"], seeded.database_id);
    assert_eq!(resp[0]["path"], "src/foo.rs");
    assert_eq!(resp[0]["line"], 10);
    assert_eq!(resp[0]["body"], "nit");
    assert_eq!(resp[0]["user"]["login"], "alice");
    // No `in_reply_to_id` for root comments.
    assert!(resp[0].get("in_reply_to_id").is_none());
}

#[tokio::test]
async fn issue_comments_and_reviews_are_empty() {
    let (base, backend) = spawn().await;
    seed_pr(&backend);

    for path in [
        "/repos/octocat/hello/issues/42/comments",
        "/repos/octocat/hello/pulls/42/reviews",
    ] {
        let url = format!("{}{}", base, path);
        let resp: Vec<Value> = reqwest::get(&url).await.unwrap().json().await.unwrap();
        assert!(resp.is_empty(), "{} should be empty", path);
    }
}

#[tokio::test]
async fn reply_to_comment_persists_and_round_trips() {
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);
    let parent = backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let client = reqwest::Client::new();
    let post = client
        .post(format!(
            "{}/repos/octocat/hello/pulls/42/comments/{}/replies",
            base, parent.database_id
        ))
        .bearer_auth("redpen-agent")
        .json(&json!({ "body": "fixed in abc123" }))
        .send()
        .await
        .unwrap();
    assert_eq!(post.status(), 201);
    let reply: Value = post.json().await.unwrap();
    assert_eq!(reply["body"], "fixed in abc123");
    assert_eq!(reply["in_reply_to_id"], parent.database_id);
    // Identity comes from the bearer token.
    assert_eq!(reply["user"]["login"], "redpen-agent");

    // Round-trip: fetching all comments now returns parent + reply.
    let listing: Vec<Value> = reqwest::get(format!(
        "{}/repos/octocat/hello/pulls/42/comments",
        base
    ))
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert_eq!(listing.len(), 2);
}

#[tokio::test]
async fn reply_to_unknown_comment_404s() {
    let (base, backend) = spawn().await;
    seed_pr(&backend);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/repos/octocat/hello/pulls/42/comments/999999/replies",
            base
        ))
        .bearer_auth("x")
        .json(&json!({ "body": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn graphql_find_thread_returns_seeded_thread() {
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);
    let seeded = backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let query = r#"
        query($owner:String!,$repo:String!,$pr:Int!) {
          repository(owner:$owner,name:$repo) {
            pullRequest(number:$pr) {
              reviewThreads(first:100) {
                pageInfo { hasNextPage endCursor }
                nodes { id isResolved comments(first:1) { nodes { databaseId } } }
              }
            }
          }
        }
    "#;
    let body = json!({
        "query": query,
        "variables": { "owner": "octocat", "repo": "hello", "pr": 42 }
    });

    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{}/graphql", base))
        .bearer_auth("x")
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let nodes = &resp["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"];
    assert_eq!(nodes.as_array().unwrap().len(), 1);
    assert_eq!(nodes[0]["isResolved"], false);
    assert_eq!(nodes[0]["comments"]["nodes"][0]["databaseId"], seeded.database_id);
}

#[tokio::test]
async fn api_v3_prefix_serves_same_routes() {
    // Sanity: the /api/v3/* mount used by gh's GHE-style clients works.
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);
    backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let url = format!("{}/api/v3/repos/octocat/hello/pulls/42/comments", base);
    let resp: Vec<Value> = reqwest::get(&url).await.unwrap().json().await.unwrap();
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0]["path"], "src/foo.rs");
}

#[tokio::test]
async fn create_review_comment_seeds_a_thread() {
    let (base, backend) = spawn().await;
    seed_pr(&backend);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/repos/octocat/hello/pulls/42/comments", base))
        .bearer_auth("codex-review")
        .json(&json!({
            "path": "src/foo.rs",
            "line": 25,
            "body": "this allocation is unbounded",
            "side": "RIGHT",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let comment: Value = resp.json().await.unwrap();
    assert_eq!(comment["path"], "src/foo.rs");
    assert_eq!(comment["line"], 25);
    assert_eq!(comment["user"]["login"], "codex-review");
    assert!(comment.get("in_reply_to_id").is_none());

    // Listing now returns the seeded comment.
    let listing: Vec<Value> = reqwest::get(format!(
        "{}/repos/octocat/hello/pulls/42/comments",
        base
    ))
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert!(listing.iter().any(|c| c["path"] == "src/foo.rs"
        && c["line"] == 25
        && c["user"]["login"] == "codex-review"));

    // And it can be replied to via the same id agent-reviews would use.
    let parent_id = comment["id"].as_i64().unwrap();
    let reply = client
        .post(format!(
            "{}/repos/octocat/hello/pulls/42/comments/{}/replies",
            base, parent_id
        ))
        .bearer_auth("agent-reviews")
        .json(&json!({ "body": "fixed in HEAD~" }))
        .send()
        .await
        .unwrap();
    assert_eq!(reply.status(), 201);
    let reply_json: Value = reply.json().await.unwrap();
    assert_eq!(reply_json["in_reply_to_id"], parent_id);
}

#[tokio::test]
async fn meta_endpoint_describes_active_sessions() {
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);
    backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let resp: Value = reqwest::get(format!("{}/redpen/meta", base))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["server"], "redpen-gh-fake");
    assert_eq!(resp["semantics"], "drafts-only");
    assert!(resp["supported"]["rest"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p.as_str().unwrap().contains("/comments/{id}/replies")));
    assert!(resp["supported"]["graphql"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p.as_str().unwrap().contains("resolveReviewThread")));

    let sessions = resp["active_sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["owner"], "octocat");
    assert_eq!(sessions[0]["repo"], "hello");
    assert_eq!(sessions[0]["number"], 42);
    assert_eq!(sessions[0]["branch"], "feature/hello");
}

#[tokio::test]
async fn graphql_resolve_thread_flips_state() {
    let (base, backend) = spawn().await;
    let session = seed_pr(&backend);
    let seeded = backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");
    let thread_id = seeded.thread_id.clone();

    let mutation = r#"
        mutation($threadId: ID!) {
          resolveReviewThread(input: { threadId: $threadId }) {
            thread { id isResolved }
          }
        }
    "#;
    let body = json!({ "query": mutation, "variables": { "threadId": thread_id } });

    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{}/graphql", base))
        .bearer_auth("x")
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["data"]["resolveReviewThread"]["thread"]["isResolved"], true);
    assert!(backend.threads_for(&session.id)[0].is_resolved);
}
