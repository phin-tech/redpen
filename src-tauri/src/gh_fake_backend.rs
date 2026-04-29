//! Adapter implementing `redpen_gh_fake::GhBackend` against Tauri-side state.
//!
//! Maps fake-GitHub operations onto Redpen sessions:
//!   * Reads walk session sidecars and surface annotations as PR review comments.
//!   * Writes (`append_reply`, `set_thread_resolved`) mutate sidecars directly,
//!     marking new replies as `PendingPublish` so they ride the existing human
//!     "Submit review" flow when the user is ready.
//!
//! Known limitation: comment ids returned to clients are synthetic (a stable
//! 53-bit hash of the GraphQL node id), and the existing publish path uses
//! the same node id when posting replies upstream — that's the REST/databaseId
//! bug tracked as plan §1. Local round-trips work; upstream publish for replies
//! to imported comments is broken until §1 lands.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use redpen_core::annotation::{
    Anchor, Annotation, AnnotationKind, GitHubAnnotationMetadata, GitHubSyncState,
};
use redpen_core::sidecar::SidecarFile;
use redpen_gh_fake::{BackendError, GhBackend, ReviewComment, SessionRef, ThreadRef};
use tauri::{AppHandle, Manager};

use crate::commands::github_review::{
    list_session_sidecars, load_session_by_id, GitHubPrSession,
};
use crate::state::AppState;
use crate::storage::{ReviewSessionKind, ReviewSessionStatus, StateDb, StoredReviewSession};

const SESSION_LIST_LIMIT: usize = 200;

pub struct TauriGhBackend {
    handle: AppHandle,
}

impl TauriGhBackend {
    pub fn new(handle: AppHandle) -> Arc<Self> {
        Arc::new(Self { handle })
    }

    fn db(&self) -> StateDb {
        self.handle.state::<AppState>().storage.clone()
    }

    fn list_active_gh_sessions(&self) -> Vec<StoredReviewSession> {
        self.db()
            .list_recent_sessions(
                Some(ReviewSessionKind::GitHubPr),
                Some(ReviewSessionStatus::Active),
                SESSION_LIST_LIMIT,
            )
            .unwrap_or_default()
    }

    fn load_session(&self, session_id: &str) -> Option<GitHubPrSession> {
        load_session_by_id(&self.db(), session_id).ok()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Stable, positive 53-bit hash of a GraphQL node id. Fits losslessly in JSON
/// numbers and round-trips through `Number(commentId)` on the client side.
fn hash_node_id(node_id: &str) -> i64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    node_id.hash(&mut h);
    (h.finish() & 0x001F_FFFF_FFFF_FFFF) as i64
}

fn split_owner_repo(repo: &str) -> Option<(String, String)> {
    repo.split_once('/').map(|(o, n)| (o.into(), n.into()))
}

fn session_to_ref(s: &StoredReviewSession) -> Option<SessionRef> {
    let repo = s.repo.as_deref()?;
    let (owner, repo_name) = split_owner_repo(repo)?;
    Some(SessionRef {
        id: s.id.clone(),
        owner,
        repo: repo_name,
        number: u64::from(s.pr_number?),
        title: s.title.clone().unwrap_or_default(),
        body: s.body.clone().unwrap_or_default(),
        head_ref: s.head_ref.clone().unwrap_or_default(),
        head_sha: s.head_sha.clone().unwrap_or_default(),
        base_ref: s.base_ref.clone().unwrap_or_default(),
        base_sha: s.base_sha.clone().unwrap_or_default(),
        author_login: s.author_login.clone().unwrap_or_default(),
        viewer_login: s.viewer_login.clone().unwrap_or_default(),
        html_url: s.url.clone().unwrap_or_default(),
    })
}

/// Walk every sidecar in a session and yield (sidecar_path, source_path,
/// relative_path_from_worktree, annotation). The annotation is cloned out so
/// callers don't keep the sidecar file open.
fn walk_annotations(
    session: &GitHubPrSession,
) -> Vec<(PathBuf, PathBuf, String, Annotation)> {
    let mut out = Vec::new();
    let sidecars = list_session_sidecars(session).unwrap_or_default();
    for (source_path, sidecar_path) in sidecars {
        let Ok(sidecar) = SidecarFile::load(&sidecar_path) else {
            continue;
        };
        let relative = source_path
            .strip_prefix(&session.worktree_path)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| source_path.to_string_lossy().into_owned());
        for annotation in sidecar.annotations {
            out.push((
                sidecar_path.clone(),
                source_path.clone(),
                relative.clone(),
                annotation,
            ));
        }
    }
    out
}

fn anchor_line(a: &Annotation) -> u32 {
    let Anchor::TextContext { range, .. } = &a.anchor;
    range.start_line
}

fn annotation_node_id(a: &Annotation) -> Option<&str> {
    a.github.as_ref()?.external_comment_id.as_deref()
}

fn annotation_thread_id(a: &Annotation) -> Option<&str> {
    a.github.as_ref()?.external_thread_id.as_deref()
}

/// Stable REST `databaseId` for an annotation. Prefers the real GitHub
/// `databaseId` populated at import time; falls back to a hash for legacy
/// sidecars (imported before the schema split) and for locally-authored
/// drafts that don't have an upstream id yet.
fn database_id_for(a: &Annotation) -> i64 {
    if let Some(id) = a
        .github
        .as_ref()
        .and_then(|m| m.external_comment_database_id)
    {
        return id;
    }
    match annotation_node_id(a) {
        Some(node_id) => hash_node_id(node_id),
        None => hash_node_id(&a.id),
    }
}

fn node_id_for(a: &Annotation) -> String {
    match annotation_node_id(a) {
        Some(node_id) => node_id.to_string(),
        None => format!("REDPEN_DRAFT_{}", a.id),
    }
}

/// True for annotation kinds that represent a PR review comment / line note.
/// Filters out labels, questions, and explanations — those aren't reviews.
fn is_review_kind(kind: &AnnotationKind) -> bool {
    matches!(kind, AnnotationKind::Comment | AnnotationKind::LineNote)
}

/// Synthetic thread id for locally-authored review comments that have no
/// upstream GitHub thread. Format keeps it distinguishable from real
/// `PRRT_*` ids and stable across reads.
fn synthetic_thread_id(local_uuid: &str) -> String {
    format!("REDPEN_THREAD_{}", local_uuid)
}

/// Where to store a new top-level review comment for `absolute_source_path`
/// inside the given session. Mirrors `session_sidecar_path` in github_review.rs
/// but without erroring when the source file is outside the worktree (we just
/// fall back to the file name).
fn sidecar_path_for(
    session: &GitHubPrSession,
    absolute_source_path: &Path,
) -> Option<PathBuf> {
    let session_dir = dirs::home_dir()?
        .join(".config")
        .join("redpen")
        .join("sessions")
        .join(&session.id)
        .join("comments");
    let relative = absolute_source_path
        .strip_prefix(&session.worktree_path)
        .ok()?;
    let file_name = relative.file_name()?.to_string_lossy().into_owned();
    Some(session_dir.join(relative.with_file_name(format!("{file_name}.json"))))
}

fn to_review_comment(
    a: &Annotation,
    relative_path: &str,
    pr_url: &str,
    parent_db_id: Option<i64>,
) -> ReviewComment {
    let database_id = database_id_for(a);
    ReviewComment {
        database_id,
        node_id: node_id_for(a),
        thread_id: annotation_thread_id(a).unwrap_or("").to_string(),
        path: relative_path.to_string(),
        line: anchor_line(a),
        diff_hunk: String::new(),
        body: a.body.clone(),
        author: a.author.clone(),
        created_at: a.created_at.unwrap_or_else(Utc::now),
        updated_at: a.updated_at.unwrap_or_else(Utc::now),
        in_reply_to_database_id: parent_db_id,
        html_url: format!("{}#discussion_r{}", pr_url, database_id),
    }
}

// ---------------------------------------------------------------------------
// Trait impl
// ---------------------------------------------------------------------------

#[async_trait]
impl GhBackend for TauriGhBackend {
    async fn find_session_for_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Option<SessionRef> {
        let target = format!("{}/{}", owner, repo);
        self.list_active_gh_sessions()
            .iter()
            .find(|s| {
                s.repo.as_deref() == Some(target.as_str())
                    && s.head_ref.as_deref() == Some(branch)
            })
            .and_then(session_to_ref)
    }

    async fn find_session_for_pr(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Option<SessionRef> {
        let target = format!("{}/{}", owner, repo);
        self.list_active_gh_sessions()
            .iter()
            .find(|s| {
                s.repo.as_deref() == Some(target.as_str())
                    && s.pr_number.map(u64::from) == Some(number)
            })
            .and_then(session_to_ref)
    }

    async fn list_active_sessions(&self) -> Vec<SessionRef> {
        self.list_active_gh_sessions()
            .iter()
            .filter_map(session_to_ref)
            .collect()
    }

    async fn list_review_comments(&self, session_id: &str) -> Vec<ReviewComment> {
        let Some(session) = self.load_session(session_id) else {
            return Vec::new();
        };
        let entries = walk_annotations(&session);

        // Pass 1: every review-kind annotation (imported or draft) gets a
        // stable database_id, indexed by local UUID for in_reply_to mapping.
        let mut local_to_db: HashMap<String, i64> = HashMap::new();
        for (_, _, _, a) in &entries {
            if is_review_kind(&a.kind) {
                local_to_db.insert(a.id.clone(), database_id_for(a));
            }
        }

        // Pass 2: produce ReviewComments. Includes drafts (agent replies,
        // PendingPublish) so subsequent reads see prior responses and don't
        // re-address resolved threads.
        entries
            .iter()
            .filter(|(_, _, _, a)| is_review_kind(&a.kind))
            .map(|(_, _, relative, a)| {
                let parent_db = a
                    .reply_to
                    .as_deref()
                    .and_then(|local| local_to_db.get(local).copied());
                to_review_comment(a, relative, &session.url, parent_db)
            })
            .collect()
    }

    async fn list_threads(&self, session_id: &str) -> Vec<ThreadRef> {
        let Some(session) = self.load_session(session_id) else {
            return Vec::new();
        };
        walk_annotations(&session)
            .iter()
            .filter(|(_, _, _, a)| a.reply_to.is_none() && is_review_kind(&a.kind))
            .map(|(_, _, _, a)| ThreadRef {
                node_id: annotation_thread_id(a)
                    .map(String::from)
                    .unwrap_or_else(|| synthetic_thread_id(&a.id)),
                is_resolved: a.resolved,
                root_database_id: database_id_for(a),
            })
            .collect()
    }

    async fn append_review_comment(
        &self,
        session_id: &str,
        agent: &str,
        path: &str,
        line: u32,
        body: &str,
    ) -> Result<ReviewComment, BackendError> {
        let session = self
            .load_session(session_id)
            .ok_or(BackendError::NotFound)?;

        // Resolve the file path inside the worktree and load (or create) its
        // sidecar. `path` is relative to the repo root.
        let absolute = std::path::Path::new(&session.worktree_path).join(path);
        let sidecar_path = sidecar_path_for(&session, &absolute).ok_or_else(|| {
            BackendError::Internal(format!("could not derive sidecar path for {}", path))
        })?;
        if let Some(parent) = sidecar_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| BackendError::Internal(format!("create sidecar dir: {e}")))?;
        }

        // Anchor at the requested line. We don't have the file's source hash
        // available cheaply; use an empty hash + minimal anchor (Redpen will
        // re-anchor on next load if the source has changed).
        let anchor = Anchor::TextContext {
            line_content: String::new(),
            surrounding_lines: vec![],
            content_hash: String::new(),
            range: redpen_core::annotation::Range {
                start_line: line,
                start_column: 0,
                end_line: line,
                end_column: 0,
            },
            last_known_line: line,
        };

        let mut annotation = Annotation::new(
            redpen_core::annotation::AnnotationKind::Comment,
            body.to_string(),
            vec![],
            agent.to_string(),
            anchor,
        );
        // Bot-seeded comments are local-only review content. Synthesize a
        // thread id so list_threads/set_thread_resolved can address them.
        annotation.github = Some(GitHubAnnotationMetadata {
            sync_state: Some(GitHubSyncState::LocalOnly),
            external_comment_id: None,
            external_thread_id: Some(synthetic_thread_id(&annotation.id)),
            publishable_reason: None,
            ..Default::default()
        });

        let mut sidecar = if sidecar_path.exists() {
            SidecarFile::load(&sidecar_path)
                .map_err(|e| BackendError::Internal(format!("load sidecar: {e}")))?
        } else {
            // No content_hash available without reading the file — use empty.
            SidecarFile::new(String::new())
        };
        sidecar.annotations.push(annotation.clone());
        sidecar
            .save(&sidecar_path)
            .map_err(|e| BackendError::Internal(format!("save sidecar: {e}")))?;

        let database_id = database_id_for(&annotation);
        Ok(ReviewComment {
            database_id,
            node_id: node_id_for(&annotation),
            thread_id: synthetic_thread_id(&annotation.id),
            path: path.to_string(),
            line,
            diff_hunk: String::new(),
            body: annotation.body.clone(),
            author: annotation.author.clone(),
            created_at: annotation.created_at.unwrap_or_else(Utc::now),
            updated_at: annotation.updated_at.unwrap_or_else(Utc::now),
            in_reply_to_database_id: None,
            html_url: format!("{}#discussion_r{}", session.url, database_id),
        })
    }

    async fn append_reply(
        &self,
        session_id: &str,
        parent_database_id: i64,
        agent: &str,
        body: &str,
    ) -> Result<ReviewComment, BackendError> {
        let session = self
            .load_session(session_id)
            .ok_or(BackendError::NotFound)?;

        // Find the parent annotation by its stable database_id (works for
        // both imported comments and prior agent drafts).
        let entries = walk_annotations(&session);
        let parent = entries
            .iter()
            .find(|(_, _, _, a)| {
                is_review_kind(&a.kind) && database_id_for(a) == parent_database_id
            })
            .ok_or(BackendError::NotFound)?
            .clone();
        let (parent_sidecar, _parent_source, parent_relative, parent_annotation) = parent;

        // Build the reply annotation, reusing the parent's anchor so it
        // anchors to the same line in the same file.
        let mut reply = Annotation::new_reply(
            body.to_string(),
            agent.to_string(),
            parent_annotation.id.clone(),
            parent_annotation.anchor.clone(),
        );
        reply.github = Some(GitHubAnnotationMetadata {
            sync_state: Some(GitHubSyncState::PendingPublish),
            external_comment_id: None,
            external_thread_id: parent_annotation
                .github
                .as_ref()
                .and_then(|m| m.external_thread_id.clone()),
            publishable_reason: None,
            ..Default::default()
        });

        // Append + save the sidecar that holds the parent.
        let mut sidecar = SidecarFile::load(&parent_sidecar)
            .map_err(|e| BackendError::Internal(format!("load sidecar: {e}")))?;
        sidecar.annotations.push(reply.clone());
        sidecar
            .save(&parent_sidecar)
            .map_err(|e| BackendError::Internal(format!("save sidecar: {e}")))?;

        let database_id = database_id_for(&reply);
        Ok(ReviewComment {
            database_id,
            node_id: node_id_for(&reply),
            thread_id: parent_annotation
                .github
                .as_ref()
                .and_then(|m| m.external_thread_id.clone())
                .unwrap_or_default(),
            path: parent_relative,
            line: anchor_line(&reply),
            diff_hunk: String::new(),
            body: reply.body.clone(),
            author: reply.author.clone(),
            created_at: reply.created_at.unwrap_or_else(Utc::now),
            updated_at: reply.updated_at.unwrap_or_else(Utc::now),
            in_reply_to_database_id: Some(parent_database_id),
            html_url: format!("{}#discussion_r{}", session.url, database_id),
        })
    }

    async fn set_thread_resolved(
        &self,
        _session_id: &str,
        thread_node_id: &str,
        resolved: bool,
    ) -> Result<ThreadRef, BackendError> {
        // GraphQL mutation only carries the threadId — scan all active GH
        // sessions to find the owning sidecar.
        for stored in self.list_active_gh_sessions() {
            let Some(session) = self.load_session(&stored.id) else {
                continue;
            };
            for (sidecar_path, _, _, _) in walk_annotations(&session) {
                let mut sidecar = match SidecarFile::load(&sidecar_path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut hit = None;
                for annotation in sidecar.annotations.iter_mut() {
                    if annotation.reply_to.is_some() {
                        continue; // only roots can resolve a thread
                    }
                    let matches_external =
                        annotation_thread_id(annotation) == Some(thread_node_id);
                    let matches_synthetic =
                        synthetic_thread_id(&annotation.id) == thread_node_id;
                    if matches_external || matches_synthetic {
                        let prior = annotation.resolved;
                        annotation.resolved = resolved;
                        annotation.updated_at = Some(Utc::now());
                        // Flag the resolution change for upstream publish on
                        // imported annotations whose state actually flipped.
                        if let Some(meta) = annotation.github.as_mut() {
                            if matches!(meta.sync_state, Some(GitHubSyncState::Imported))
                                && resolved != prior
                            {
                                meta.pending_resolution_change = Some(true);
                            }
                        }
                        hit = Some(ThreadRef {
                            node_id: thread_node_id.to_string(),
                            is_resolved: resolved,
                            root_database_id: database_id_for(annotation),
                        });
                        break;
                    }
                }
                if let Some(thread_ref) = hit {
                    sidecar
                        .save(&sidecar_path)
                        .map_err(|e| BackendError::Internal(format!("save sidecar: {e}")))?;
                    let _ = Path::new(&sidecar_path); // silence unused (compiler fine, but keeps intent)
                    return Ok(thread_ref);
                }
            }
        }
        Err(BackendError::NotFound)
    }
}
