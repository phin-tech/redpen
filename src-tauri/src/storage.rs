use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use std::fs;
use std::path::{Path, PathBuf};

pub const APP_HOME_DIRECTORY: &str = ".config/redpen";
pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const STATE_DB_FILE_NAME: &str = "state.db";

const STALE_SESSION_AFTER_HOURS: i64 = 72;
const CLEANUP_AFTER_DAYS: i64 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewSessionKind {
    LocalReview,
    GitHubPr,
}

impl ReviewSessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalReview => "local_review",
            Self::GitHubPr => "github_pr",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "local_review" => Some(Self::LocalReview),
            "github_pr" => Some(Self::GitHubPr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewSessionStatus {
    Active,
    Completed,
    Cancelled,
    TimedOut,
    Stale,
    Archived,
}

impl ReviewSessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Stale => "stale",
            Self::Archived => "archived",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            "timed_out" => Some(Self::TimedOut),
            "stale" => Some(Self::Stale),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }

    pub fn can_transition_to(self, target: Self) -> bool {
        match (self, target) {
            // Active sessions can move to any terminal state or stale
            (Self::Active, Self::Completed | Self::Cancelled | Self::TimedOut | Self::Stale) => {
                true
            }
            // Stale sessions can be completed, cancelled, or archived
            (Self::Stale, Self::Completed | Self::Cancelled | Self::Archived) => true,
            // Terminal states can only move to archived
            (Self::Completed | Self::Cancelled | Self::TimedOut, Self::Archived) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoredReviewSession {
    pub id: String,
    pub kind: ReviewSessionKind,
    pub status: ReviewSessionStatus,
    pub repo: Option<String>,
    pub pr_number: Option<u32>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub url: Option<String>,
    pub local_repo_path: Option<String>,
    pub worktree_path: Option<String>,
    pub primary_file_path: Option<String>,
    pub project_root: Option<String>,
    pub author_login: Option<String>,
    pub viewer_login: Option<String>,
    pub base_ref: Option<String>,
    pub base_sha: Option<String>,
    pub head_ref: Option<String>,
    pub head_sha: Option<String>,
    pub changed_files: Vec<String>,
    pub verdict: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub file_count: usize,
    pub agent_status: Option<String>,
    pub agent_task: Option<String>,
    pub agent_pid: Option<i64>,
    pub last_heartbeat: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IndexedSessionFile {
    pub file_path: String,
    pub relative_path: Option<String>,
    pub annotation_count: usize,
    pub pending_count: usize,
    pub resolved_count: usize,
}

pub struct CleanupResult {
    pub removed_sessions: usize,
}

pub type ReviewSessionStatusRow = (ReviewSessionStatus, Option<String>, Option<String>);

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Chrono(#[from] chrono::ParseError),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
}

pub struct StateDb {
    path: PathBuf,
}

impl Clone for StateDb {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
        }
    }
}

impl StateDb {
    pub fn new() -> Result<Self, StorageError> {
        let path = state_db_path()?;
        let db = Self { path };
        db.initialize()?;
        Ok(db)
    }

    pub fn upsert_review_session(&self, session: &StoredReviewSession) -> Result<(), StorageError> {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        let now = Utc::now().to_rfc3339();
        let repo_id = if let (Some(repo), Some(local_repo_path)) =
            (&session.repo, &session.local_repo_path)
        {
            Some(upsert_repo_tx(&tx, repo, local_repo_path, &now)?)
        } else {
            None
        };

        tx.execute(
            r#"
            INSERT INTO review_sessions (
                id, kind, status, repo_id, repo, pr_number, title, body, url,
                local_repo_path, worktree_path, primary_file_path, project_root,
                author_login, viewer_login, base_ref, base_sha, head_ref, head_sha,
                changed_files_json, verdict, created_at, updated_at, completed_at, last_accessed_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
                ?20, ?21, ?22, ?23, ?24, ?25
            )
            ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                status = excluded.status,
                repo_id = excluded.repo_id,
                repo = excluded.repo,
                pr_number = excluded.pr_number,
                title = excluded.title,
                body = excluded.body,
                url = excluded.url,
                local_repo_path = excluded.local_repo_path,
                worktree_path = excluded.worktree_path,
                primary_file_path = excluded.primary_file_path,
                project_root = excluded.project_root,
                author_login = excluded.author_login,
                viewer_login = excluded.viewer_login,
                base_ref = excluded.base_ref,
                base_sha = excluded.base_sha,
                head_ref = excluded.head_ref,
                head_sha = excluded.head_sha,
                changed_files_json = excluded.changed_files_json,
                verdict = excluded.verdict,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at,
                last_accessed_at = excluded.last_accessed_at,
                agent_status = COALESCE(excluded.agent_status, agent_status),
                agent_task = COALESCE(excluded.agent_task, agent_task),
                agent_pid = COALESCE(excluded.agent_pid, agent_pid),
                last_heartbeat = COALESCE(excluded.last_heartbeat, last_heartbeat)
            "#,
            params![
                session.id,
                session.kind.as_str(),
                session.status.as_str(),
                repo_id,
                session.repo,
                session.pr_number.map(i64::from),
                session.title,
                session.body,
                session.url,
                session.local_repo_path,
                session.worktree_path,
                session.primary_file_path,
                session.project_root,
                session.author_login,
                session.viewer_login,
                session.base_ref,
                session.base_sha,
                session.head_ref,
                session.head_sha,
                serde_json::to_string(&session.changed_files)?,
                session.verdict,
                session.created_at,
                session.updated_at,
                session.completed_at,
                now,
            ],
        )?;

        record_activity_tx(
            &tx,
            Some(&session.id),
            repo_id,
            "session_upserted",
            session.title.as_deref(),
            session.primary_file_path.as_deref(),
            None,
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn complete_review_session(
        &self,
        session_id: &str,
        verdict: &str,
    ) -> Result<Option<StoredReviewSession>, StorageError> {
        let current = self.review_session_status(session_id)?;
        match current {
            None => return Ok(None),
            Some((status, _, _)) => {
                if !status.can_transition_to(ReviewSessionStatus::Completed) {
                    return Err(StorageError::Message(format!(
                        "cannot complete session in {} state",
                        status.as_str(),
                    )));
                }
            }
        }

        let now = Utc::now().to_rfc3339();
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            r#"
            UPDATE review_sessions
            SET status = ?2,
                verdict = ?3,
                updated_at = ?4,
                completed_at = ?4,
                last_accessed_at = ?4
            WHERE id = ?1
            "#,
            params![
                session_id,
                ReviewSessionStatus::Completed.as_str(),
                verdict,
                now
            ],
        )?;

        record_activity_tx(
            &tx,
            Some(session_id),
            None,
            "session_completed",
            Some(verdict),
            None,
            None,
        )?;

        tx.commit()?;
        self.get_review_session(session_id)
    }

    pub fn cancel_review_session(
        &self,
        session_id: &str,
    ) -> Result<Option<StoredReviewSession>, StorageError> {
        self.transition_session(
            session_id,
            ReviewSessionStatus::Cancelled,
            Some("cancelled"),
        )
    }

    pub fn timeout_review_session(
        &self,
        session_id: &str,
    ) -> Result<Option<StoredReviewSession>, StorageError> {
        self.transition_session(session_id, ReviewSessionStatus::TimedOut, Some("timed_out"))
    }

    fn transition_session(
        &self,
        session_id: &str,
        target_status: ReviewSessionStatus,
        verdict: Option<&str>,
    ) -> Result<Option<StoredReviewSession>, StorageError> {
        let current = self.review_session_status(session_id)?;
        let current_status = match current {
            Some((status, _, _)) => status,
            None => return Ok(None),
        };
        if !current_status.can_transition_to(target_status) {
            return Err(StorageError::Message(format!(
                "cannot transition session from {} to {}",
                current_status.as_str(),
                target_status.as_str(),
            )));
        }

        let now = Utc::now().to_rfc3339();
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            r#"
            UPDATE review_sessions
            SET status = ?2, verdict = COALESCE(?3, verdict),
                updated_at = ?4, completed_at = ?4, last_accessed_at = ?4
            WHERE id = ?1
            "#,
            params![session_id, target_status.as_str(), verdict, now],
        )?;
        let activity_kind = match target_status {
            ReviewSessionStatus::Cancelled => "session_cancelled",
            ReviewSessionStatus::TimedOut => "session_timed_out",
            _ => "session_transitioned",
        };
        record_activity_tx(
            &tx,
            Some(session_id),
            None,
            activity_kind,
            verdict,
            None,
            None,
        )?;
        tx.commit()?;
        self.get_review_session(session_id)
    }

    pub fn replace_session_files(
        &self,
        session_id: &str,
        files: &[IndexedSessionFile],
    ) -> Result<(), StorageError> {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM session_files WHERE session_id = ?1",
            params![session_id],
        )?;
        for file in files {
            insert_session_file_tx(&tx, session_id, file)?;
        }
        tx.execute(
            "UPDATE review_sessions SET updated_at = ?2, last_accessed_at = ?2 WHERE id = ?1",
            params![session_id, Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn upsert_session_file(
        &self,
        session_id: &str,
        file: &IndexedSessionFile,
    ) -> Result<(), StorageError> {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        insert_session_file_tx(&tx, session_id, file)?;
        tx.execute(
            "UPDATE review_sessions SET updated_at = ?2, last_accessed_at = ?2 WHERE id = ?1",
            params![session_id, Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_session_file(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> Result<(), StorageError> {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM session_files WHERE session_id = ?1 AND file_path = ?2",
            params![session_id, file_path],
        )?;
        tx.execute(
            "UPDATE review_sessions SET updated_at = ?2, last_accessed_at = ?2 WHERE id = ?1",
            params![session_id, Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_review_session(
        &self,
        session_id: &str,
    ) -> Result<Option<StoredReviewSession>, StorageError> {
        let conn = self.connect()?;
        conn.query_row(SESSION_SELECT_SQL, params![session_id], map_session_row)
            .optional()
            .map_err(StorageError::from)
    }

    pub fn list_session_files(
        &self,
        session_id: &str,
    ) -> Result<Vec<IndexedSessionFile>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT file_path, relative_path, annotation_count, pending_count, resolved_count
            FROM session_files
            WHERE session_id = ?1
            ORDER BY file_path ASC
            "#,
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(IndexedSessionFile {
                file_path: row.get(0)?,
                relative_path: row.get(1)?,
                annotation_count: row.get::<_, i64>(2)? as usize,
                pending_count: row.get::<_, i64>(3)? as usize,
                resolved_count: row.get::<_, i64>(4)? as usize,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub fn find_github_session_for_file(
        &self,
        file_path: &Path,
    ) -> Result<Option<StoredReviewSession>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                s.id, s.kind, s.status, s.repo, s.pr_number, s.title, s.body, s.url,
                s.local_repo_path, s.worktree_path, s.primary_file_path, s.project_root,
                s.author_login, s.viewer_login, s.base_ref, s.base_sha, s.head_ref, s.head_sha,
                s.changed_files_json, s.verdict, s.created_at, s.updated_at, s.completed_at,
                COALESCE(COUNT(f.id), 0) AS file_count,
                s.agent_status, s.agent_task, s.agent_pid, s.last_heartbeat
            FROM review_sessions s
            LEFT JOIN session_files f ON f.session_id = s.id
            WHERE s.kind = ?1 AND s.worktree_path IS NOT NULL AND s.status != ?2
            GROUP BY s.id
            ORDER BY s.updated_at DESC
            "#,
        )?;
        let sessions = stmt
            .query_map(
                params![
                    ReviewSessionKind::GitHubPr.as_str(),
                    ReviewSessionStatus::Archived.as_str()
                ],
                map_session_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions
            .into_iter()
            .filter(|session| {
                session
                    .worktree_path
                    .as_ref()
                    .is_some_and(|worktree| file_path.starts_with(Path::new(worktree)))
            })
            .max_by_key(|session| {
                session
                    .worktree_path
                    .as_ref()
                    .map(|path| path.len())
                    .unwrap_or(0)
            }))
    }

    pub fn review_session_status(
        &self,
        session_id: &str,
    ) -> Result<Option<ReviewSessionStatusRow>, StorageError> {
        let conn = self.connect()?;
        conn.query_row(
            "SELECT status, verdict, primary_file_path FROM review_sessions WHERE id = ?1",
            params![session_id],
            |row| {
                let status: String = row.get(0)?;
                Ok((
                    ReviewSessionStatus::parse(&status).ok_or_else(|| {
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "status".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            },
        )
        .optional()
        .map_err(StorageError::from)
    }

    pub fn list_recent_sessions(
        &self,
        kind: Option<ReviewSessionKind>,
        status: Option<ReviewSessionStatus>,
        limit: usize,
    ) -> Result<Vec<StoredReviewSession>, StorageError> {
        self.mark_stale_sessions()?;
        let conn = self.connect()?;
        let mut sessions = Vec::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT
                s.id, s.kind, s.status, s.repo, s.pr_number, s.title, s.body, s.url,
                s.local_repo_path, s.worktree_path, s.primary_file_path, s.project_root,
                s.author_login, s.viewer_login, s.base_ref, s.base_sha, s.head_ref, s.head_sha,
                s.changed_files_json, s.verdict, s.created_at, s.updated_at, s.completed_at,
                COALESCE(COUNT(f.id), 0) AS file_count,
                s.agent_status, s.agent_task, s.agent_pid, s.last_heartbeat
            FROM review_sessions s
            LEFT JOIN session_files f ON f.session_id = s.id
            GROUP BY s.id
            ORDER BY s.last_accessed_at DESC, s.updated_at DESC
            "#,
        )?;
        let rows = stmt.query_map([], map_session_row)?;
        for row in rows {
            let session = row?;
            if kind.is_some_and(|expected| session.kind != expected) {
                continue;
            }
            if status.is_some_and(|expected| session.status != expected) {
                continue;
            }
            sessions.push(session);
            if sessions.len() >= limit {
                break;
            }
        }
        Ok(sessions)
    }

    pub fn cleanup_stale_sessions(&self) -> Result<CleanupResult, StorageError> {
        self.mark_stale_sessions()?;
        let cutoff = (Utc::now() - Duration::days(CLEANUP_AFTER_DAYS)).to_rfc3339();
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id FROM review_sessions
            WHERE status IN (?1, ?2, ?3, ?4) AND updated_at <= ?5
            "#,
        )?;
        let session_ids = stmt
            .query_map(
                params![
                    ReviewSessionStatus::Completed.as_str(),
                    ReviewSessionStatus::Stale.as_str(),
                    ReviewSessionStatus::Cancelled.as_str(),
                    ReviewSessionStatus::TimedOut.as_str(),
                    cutoff
                ],
                |row| row.get::<_, String>(0),
            )?
            .collect::<Result<Vec<_>, _>>()?;

        let mut removed = 0usize;
        for session_id in session_ids {
            let session_dir = sessions_root()?.join(&session_id);
            if session_dir.exists() {
                let _ = fs::remove_dir_all(&session_dir);
            }

            let mut conn = self.connect()?;
            let tx = conn.transaction()?;
            record_activity_tx(
                &tx,
                Some(&session_id),
                None,
                "session_cleaned_up",
                None,
                None,
                None,
            )?;
            tx.execute(
                "DELETE FROM session_files WHERE session_id = ?1",
                params![session_id],
            )?;
            tx.execute(
                "DELETE FROM review_sessions WHERE id = ?1",
                params![session_id.clone()],
            )?;
            tx.commit()?;
            removed += 1;
        }

        Ok(CleanupResult {
            removed_sessions: removed,
        })
    }

    pub fn update_agent_status(
        &self,
        session_id: &str,
        status: &str,
        task: Option<&str>,
        pid: Option<i64>,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.connect()?;
        conn.execute(
            r#"
            UPDATE review_sessions
            SET agent_status = ?2, agent_task = ?3, agent_pid = ?4, last_heartbeat = ?5
            WHERE id = ?1
            "#,
            params![session_id, status, task, pid, now],
        )?;
        Ok(())
    }

    pub fn clear_agent_status(&self, session_id: &str) -> Result<(), StorageError> {
        let conn = self.connect()?;
        conn.execute(
            r#"
            UPDATE review_sessions
            SET agent_status = NULL, agent_task = NULL, agent_pid = NULL, last_heartbeat = NULL
            WHERE id = ?1
            "#,
            params![session_id],
        )?;
        Ok(())
    }

    /// Returns (session_id, agent_pid) for all sessions with the given agent_status.
    pub fn list_sessions_by_agent_status(
        &self,
        agent_status: &str,
    ) -> Result<Vec<(String, Option<i64>)>, StorageError> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_pid FROM review_sessions WHERE agent_status = ?1",
        )?;
        let rows = stmt.query_map(params![agent_status], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
    }

    pub fn mark_stale_sessions(&self) -> Result<(), StorageError> {
        let cutoff = (Utc::now() - Duration::hours(STALE_SESSION_AFTER_HOURS)).to_rfc3339();
        let conn = self.connect()?;
        conn.execute(
            r#"
            UPDATE review_sessions
            SET status = ?2
            WHERE status = ?1 AND updated_at <= ?3
            "#,
            params![
                ReviewSessionStatus::Active.as_str(),
                ReviewSessionStatus::Stale.as_str(),
                cutoff
            ],
        )?;
        Ok(())
    }

    fn connect(&self) -> Result<Connection, StorageError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&self.path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(conn)
    }

    fn initialize(&self) -> Result<(), StorageError> {
        let mut conn = self.connect()?;
        migrations().to_latest(&mut conn)?;
        Ok(())
    }
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up_with_hook(
        // Migration 1: initial schema — DO NOT MODIFY
        r#"
        CREATE TABLE IF NOT EXISTS repos (
            id INTEGER PRIMARY KEY,
            repo TEXT NOT NULL UNIQUE,
            local_path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_accessed_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS review_sessions (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            repo_id INTEGER REFERENCES repos(id) ON DELETE SET NULL,
            repo TEXT,
            pr_number INTEGER,
            title TEXT,
            body TEXT,
            url TEXT,
            local_repo_path TEXT,
            worktree_path TEXT,
            primary_file_path TEXT,
            project_root TEXT,
            author_login TEXT,
            viewer_login TEXT,
            base_ref TEXT,
            base_sha TEXT,
            head_ref TEXT,
            head_sha TEXT,
            changed_files_json TEXT NOT NULL DEFAULT '[]',
            verdict TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            last_accessed_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_review_sessions_kind_status
            ON review_sessions(kind, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_review_sessions_worktree
            ON review_sessions(worktree_path);

        CREATE TABLE IF NOT EXISTS session_files (
            id INTEGER PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
            file_path TEXT NOT NULL,
            relative_path TEXT,
            annotation_count INTEGER NOT NULL DEFAULT 0,
            pending_count INTEGER NOT NULL DEFAULT 0,
            resolved_count INTEGER NOT NULL DEFAULT 0,
            last_seen_at TEXT NOT NULL,
            UNIQUE(session_id, file_path)
        );

        CREATE INDEX IF NOT EXISTS idx_session_files_session
            ON session_files(session_id);

        CREATE TABLE IF NOT EXISTS activity_log (
            id INTEGER PRIMARY KEY,
            session_id TEXT REFERENCES review_sessions(id) ON DELETE SET NULL,
            repo_id INTEGER REFERENCES repos(id) ON DELETE SET NULL,
            activity_kind TEXT NOT NULL,
            message TEXT,
            file_path TEXT,
            payload_json TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS recent_items (
            id INTEGER PRIMARY KEY,
            item_type TEXT NOT NULL,
            session_id TEXT REFERENCES review_sessions(id) ON DELETE CASCADE,
            repo_id INTEGER REFERENCES repos(id) ON DELETE CASCADE,
            file_path TEXT,
            title TEXT,
            last_accessed_at TEXT NOT NULL
        );
        "#,
        |tx| {
            Ok(tx.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                );
                INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
                VALUES (1, 'initial_app_state', CURRENT_TIMESTAMP);
                "#,
            )?)
        },
    ),
    // Migration 2: agent status columns
    M::up(
        r#"
        ALTER TABLE review_sessions ADD COLUMN agent_status TEXT;
        ALTER TABLE review_sessions ADD COLUMN agent_task TEXT;
        ALTER TABLE review_sessions ADD COLUMN agent_pid INTEGER;
        ALTER TABLE review_sessions ADD COLUMN last_heartbeat TEXT;
        "#,
    )])
}

const SESSION_SELECT_SQL: &str = r#"
SELECT
    s.id, s.kind, s.status, s.repo, s.pr_number, s.title, s.body, s.url,
    s.local_repo_path, s.worktree_path, s.primary_file_path, s.project_root,
    s.author_login, s.viewer_login, s.base_ref, s.base_sha, s.head_ref, s.head_sha,
    s.changed_files_json, s.verdict, s.created_at, s.updated_at, s.completed_at,
    COALESCE(COUNT(f.id), 0) AS file_count,
    s.agent_status, s.agent_task, s.agent_pid, s.last_heartbeat
FROM review_sessions s
LEFT JOIN session_files f ON f.session_id = s.id
WHERE s.id = ?1
GROUP BY s.id
"#;

fn map_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredReviewSession> {
    let kind: String = row.get(1)?;
    let status: String = row.get(2)?;
    let changed_files_json: String = row.get(18)?;
    Ok(StoredReviewSession {
        id: row.get(0)?,
        kind: ReviewSessionKind::parse(&kind).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(1, "kind".into(), rusqlite::types::Type::Text)
        })?,
        status: ReviewSessionStatus::parse(&status).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(2, "status".into(), rusqlite::types::Type::Text)
        })?,
        repo: row.get(3)?,
        pr_number: row.get::<_, Option<i64>>(4)?.map(|value| value as u32),
        title: row.get(5)?,
        body: row.get(6)?,
        url: row.get(7)?,
        local_repo_path: row.get(8)?,
        worktree_path: row.get(9)?,
        primary_file_path: row.get(10)?,
        project_root: row.get(11)?,
        author_login: row.get(12)?,
        viewer_login: row.get(13)?,
        base_ref: row.get(14)?,
        base_sha: row.get(15)?,
        head_ref: row.get(16)?,
        head_sha: row.get(17)?,
        changed_files: serde_json::from_str(&changed_files_json).unwrap_or_default(),
        verdict: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
        completed_at: row.get(22)?,
        file_count: row.get::<_, i64>(23)? as usize,
        agent_status: row.get(24)?,
        agent_task: row.get(25)?,
        agent_pid: row.get(26)?,
        last_heartbeat: row.get(27)?,
    })
}

fn upsert_repo_tx(
    tx: &rusqlite::Transaction<'_>,
    repo: &str,
    local_path: &str,
    now: &str,
) -> Result<i64, StorageError> {
    tx.execute(
        r#"
        INSERT INTO repos (repo, local_path, created_at, updated_at, last_accessed_at)
        VALUES (?1, ?2, ?3, ?3, ?3)
        ON CONFLICT(repo) DO UPDATE SET
            local_path = excluded.local_path,
            updated_at = excluded.updated_at,
            last_accessed_at = excluded.last_accessed_at
        "#,
        params![repo, local_path, now],
    )?;

    Ok(tx.query_row(
        "SELECT id FROM repos WHERE repo = ?1",
        params![repo],
        |row| row.get(0),
    )?)
}

fn record_activity_tx(
    tx: &rusqlite::Transaction<'_>,
    session_id: Option<&str>,
    repo_id: Option<i64>,
    kind: &str,
    message: Option<&str>,
    file_path: Option<&str>,
    payload_json: Option<&str>,
) -> Result<(), StorageError> {
    tx.execute(
        r#"
        INSERT INTO activity_log (session_id, repo_id, activity_kind, message, file_path, payload_json, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            session_id,
            repo_id,
            kind,
            message,
            file_path,
            payload_json,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn insert_session_file_tx(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    file: &IndexedSessionFile,
) -> Result<(), StorageError> {
    tx.execute(
        r#"
        INSERT INTO session_files (
            session_id, file_path, relative_path, annotation_count, pending_count, resolved_count, last_seen_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(session_id, file_path) DO UPDATE SET
            relative_path = excluded.relative_path,
            annotation_count = excluded.annotation_count,
            pending_count = excluded.pending_count,
            resolved_count = excluded.resolved_count,
            last_seen_at = excluded.last_seen_at
        "#,
        params![
            session_id,
            file.file_path,
            file.relative_path,
            file.annotation_count as i64,
            file.pending_count as i64,
            file.resolved_count as i64,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn app_home_path() -> Result<PathBuf, StorageError> {
    let home = dirs::home_dir()
        .ok_or_else(|| StorageError::Message("could not resolve home directory".into()))?;
    Ok(home.join(APP_HOME_DIRECTORY))
}

pub fn settings_path() -> Result<PathBuf, StorageError> {
    Ok(app_home_path()?.join(SETTINGS_FILE_NAME))
}

pub fn state_db_path() -> Result<PathBuf, StorageError> {
    Ok(app_home_path()?.join(STATE_DB_FILE_NAME))
}

pub fn sessions_root() -> Result<PathBuf, StorageError> {
    Ok(app_home_path()?.join("sessions"))
}

pub fn checkouts_root() -> Result<PathBuf, StorageError> {
    Ok(app_home_path()?.join("checkouts"))
}

pub fn is_stale_timestamp(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc) <= Utc::now() - Duration::hours(STALE_SESSION_AFTER_HOURS))
        .unwrap_or(false)
}
