use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

// thiserror is the only extra dep; pull it in via Cargo.toml on first compile.
// Inlining a small impl here would also work, but thiserror keeps it tidy.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRef {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub body: String,
    pub head_ref: String,
    pub head_sha: String,
    pub base_ref: String,
    pub base_sha: String,
    pub author_login: String,
    pub viewer_login: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub database_id: i64,
    pub node_id: String,
    pub thread_id: String,
    pub path: String,
    pub line: u32,
    pub diff_hunk: String,
    pub body: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub in_reply_to_database_id: Option<i64>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadRef {
    pub node_id: String,
    pub is_resolved: bool,
    pub root_database_id: i64,
}

#[async_trait]
pub trait GhBackend: Send + Sync + 'static {
    async fn find_session_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Option<SessionRef>;

    async fn find_session_for_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Option<SessionRef>;

    /// All active GitHub sessions. Used by the `/redpen/meta` self-describing
    /// endpoint so agents/skills can discover which PRs they may operate on
    /// without needing the user to pass flags.
    async fn list_active_sessions(&self) -> Vec<SessionRef>;

    async fn list_review_comments(&self, session_id: &str) -> Vec<ReviewComment>;

    async fn list_threads(&self, session_id: &str) -> Vec<ThreadRef>;

    /// Create a new top-level review comment (a "thread root"). Used by
    /// reviewer bots to seed review findings against an active session
    /// without going to api.github.com. Mirrors GitHub's
    /// `POST /repos/{owner}/{repo}/pulls/{n}/comments` endpoint.
    async fn append_review_comment(
        &self,
        session_id: &str,
        agent: &str,
        path: &str,
        line: u32,
        body: &str,
    ) -> Result<ReviewComment, BackendError>;

    async fn append_reply(
        &self,
        session_id: &str,
        parent_database_id: i64,
        agent: &str,
        body: &str,
    ) -> Result<ReviewComment, BackendError>;

    async fn set_thread_resolved(
        &self,
        session_id: &str,
        thread_node_id: &str,
        resolved: bool,
    ) -> Result<ThreadRef, BackendError>;
}
