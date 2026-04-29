//! In-memory `GhBackend` for tests and examples.
//!
//! Not intended for production. Sessions, comments, and threads are stored
//! in a single mutex-guarded struct.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;

use crate::backend::{BackendError, GhBackend, ReviewComment, SessionRef, ThreadRef};

#[derive(Default)]
struct Inner {
    sessions: Vec<SessionRef>,
    comments: Vec<(String, ReviewComment)>, // (session_id, comment)
    threads: Vec<(String, ThreadRef)>,      // (session_id, thread)
    next_database_id: i64,
}

#[derive(Clone, Default)]
pub struct TestBackend {
    inner: Arc<Mutex<Inner>>,
}

impl TestBackend {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                next_database_id: 1_000_000,
                ..Default::default()
            })),
        }
    }

    pub fn add_session(&self, session: SessionRef) {
        self.inner.lock().unwrap().sessions.push(session);
    }

    /// Seed a root review comment + corresponding thread. Returns the comment.
    pub fn seed_comment(
        &self,
        session_id: &str,
        path: &str,
        line: u32,
        body: &str,
        author: &str,
    ) -> ReviewComment {
        let mut inner = self.inner.lock().unwrap();
        let database_id = inner.next_database_id;
        inner.next_database_id += 1;
        let node_id = format!("PRRC_{}", database_id);
        let thread_node_id = format!("PRRT_{}", database_id);
        let now = Utc::now();
        let comment = ReviewComment {
            database_id,
            node_id: node_id.clone(),
            thread_id: thread_node_id.clone(),
            path: path.to_string(),
            line,
            diff_hunk: format!("@@ -{0},1 +{0},1 @@", line),
            body: body.to_string(),
            author: author.to_string(),
            created_at: now,
            updated_at: now,
            in_reply_to_database_id: None,
            html_url: format!("redpen://comment/{}", database_id),
        };
        inner.comments.push((session_id.to_string(), comment.clone()));
        inner.threads.push((
            session_id.to_string(),
            ThreadRef {
                node_id: thread_node_id,
                is_resolved: false,
                root_database_id: database_id,
            },
        ));
        comment
    }

    /// Snapshot all comments for a session — used by tests to assert state.
    pub fn comments_for(&self, session_id: &str) -> Vec<ReviewComment> {
        self.inner
            .lock()
            .unwrap()
            .comments
            .iter()
            .filter(|(sid, _)| sid == session_id)
            .map(|(_, c)| c.clone())
            .collect()
    }

    pub fn threads_for(&self, session_id: &str) -> Vec<ThreadRef> {
        self.inner
            .lock()
            .unwrap()
            .threads
            .iter()
            .filter(|(sid, _)| sid == session_id)
            .map(|(_, t)| t.clone())
            .collect()
    }
}

#[async_trait]
impl GhBackend for TestBackend {
    async fn find_session_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Option<SessionRef> {
        self.inner
            .lock()
            .unwrap()
            .sessions
            .iter()
            .find(|s| s.owner == owner && s.repo == repo && s.head_ref == branch)
            .cloned()
    }

    async fn find_session_for_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Option<SessionRef> {
        self.inner
            .lock()
            .unwrap()
            .sessions
            .iter()
            .find(|s| s.owner == owner && s.repo == repo && s.number == number)
            .cloned()
    }

    async fn list_active_sessions(&self) -> Vec<SessionRef> {
        self.inner.lock().unwrap().sessions.clone()
    }

    async fn list_review_comments(&self, session_id: &str) -> Vec<ReviewComment> {
        self.comments_for(session_id)
    }

    async fn list_threads(&self, session_id: &str) -> Vec<ThreadRef> {
        self.threads_for(session_id)
    }

    async fn append_review_comment(
        &self,
        session_id: &str,
        agent: &str,
        path: &str,
        line: u32,
        body: &str,
    ) -> Result<ReviewComment, BackendError> {
        let mut inner = self.inner.lock().unwrap();
        // Verify the session exists.
        if !inner.sessions.iter().any(|s| s.id == session_id) {
            return Err(BackendError::NotFound);
        }
        let database_id = inner.next_database_id;
        inner.next_database_id += 1;
        let node_id = format!("PRRC_{}", database_id);
        let thread_node_id = format!("PRRT_{}", database_id);
        let now = Utc::now();
        let comment = ReviewComment {
            database_id,
            node_id: node_id.clone(),
            thread_id: thread_node_id.clone(),
            path: path.to_string(),
            line,
            diff_hunk: format!("@@ -{0},1 +{0},1 @@", line),
            body: body.to_string(),
            author: agent.to_string(),
            created_at: now,
            updated_at: now,
            in_reply_to_database_id: None,
            html_url: format!("redpen://comment/{}", database_id),
        };
        inner.comments.push((session_id.to_string(), comment.clone()));
        inner.threads.push((
            session_id.to_string(),
            ThreadRef {
                node_id: thread_node_id,
                is_resolved: false,
                root_database_id: database_id,
            },
        ));
        Ok(comment)
    }

    async fn append_reply(
        &self,
        session_id: &str,
        parent_database_id: i64,
        agent: &str,
        body: &str,
    ) -> Result<ReviewComment, BackendError> {
        let mut inner = self.inner.lock().unwrap();
        // Verify parent exists in this session, copy its path/line/thread_id.
        let parent = inner
            .comments
            .iter()
            .find(|(sid, c)| sid == session_id && c.database_id == parent_database_id)
            .map(|(_, c)| c.clone())
            .ok_or(BackendError::NotFound)?;
        let database_id = inner.next_database_id;
        inner.next_database_id += 1;
        let now = Utc::now();
        let reply = ReviewComment {
            database_id,
            node_id: format!("PRRC_{}", database_id),
            thread_id: parent.thread_id.clone(),
            path: parent.path.clone(),
            line: parent.line,
            diff_hunk: parent.diff_hunk.clone(),
            body: body.to_string(),
            author: agent.to_string(),
            created_at: now,
            updated_at: now,
            in_reply_to_database_id: Some(parent_database_id),
            html_url: format!("redpen://comment/{}", database_id),
        };
        inner.comments.push((session_id.to_string(), reply.clone()));
        Ok(reply)
    }

    async fn set_thread_resolved(
        &self,
        _session_id: &str,
        thread_node_id: &str,
        resolved: bool,
    ) -> Result<ThreadRef, BackendError> {
        let mut inner = self.inner.lock().unwrap();
        let thread = inner
            .threads
            .iter_mut()
            .find(|(_, t)| t.node_id == thread_node_id)
            .map(|(_, t)| {
                t.is_resolved = resolved;
                t.clone()
            })
            .ok_or(BackendError::NotFound)?;
        Ok(thread)
    }
}
