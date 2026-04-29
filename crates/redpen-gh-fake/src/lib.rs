//! Localhost GitHub-API-compatible HTTP server.
//!
//! Speaks enough of GitHub's REST + GraphQL surface that agent tools written
//! against `api.github.com` (e.g. `pbakaus/agent-reviews`) can run against
//! a Redpen session instead.
//!
//! The server is backed by a pluggable `GhBackend` trait so it can be
//! integration-tested without Tauri, SQLite, or sidecar I/O.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

mod backend;
mod graphql;
mod meta;
mod rest;
mod shapes;

pub use backend::{BackendError, GhBackend, ReviewComment, SessionRef, ThreadRef};
pub mod test_backend;

#[derive(Clone)]
pub(crate) struct ServerState {
    pub backend: Arc<dyn GhBackend>,
}

/// Build the router.
///
/// Routes are mounted under both `/` (for clients hitting raw `api.github.com`
/// paths, e.g. `agent-reviews` with `GITHUB_API_URL=http://host:port`) and
/// `/api/v3` (for GHE-style clients like `gh` with `GH_HOST=host:port`).
pub fn router(backend: Arc<dyn GhBackend>) -> Router {
    let state = ServerState { backend };

    let routes = Router::new()
        .route("/repos/{owner}/{repo}/pulls", get(rest::list_pulls))
        .route(
            "/repos/{owner}/{repo}/pulls/{number}/comments",
            get(rest::list_review_comments).post(rest::create_review_comment),
        )
        .route(
            "/repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies",
            post(rest::reply_to_comment),
        )
        .route(
            "/repos/{owner}/{repo}/issues/{number}/comments",
            get(rest::list_issue_comments).post(rest::create_issue_comment),
        )
        .route(
            "/repos/{owner}/{repo}/pulls/{number}/reviews",
            get(rest::list_reviews),
        )
        .route("/graphql", post(graphql::handle));

    Router::new()
        .merge(routes.clone())
        .nest("/api/v3", routes)
        // Redpen-specific self-description — NOT mounted under /api/v3 because
        // it's not a GitHub API surface. Agents/skills probe this to discover
        // active sessions and confirm they're sandboxed.
        .route("/redpen/meta", get(meta::handle))
        .with_state(state)
}

/// Convenience: bind on `127.0.0.1:port` (0 for ephemeral) and return the
/// bound address + a future that drives the server.
pub async fn bind(
    backend: Arc<dyn GhBackend>,
    port: u16,
) -> std::io::Result<(std::net::SocketAddr, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let addr = listener.local_addr()?;
    let app = router(backend);
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((addr, handle))
}
