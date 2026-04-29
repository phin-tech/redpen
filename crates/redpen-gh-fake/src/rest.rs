//! REST handlers covering the surface used by `pbakaus/agent-reviews`.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::shapes::{pr_json, review_comment_json};
use crate::ServerState;

#[derive(Deserialize)]
pub(crate) struct PullsQuery {
    head: Option<String>,
    state: Option<String>,
}

/// `GET /repos/{owner}/{repo}/pulls?head={owner}:{branch}&state=open`
pub(crate) async fn list_pulls(
    State(s): State<ServerState>,
    Path((owner, repo)): Path<(String, String)>,
    Query(q): Query<PullsQuery>,
) -> Json<Vec<Value>> {
    if q.state.as_deref().unwrap_or("open") != "open" {
        return Json(vec![]);
    }
    let Some(head) = q.head else {
        return Json(vec![]);
    };
    // head is "owner:branch" — strip the owner prefix.
    let branch = head.split_once(':').map(|(_, b)| b).unwrap_or(&head);
    match s.backend.find_session_for_branch(&owner, &repo, branch).await {
        Some(session) => Json(vec![pr_json(&session)]),
        None => Json(vec![]),
    }
}

/// `GET /repos/{owner}/{repo}/pulls/{number}/comments`
pub(crate) async fn list_review_comments(
    State(s): State<ServerState>,
    Path((owner, repo, number)): Path<(String, String, u64)>,
) -> Result<Json<Vec<Value>>, StatusCode> {
    let session = s
        .backend
        .find_session_for_pr(&owner, &repo, number)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let comments = s.backend.list_review_comments(&session.id).await;
    Ok(Json(comments.iter().map(review_comment_json).collect()))
}

/// `GET /repos/{owner}/{repo}/issues/{number}/comments`
///
/// Returns `[]` v1 — Redpen has no issue-level comment concept.
pub(crate) async fn list_issue_comments(
    State(_s): State<ServerState>,
    Path((_owner, _repo, _number)): Path<(String, String, u64)>,
) -> Json<Vec<Value>> {
    Json(vec![])
}

/// `POST /repos/{owner}/{repo}/issues/{number}/comments`
///
/// Defensive: agent-reviews falls back here if a threaded reply 404s.
/// We never 404, so this is a no-op echo.
pub(crate) async fn create_issue_comment(
    State(_s): State<ServerState>,
    Path((_owner, _repo, _number)): Path<(String, String, u64)>,
    headers: HeaderMap,
    Json(body): Json<HashMap<String, Value>>,
) -> Json<Value> {
    let agent = bearer_identity(&headers);
    let body_text = body
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Json(json!({
        "id": chrono::Utc::now().timestamp_micros(),
        "user": { "login": agent },
        "body": body_text,
        "html_url": "redpen://issue-comment-noop",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    }))
}

/// `GET /repos/{owner}/{repo}/pulls/{number}/reviews`
///
/// Returns `[]` v1 — Redpen import doesn't currently capture review-summary
/// bodies (only individual review threads).
pub(crate) async fn list_reviews(
    State(_s): State<ServerState>,
    Path((_owner, _repo, _number)): Path<(String, String, u64)>,
) -> Json<Vec<Value>> {
    Json(vec![])
}

/// `POST /repos/{owner}/{repo}/pulls/{number}/comments`
///
/// Create a new top-level review comment. Used by reviewer bots to seed
/// findings into a Redpen session without going to api.github.com.
pub(crate) async fn create_review_comment(
    State(s): State<ServerState>,
    Path((owner, repo, number)): Path<(String, String, u64)>,
    headers: HeaderMap,
    Json(body): Json<HashMap<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let session = s
        .backend
        .find_session_for_pr(&owner, &repo, number)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let agent = bearer_identity(&headers);
    let path = body
        .get("path")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    let line = body
        .get("line")
        .and_then(Value::as_u64)
        .or_else(|| body.get("original_line").and_then(Value::as_u64))
        .unwrap_or(1) as u32;
    let text = body
        .get("body")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    match s
        .backend
        .append_review_comment(&session.id, &agent, path, line, text)
        .await
    {
        Ok(c) => Ok((StatusCode::CREATED, Json(review_comment_json(&c)))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// `POST /repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies`
pub(crate) async fn reply_to_comment(
    State(s): State<ServerState>,
    Path((owner, repo, number, comment_id)): Path<(String, String, u64, i64)>,
    headers: HeaderMap,
    Json(body): Json<HashMap<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let session = s
        .backend
        .find_session_for_pr(&owner, &repo, number)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let agent = bearer_identity(&headers);
    let text = body
        .get("body")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    match s
        .backend
        .append_reply(&session.id, comment_id, &agent, text)
        .await
    {
        Ok(reply) => Ok((StatusCode::CREATED, Json(review_comment_json(&reply)))),
        Err(crate::backend::BackendError::NotFound) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Identity extracted from `Authorization: Bearer <token>`. The token *is*
/// the identity for v1 — anything goes; loopback-only mitigates spoofing.
pub(crate) fn bearer_identity(headers: &HeaderMap) -> String {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("token "))
        })
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "anonymous".to_string())
}
