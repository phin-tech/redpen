use crate::commands::error::{CommandError, CommandResult};
use crate::commands::github_review::GitHubPrSession;
use crate::state::AppState;
use crate::storage::{
    is_stale_timestamp, ReviewSessionKind, ReviewSessionStatus, StoredReviewSession,
};
use tauri::State;
use ts_rs::TS;

#[derive(Debug, Clone, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ReviewHistoryItem {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub subtitle: String,
    pub updated_at: String,
    pub primary_file_path: Option<String>,
    pub file_count: usize,
    pub verdict: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ReviewHistory {
    pub active_session: Option<ReviewHistoryItem>,
    pub recent_pull_requests: Vec<ReviewHistoryItem>,
    pub recent_files: Vec<ReviewHistoryItem>,
    pub stale_sessions: Vec<ReviewHistoryItem>,
}

#[derive(Debug, Clone, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct ResumeReviewSessionResult {
    pub kind: String,
    pub session_id: String,
    pub project_root: Option<String>,
    pub files: Vec<String>,
    pub github_session: Option<GitHubPrSession>,
}

#[derive(Debug, Clone, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct CleanupReviewSessionsResult {
    pub removed_sessions: usize,
}

#[tauri::command]
pub fn get_review_history(state: State<'_, AppState>) -> CommandResult<ReviewHistory> {
    let sessions = state
        .storage
        .list_recent_sessions(None, None, 50)
        .map_err(storage_error)?;

    let active_session = sessions
        .iter()
        .find(|session| {
            session.status == ReviewSessionStatus::Active
                && !is_stale_timestamp(&session.updated_at)
        })
        .cloned()
        .map(history_item_from_session);

    let recent_pull_requests = sessions
        .iter()
        .filter(|session| session.kind == ReviewSessionKind::GitHubPr)
        .take(6)
        .cloned()
        .map(history_item_from_session)
        .collect();

    let recent_files = sessions
        .iter()
        .filter(|session| session.kind == ReviewSessionKind::LocalReview)
        .take(6)
        .cloned()
        .map(history_item_from_session)
        .collect();

    let stale_sessions = sessions
        .iter()
        .filter(|session| {
            session.status == ReviewSessionStatus::Stale
                || (session.status == ReviewSessionStatus::Active
                    && is_stale_timestamp(&session.updated_at))
        })
        .take(6)
        .cloned()
        .map(history_item_from_session)
        .collect();

    Ok(ReviewHistory {
        active_session,
        recent_pull_requests,
        recent_files,
        stale_sessions,
    })
}

#[tauri::command]
pub fn resume_review_session(
    session_id: String,
    state: State<'_, AppState>,
) -> CommandResult<ResumeReviewSessionResult> {
    let session = state
        .storage
        .get_review_session(&session_id)
        .map_err(storage_error)?
        .ok_or_else(|| {
            CommandError::NotFound(format!("Review session {} not found", session_id))
        })?;
    let files = state
        .storage
        .list_session_files(&session.id)
        .map_err(storage_error)?
        .into_iter()
        .map(|file| file.file_path)
        .collect::<Vec<_>>();

    let github_session = if session.kind == ReviewSessionKind::GitHubPr {
        Some(github_session_from_stored(session.clone())?)
    } else {
        None
    };

    Ok(ResumeReviewSessionResult {
        kind: session.kind.as_str().to_string(),
        session_id: session.id,
        project_root: session.project_root,
        files,
        github_session,
    })
}

#[tauri::command]
pub fn cleanup_stale_review_sessions(
    state: State<'_, AppState>,
) -> CommandResult<CleanupReviewSessionsResult> {
    let result = state
        .storage
        .cleanup_stale_sessions()
        .map_err(storage_error)?;
    Ok(CleanupReviewSessionsResult {
        removed_sessions: result.removed_sessions,
    })
}

fn history_item_from_session(session: StoredReviewSession) -> ReviewHistoryItem {
    let title = match session.kind {
        ReviewSessionKind::GitHubPr => session.title.clone().unwrap_or_else(|| {
            format!(
                "{} #{}",
                session.repo.clone().unwrap_or_default(),
                session.pr_number.unwrap_or_default()
            )
        }),
        ReviewSessionKind::LocalReview => session
            .title
            .clone()
            .or_else(|| session.primary_file_path.clone())
            .unwrap_or_else(|| "File review".to_string()),
    };

    let subtitle = match session.kind {
        ReviewSessionKind::GitHubPr => format!(
            "{} #{}",
            session.repo.clone().unwrap_or_default(),
            session.pr_number.unwrap_or_default()
        ),
        ReviewSessionKind::LocalReview => session
            .primary_file_path
            .clone()
            .unwrap_or_else(|| "Local review".to_string()),
    };

    ReviewHistoryItem {
        id: session.id,
        kind: session.kind.as_str().to_string(),
        status: session.status.as_str().to_string(),
        title,
        subtitle,
        updated_at: session.updated_at,
        primary_file_path: session.primary_file_path,
        file_count: session.file_count,
        verdict: session.verdict,
    }
}

fn github_session_from_stored(session: StoredReviewSession) -> CommandResult<GitHubPrSession> {
    Ok(GitHubPrSession {
        id: session.id,
        repo: session.repo.ok_or_else(|| {
            CommandError::InvalidArgument("Stored review session is missing repo".into())
        })?,
        number: session.pr_number.ok_or_else(|| {
            CommandError::InvalidArgument("Stored review session is missing PR number".into())
        })?,
        title: session.title.unwrap_or_default(),
        author_login: session.author_login.unwrap_or_default(),
        viewer_login: session.viewer_login.unwrap_or_default(),
        body: session.body.unwrap_or_default(),
        url: session.url.unwrap_or_default(),
        base_ref: session.base_ref.unwrap_or_default(),
        base_sha: session.base_sha.unwrap_or_default(),
        head_ref: session.head_ref.unwrap_or_default(),
        head_sha: session.head_sha.unwrap_or_default(),
        local_repo_path: session.local_repo_path.unwrap_or_default(),
        worktree_path: session.worktree_path.unwrap_or_default(),
        changed_files: session.changed_files,
        updated_at: session.updated_at,
    })
}

fn storage_error(error: crate::storage::StorageError) -> CommandError {
    CommandError::InvalidArgument(error.to_string())
}
