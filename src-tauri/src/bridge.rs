use crate::commands::github_review::open_github_pr_review_impl;
use crate::commands::github_review::GitHubPrSession;
use crate::state::AppState;
use crate::storage::{
    IndexedSessionFile, ReviewSessionKind, ReviewSessionStatus, StoredReviewSession,
};
use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::Annotation;
use redpen_core::hash::hash_file;
use redpen_core::sidecar::SidecarFile;
use redpen_server::AppBridge;
use redpen_server::ReviewSessionState;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Implements `AppBridge` by delegating to Tauri's `AppHandle` and the sidecar store.
pub struct TauriBridge {
    handle: AppHandle,
}

impl TauriBridge {
    pub fn new(handle: AppHandle) -> Arc<Self> {
        Arc::new(Self { handle })
    }
}

impl AppBridge for TauriBridge {
    fn open_file(
        &self,
        file: &str,
        line: Option<u32>,
        review_session_id: Option<&str>,
    ) -> Result<(), String> {
        let mut url = format!("redpen://open?file={}", urlencoding::encode(file));
        if let Some(l) = line {
            url.push_str(&format!("&line={}", l));
        }
        if let Some(session_id) = review_session_id {
            url.push_str(&format!(
                "&reviewSession={}",
                urlencoding::encode(session_id)
            ));
        }
        self.handle
            .emit("deep-link-open", &url)
            .map_err(|e| e.to_string())
    }

    fn refresh_file(&self, file: &str) -> Result<(), String> {
        let url = format!("redpen://refresh?file={}", urlencoding::encode(file));
        self.handle
            .emit("deep-link-open", &url)
            .map_err(|e| e.to_string())
    }

    fn get_annotations(&self, file: &str) -> Result<Vec<Annotation>, String> {
        let source_path = Path::new(file);
        let project_root = resolve_project_root(source_path);
        let sidecar =
            load_sidecar_for_file(&project_root, source_path).map_err(|e| e.to_string())?;
        Ok(sidecar.annotations)
    }

    fn open_pr_review(
        &self,
        pr_ref: &str,
        local_path_hint: Option<&str>,
    ) -> Result<String, String> {
        let state = self.handle.state::<AppState>();
        let session = open_github_pr_review_impl(
            &state,
            pr_ref.to_string(),
            local_path_hint.map(ToString::to_string),
        )
        .map_err(|e| e.to_string())?;
        self.handle
            .emit(
                "open-github-review-session",
                SerializableGitHubPrSession::from(&session),
            )
            .map_err(|e| e.to_string())?;
        Ok(session.worktree_path)
    }

    fn start_review_session(&self, session_id: &str, file: &str) -> Result<(), String> {
        let state = self.handle.state::<AppState>();
        let source_path = Path::new(file);
        let project_root = resolve_project_root(source_path);
        let sidecar = load_sidecar_for_file(&project_root, source_path)?;
        let relative_path = source_path
            .strip_prefix(&project_root)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
        let now = chrono::Utc::now().to_rfc3339();
        let record = StoredReviewSession {
            id: session_id.to_string(),
            kind: ReviewSessionKind::LocalReview,
            status: ReviewSessionStatus::Active,
            repo: None,
            pr_number: None,
            title: source_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
            body: None,
            url: None,
            local_repo_path: None,
            worktree_path: None,
            primary_file_path: Some(file.to_string()),
            project_root: Some(project_root.to_string_lossy().to_string()),
            author_login: None,
            viewer_login: None,
            base_ref: None,
            base_sha: None,
            head_ref: None,
            head_sha: None,
            changed_files: Vec::new(),
            verdict: None,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
            file_count: 1,
        };
        state
            .storage
            .upsert_review_session(&record)
            .map_err(|e| e.to_string())?;
        state
            .storage
            .replace_session_files(
                session_id,
                &[IndexedSessionFile {
                    file_path: file.to_string(),
                    relative_path,
                    annotation_count: sidecar.annotations.len(),
                    pending_count: 0,
                    resolved_count: sidecar
                        .annotations
                        .iter()
                        .filter(|annotation| annotation.resolved)
                        .count(),
                }],
            )
            .map_err(|e| e.to_string())
    }

    fn complete_review_session(&self, session_id: &str, verdict: &str) -> Result<(), String> {
        let state = self.handle.state::<AppState>();
        if let Some(session) = state
            .storage
            .complete_review_session(session_id, verdict)
            .map_err(|e| e.to_string())?
        {
            if let (Some(project_root), Some(file_path)) =
                (session.project_root, session.primary_file_path)
            {
                let source_path = Path::new(&file_path);
                if source_path.exists() {
                    let sidecar = load_sidecar_for_file(Path::new(&project_root), source_path)?;
                    let relative_path = source_path
                        .strip_prefix(Path::new(&project_root))
                        .ok()
                        .map(|path| path.to_string_lossy().to_string());
                    state
                        .storage
                        .replace_session_files(
                            session_id,
                            &[IndexedSessionFile {
                                file_path,
                                relative_path,
                                annotation_count: sidecar.annotations.len(),
                                pending_count: 0,
                                resolved_count: sidecar
                                    .annotations
                                    .iter()
                                    .filter(|annotation| annotation.resolved)
                                    .count(),
                            }],
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    fn cancel_review_session(&self, session_id: &str) -> Result<(), String> {
        let state = self.handle.state::<AppState>();
        state
            .storage
            .cancel_review_session(session_id)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn timeout_review_session(&self, session_id: &str) -> Result<(), String> {
        let state = self.handle.state::<AppState>();
        state
            .storage
            .timeout_review_session(session_id)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn review_session_status(
        &self,
        session_id: &str,
    ) -> Result<Option<ReviewSessionState>, String> {
        let state = self.handle.state::<AppState>();
        state
            .storage
            .review_session_status(session_id)
            .map_err(|e| e.to_string())
            .map(|status| {
                status.map(|(session_status, verdict, file)| ReviewSessionState {
                    status: session_status.as_str().to_string(),
                    file,
                    verdict,
                })
            })
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializableGitHubPrSession<'a> {
    id: &'a str,
    repo: &'a str,
    number: u32,
    author_login: &'a str,
    viewer_login: &'a str,
    title: &'a str,
    body: &'a str,
    url: &'a str,
    base_ref: &'a str,
    base_sha: &'a str,
    head_ref: &'a str,
    head_sha: &'a str,
    local_repo_path: &'a str,
    worktree_path: &'a str,
    changed_files: &'a [String],
    updated_at: &'a str,
}

impl<'a> From<&'a GitHubPrSession> for SerializableGitHubPrSession<'a> {
    fn from(session: &'a GitHubPrSession) -> Self {
        Self {
            id: &session.id,
            repo: &session.repo,
            number: session.number,
            author_login: &session.author_login,
            viewer_login: &session.viewer_login,
            title: &session.title,
            body: &session.body,
            url: &session.url,
            base_ref: &session.base_ref,
            base_sha: &session.base_sha,
            head_ref: &session.head_ref,
            head_sha: &session.head_sha,
            local_repo_path: &session.local_repo_path,
            worktree_path: &session.worktree_path,
            changed_files: &session.changed_files,
            updated_at: &session.updated_at,
        }
    }
}

fn resolve_project_root(source_path: &Path) -> std::path::PathBuf {
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
    }
}

fn load_sidecar_for_file(project_root: &Path, source_path: &Path) -> Result<SidecarFile, String> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if annotation_path.exists() {
        let mut sidecar = SidecarFile::load(&annotation_path).map_err(|e| e.to_string())?;
        let current_hash = hash_file(source_path).map_err(|e| e.to_string())?;
        if sidecar.source_file_hash != current_hash {
            let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
            reanchor_annotations(&mut sidecar.annotations, &content);
            sidecar.source_file_hash = current_hash;
        }
        Ok(sidecar)
    } else {
        let hash = hash_file(source_path).map_err(|e| e.to_string())?;
        Ok(SidecarFile::new(hash))
    }
}
