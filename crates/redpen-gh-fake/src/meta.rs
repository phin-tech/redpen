//! Self-describing endpoint at `/redpen/meta`.
//!
//! Lets agents/skills probe whether they're talking to a real GitHub or to
//! Redpen, what semantics apply (drafts vs publish), and which sessions are
//! available — without the user having to pass any flags.

use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::{json, Value};

use crate::ServerState;

#[derive(Serialize)]
struct MetaResponse {
    server: &'static str,
    version: &'static str,
    /// `drafts-only` — writes (replies, thread resolution) land in local
    /// sidecars and do not propagate to api.github.com until the human
    /// hits "Submit review" in the Redpen UI.
    semantics: &'static str,
    /// REST and GraphQL paths that are wired up. Anything else 404s.
    supported: SupportedSurface,
    /// PRs the agent may operate on. Empty if nothing is loaded in Redpen.
    active_sessions: Vec<Value>,
}

#[derive(Serialize)]
struct SupportedSurface {
    rest: &'static [&'static str],
    graphql: &'static [&'static str],
}

const REST_PATHS: &[&str] = &[
    "GET /repos/{owner}/{repo}/pulls",
    "GET /repos/{owner}/{repo}/pulls/{n}/comments",
    "POST /repos/{owner}/{repo}/pulls/{n}/comments/{id}/replies",
    "GET/POST /repos/{owner}/{repo}/issues/{n}/comments",
    "GET /repos/{owner}/{repo}/pulls/{n}/reviews",
];

const GRAPHQL_OPS: &[&str] = &[
    "query reviewThreads",
    "mutation resolveReviewThread",
];

pub(crate) async fn handle(State(s): State<ServerState>) -> Json<Value> {
    let active = s.backend.list_active_sessions().await;
    let active_json: Vec<Value> = active
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "owner": s.owner,
                "repo": s.repo,
                "number": s.number,
                "branch": s.head_ref,
                "title": s.title,
                "html_url": s.html_url,
            })
        })
        .collect();

    let resp = MetaResponse {
        server: "redpen-gh-fake",
        version: env!("CARGO_PKG_VERSION"),
        semantics: "drafts-only",
        supported: SupportedSurface {
            rest: REST_PATHS,
            graphql: GRAPHQL_OPS,
        },
        active_sessions: active_json,
    };
    Json(serde_json::to_value(resp).unwrap())
}
