//! Minimal GraphQL endpoint covering the two operations agent-reviews uses:
//! find a review thread by comment id, and resolve a review thread.

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::ServerState;

#[derive(Deserialize)]
pub(crate) struct Body {
    query: String,
    #[serde(default)]
    variables: Value,
}

pub(crate) async fn handle(
    State(s): State<ServerState>,
    Json(body): Json<Body>,
) -> Result<Json<Value>, StatusCode> {
    if body.query.contains("resolveReviewThread") {
        return resolve_thread(s, body.variables).await;
    }
    if body.query.contains("reviewThreads") {
        return find_thread(s, body.variables).await;
    }
    // Unsupported query — return a GraphQL-shaped error so callers can see why.
    Ok(Json(json!({
        "errors": [{ "message": "redpen-gh-fake: unsupported GraphQL query" }],
    })))
}

async fn find_thread(s: ServerState, vars: Value) -> Result<Json<Value>, StatusCode> {
    let owner = vars.get("owner").and_then(Value::as_str).unwrap_or_default();
    let repo = vars.get("repo").and_then(Value::as_str).unwrap_or_default();
    let pr = vars
        .get("pr")
        .and_then(Value::as_u64)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;

    let Some(session) = s.backend.find_session_for_pr(owner, repo, pr).await else {
        // Mirror GitHub's shape: return null pullRequest, no errors.
        return Ok(Json(json!({
            "data": { "repository": { "pullRequest": null } }
        })));
    };

    let threads = s.backend.list_threads(&session.id).await;
    let nodes: Vec<Value> = threads
        .iter()
        .map(|t| {
            json!({
                "id": t.node_id,
                "isResolved": t.is_resolved,
                "comments": {
                    "nodes": [{ "databaseId": t.root_database_id }]
                }
            })
        })
        .collect();

    // Single-page response — agent-reviews stops paginating when hasNextPage=false.
    Ok(Json(json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "reviewThreads": {
                        "pageInfo": { "hasNextPage": false, "endCursor": null },
                        "nodes": nodes,
                    }
                }
            }
        }
    })))
}

async fn resolve_thread(s: ServerState, vars: Value) -> Result<Json<Value>, StatusCode> {
    let thread_id = vars
        .get("threadId")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;

    // We don't get owner/repo/pr in the mutation variables — the thread id
    // alone is the key. The backend has to be able to resolve threads
    // globally by their synthetic node id.
    //
    // Iterate sessions? For v1 the session is implicit: backend.set_thread_resolved
    // accepts the thread node id and finds the owning session itself.
    match s.backend.set_thread_resolved("", thread_id, true).await {
        Ok(updated) => Ok(Json(json!({
            "data": {
                "resolveReviewThread": {
                    "thread": { "id": updated.node_id, "isResolved": updated.is_resolved }
                }
            }
        }))),
        Err(crate::backend::BackendError::NotFound) => Ok(Json(json!({
            "errors": [{ "message": "thread not found" }]
        }))),
        Err(e) => Ok(Json(json!({
            "errors": [{ "message": e.to_string() }]
        }))),
    }
}
