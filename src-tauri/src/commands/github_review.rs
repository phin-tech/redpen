use crate::commands::error::{CommandError, CommandResult};
use crate::settings::{normalize_optional_path, normalize_tracked_repos, TrackedRepo};
use crate::state::AppState;
use crate::storage::{
    checkouts_root, sessions_root, IndexedSessionFile, ReviewSessionKind, ReviewSessionStatus,
    StateDb, StoredReviewSession,
};
use chrono::Utc;
use git2::Repository;
use redpen_core::annotation::{
    Anchor, Annotation, AnnotationKind, GitHubAnnotationMetadata, GitHubSyncState, Range,
};
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;
use ts_rs::TS;

const REVIEW_THREADS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 100) {
        nodes {
          id
          isResolved
          comments(first: 100) {
            nodes {
              id
              body
              path
              line
              originalLine
              createdAt
              updatedAt
              author { login }
              replyTo { id }
            }
          }
        }
      }
    }
  }
}
"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GitHubReviewQueueItem {
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub url: String,
    pub author: String,
    pub updated_at: String,
    pub base_ref: String,
    pub head_ref: String,
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GitHubPrSession {
    pub id: String,
    pub repo: String,
    pub number: u32,
    pub title: String,
    #[serde(default)]
    pub author_login: String,
    #[serde(default)]
    pub viewer_login: String,
    #[serde(default)]
    pub body: String,
    pub url: String,
    pub base_ref: String,
    pub base_sha: String,
    pub head_ref: String,
    pub head_sha: String,
    pub local_repo_path: String,
    pub worktree_path: String,
    #[serde(default)]
    pub changed_files: Vec<String>,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct SubmitGitHubReviewResult {
    pub session: GitHubPrSession,
    pub published_count: usize,
    pub reply_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum GitHubReviewEvent {
    Comment,
    Approve,
    RequestChanges,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrViewJson {
    title: String,
    author: GhActor,
    #[serde(default)]
    body: String,
    url: String,
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: String,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GhActor {
    login: String,
}

#[derive(Debug, Clone)]
struct PrRef {
    repo: String,
    number: u32,
}

#[tauri::command]
pub fn list_github_review_queue(
    state: State<'_, AppState>,
) -> CommandResult<Vec<GitHubReviewQueueItem>> {
    let tracked = state
        .settings
        .lock()
        .map_err(|e| CommandError::InvalidArgument(format!("settings lock poisoned: {e}")))?
        .tracked_github_repos
        .clone();

    let mut queue = Vec::new();
    for repo in tracked {
        let output = run_gh_json(
            [
                "pr",
                "list",
                "--repo",
                repo.repo.as_str(),
                "--search",
                "state:open review-requested:@me",
                "--json",
                "number,title,url,updatedAt,author,baseRefName,headRefName",
            ],
            None,
        )?;
        let items = output.as_array().cloned().unwrap_or_default();
        for item in items {
            queue.push(GitHubReviewQueueItem {
                repo: repo.repo.clone(),
                number: item
                    .get("number")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32,
                title: item
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                url: item
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                author: item
                    .get("author")
                    .and_then(|v| v.get("login"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                updated_at: item
                    .get("updatedAt")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                base_ref: item
                    .get("baseRefName")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                head_ref: item
                    .get("headRefName")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                local_path: repo.local_path.clone(),
            });
        }
    }

    queue.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(queue)
}

#[tauri::command]
pub fn open_github_pr_review(
    pr_ref: String,
    local_path_hint: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<GitHubPrSession> {
    open_github_pr_review_impl(&state, pr_ref, local_path_hint)
}

pub fn open_github_pr_review_impl(
    state: &AppState,
    pr_ref: String,
    local_path_hint: Option<String>,
) -> CommandResult<GitHubPrSession> {
    let db = &state.storage;
    let settings = state
        .settings
        .lock()
        .map_err(|e| CommandError::InvalidArgument(format!("settings lock poisoned: {e}")))?
        .clone();
    let tracked = settings.tracked_github_repos.clone();

    let parsed = parse_pr_ref(&pr_ref, &tracked)?;
    let local_repo_path = resolve_local_repo_path(
        &parsed.repo,
        local_path_hint,
        &tracked,
        settings.default_checkout_root.clone(),
    )?;
    let pr_json = load_pr_view(&parsed, &local_repo_path)?;
    ensure_repo_is_tracked(state, &parsed.repo, &local_repo_path)?;
    let session = ensure_worktree_and_session(db, &parsed, &pr_json, &local_repo_path)?;
    write_imported_review_state(db, &session)?;
    Ok(session)
}

#[tauri::command]
pub fn resync_github_pr_review(
    session_id: String,
    state: State<'_, AppState>,
) -> CommandResult<GitHubPrSession> {
    let db = &state.storage;
    let session = load_session_by_id(db, &session_id)?;
    let pr = PrRef {
        repo: session.repo.clone(),
        number: session.number,
    };
    let pr_json = load_pr_view(&pr, &session.local_repo_path)?;
    if pr_json.head_ref_oid != session.head_sha && has_pending_local_annotations(&session)? {
        return Err(CommandError::InvalidArgument(
            "PR head changed and there are unpublished local annotations. Submit or discard them before resyncing.".into(),
        ));
    }

    let next_session = ensure_worktree_and_session(db, &pr, &pr_json, &session.local_repo_path)?;
    write_imported_review_state(db, &next_session)?;
    Ok(next_session)
}

#[tauri::command]
pub fn discard_pending_github_review_changes(
    session_id: String,
    state: State<'_, AppState>,
) -> CommandResult<GitHubPrSession> {
    let db = &state.storage;
    let session = load_session_by_id(db, &session_id)?;
    discard_pending_annotations(db, &session)?;
    Ok(session)
}

#[tauri::command]
pub fn submit_github_pr_review(
    session_id: String,
    event: GitHubReviewEvent,
    summary: Option<String>,
    state: State<'_, AppState>,
) -> CommandResult<SubmitGitHubReviewResult> {
    let db = &state.storage;
    let session = load_session_by_id(db, &session_id)?;

    if matches!(
        event,
        GitHubReviewEvent::Approve | GitHubReviewEvent::RequestChanges
    ) && !session.author_login.is_empty()
        && session
            .author_login
            .eq_ignore_ascii_case(&session.viewer_login)
    {
        return Err(CommandError::InvalidArgument(
            "GitHub does not allow approving or requesting changes on your own pull request. Use Submit comments instead."
                .into(),
        ));
    }

    let mut pending_roots = Vec::new();
    let sidecars = list_session_sidecars(&session)?;
    for (file_path, sidecar_path) in &sidecars {
        let mut sidecar = SidecarFile::load(sidecar_path)?;
        let relative_path = file_path
            .strip_prefix(&session.worktree_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        for annotation in sidecar.annotations.iter_mut() {
            let sync_state = annotation
                .github
                .as_ref()
                .and_then(|metadata| metadata.sync_state.clone());

            if annotation.reply_to.is_none() && sync_state == Some(GitHubSyncState::PendingPublish)
            {
                if is_publishable(&session, &relative_path, annotation.anchor_line())? {
                    pending_roots.push((
                        relative_path.clone(),
                        annotation.clone(),
                        sidecar_path.clone(),
                    ));
                } else {
                    let metadata = annotation.github.get_or_insert(GitHubAnnotationMetadata {
                        sync_state: Some(GitHubSyncState::LocalOnly),
                        external_comment_id: None,
                        external_thread_id: None,
                        publishable_reason: Some(
                            "Line is not part of the PR head-side diff".into(),
                        ),
                    });
                    metadata.sync_state = Some(GitHubSyncState::LocalOnly);
                    metadata.publishable_reason =
                        Some("Line is not part of the PR head-side diff".into());
                }
            }
        }

        sidecar.save(sidecar_path)?;
    }

    let comments_payload: Vec<Value> = pending_roots
        .iter()
        .map(|(path, annotation, _)| {
            serde_json::json!({
                "path": path,
                "body": annotation.body,
                "line": annotation.anchor_line(),
                "side": "RIGHT",
            })
        })
        .collect();

    let review_body = serde_json::json!({
        "body": summary.unwrap_or_default(),
        "event": match event {
            GitHubReviewEvent::Comment => "COMMENT",
            GitHubReviewEvent::Approve => "APPROVE",
            GitHubReviewEvent::RequestChanges => "REQUEST_CHANGES",
        },
        "comments": comments_payload,
    });

    let review_request_path = session_directory(&session)?.join("submit-review.json");
    fs::write(
        &review_request_path,
        serde_json::to_string_pretty(&review_body).map_err(CommandError::Json)?,
    )?;

    let review_response = run_gh_json(
        [
            "api",
            "-X",
            "POST",
            &format!("repos/{}/pulls/{}/reviews", session.repo, session.number),
            "--input",
            review_request_path.to_string_lossy().as_ref(),
        ],
        Some(&session.local_repo_path),
    )?;

    let mut published_comment_ids = review_response
        .get("comments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter();

    for (_path, annotation, sidecar_path) in pending_roots {
        let mut sidecar = SidecarFile::load(&sidecar_path)?;
        if let Some(current) = sidecar.get_annotation_mut(&annotation.id) {
            let response_comment_id = published_comment_ids
                .next()
                .and_then(|value| value.get("id").and_then(Value::as_i64))
                .map(|id| id.to_string());
            let metadata = current.github.get_or_insert(GitHubAnnotationMetadata {
                sync_state: Some(GitHubSyncState::Published),
                external_comment_id: None,
                external_thread_id: None,
                publishable_reason: None,
            });
            metadata.sync_state = Some(GitHubSyncState::Published);
            metadata.external_comment_id = response_comment_id;
            metadata.publishable_reason = None;
        }
        sidecar.save(&sidecar_path)?;
    }

    let pending_replies = collect_pending_replies(&sidecars)?;
    let mut reply_count = 0usize;
    for (reply, sidecar_path) in pending_replies {
        let parent_id = find_parent_external_comment_id(&sidecars, &reply)?;
        let response = run_gh_json(
            [
                "api",
                "-X",
                "POST",
                &format!(
                    "repos/{}/pulls/comments/{}/replies",
                    session.repo, parent_id
                ),
                "-f",
                &format!("body={}", reply.body),
            ],
            Some(&session.local_repo_path),
        )?;
        let mut sidecar = SidecarFile::load(&sidecar_path)?;
        if let Some(current) = sidecar.get_annotation_mut(&reply.id) {
            let metadata = current.github.get_or_insert(GitHubAnnotationMetadata {
                sync_state: Some(GitHubSyncState::Published),
                external_comment_id: None,
                external_thread_id: None,
                publishable_reason: None,
            });
            metadata.sync_state = Some(GitHubSyncState::Published);
            metadata.external_comment_id = extract_comment_id(&response);
            metadata.publishable_reason = None;
        }
        sidecar.save(&sidecar_path)?;
        reply_count += 1;
    }

    db.complete_review_session(
        &session_id,
        match event {
            GitHubReviewEvent::Comment => "commented",
            GitHubReviewEvent::Approve => "approved",
            GitHubReviewEvent::RequestChanges => "changes_requested",
        },
    )
    .map_err(storage_error)?;

    let refreshed = load_session_by_id(db, &session_id)?;
    Ok(SubmitGitHubReviewResult {
        session: refreshed,
        published_count: review_body
            .get("comments")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
        reply_count,
    })
}

pub fn resolve_github_session_for_file(
    db: &StateDb,
    file_path: &Path,
) -> CommandResult<Option<GitHubPrSession>> {
    db.find_github_session_for_file(file_path)
        .map_err(storage_error)?
        .map(github_session_from_stored)
        .transpose()
}

pub fn load_session_sidecar_for_file(
    session: &GitHubPrSession,
    file_path: &Path,
) -> CommandResult<SidecarFile> {
    let path = session_sidecar_path(session, file_path)?;
    if path.exists() {
        let mut sidecar = SidecarFile::load(&path)?;
        sidecar.source_file_hash = hash_file(file_path)?;
        Ok(sidecar)
    } else {
        Ok(SidecarFile::new(hash_file(file_path)?))
    }
}

pub fn save_session_sidecar_for_file(
    db: &StateDb,
    session: &GitHubPrSession,
    file_path: &Path,
    sidecar: &SidecarFile,
) -> CommandResult<()> {
    let path = session_sidecar_path(session, file_path)?;
    if sidecar.annotations.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        db.delete_session_file(&session.id, &file_path.to_string_lossy())
            .map_err(storage_error)?;
        return Ok(());
    }
    sidecar.save(&path)?;
    db.upsert_session_file(
        &session.id,
        &indexed_session_file(session, file_path, sidecar),
    )
    .map_err(storage_error)?;
    Ok(())
}

pub fn collect_session_annotations(
    session: &GitHubPrSession,
) -> CommandResult<Vec<redpen_core::annotation::FileAnnotations>> {
    let mut results = Vec::new();
    for (file_path, sidecar_path) in list_session_sidecars(session)? {
        let sidecar = SidecarFile::load(&sidecar_path)?;
        if sidecar.annotations.is_empty() {
            continue;
        }
        results.push(redpen_core::annotation::FileAnnotations {
            file_path: file_path.to_string_lossy().to_string(),
            file_name: file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            annotations: sidecar.annotations,
        });
    }
    Ok(results)
}

fn parse_pr_ref(pr_ref: &str, tracked: &[TrackedRepo]) -> CommandResult<PrRef> {
    if let Some(number) = pr_ref
        .strip_prefix('#')
        .and_then(|value| value.parse::<u32>().ok())
    {
        let repo = tracked
            .first()
            .map(|tracked_repo| tracked_repo.repo.clone())
            .ok_or_else(|| {
                CommandError::InvalidArgument(
                    "PR number without repo requires a tracked repo or explicit owner/repo".into(),
                )
            })?;
        return Ok(PrRef { repo, number });
    }

    if let Some(rest) = pr_ref.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 4 && parts[2] == "pull" {
            return Ok(PrRef {
                repo: format!("{}/{}", parts[0], parts[1]),
                number: parts[3]
                    .parse::<u32>()
                    .map_err(|_| CommandError::InvalidArgument("Invalid PR URL".into()))?,
            });
        }
    }

    if let Some((repo, number)) = pr_ref.rsplit_once('#') {
        return Ok(PrRef {
            repo: repo.trim().to_string(),
            number: number
                .parse::<u32>()
                .map_err(|_| CommandError::InvalidArgument("Invalid PR reference".into()))?,
        });
    }

    Err(CommandError::InvalidArgument(
        "Expected a GitHub PR URL or owner/repo#number".into(),
    ))
}

fn resolve_local_repo_path(
    repo: &str,
    local_path_hint: Option<String>,
    tracked: &[TrackedRepo],
    default_checkout_root: Option<String>,
) -> CommandResult<String> {
    if let Some(local_path_hint) = normalize_optional_path(local_path_hint) {
        ensure_local_repo(repo, &local_path_hint, true)?;
        return Ok(local_path_hint);
    }

    if let Some(tracked_repo) = tracked
        .iter()
        .find(|tracked_repo| tracked_repo.repo.eq_ignore_ascii_case(repo))
    {
        ensure_local_repo(repo, &tracked_repo.local_path, true)?;
        return Ok(tracked_repo.local_path.clone());
    }

    if let Some(checkout_root) = default_checkout_root {
        let local_path = default_checkout_path(&checkout_root, repo)?;
        ensure_local_repo(repo, &local_path, true)?;
        return Ok(local_path);
    }

    Err(CommandError::InvalidArgument(format!(
        "No local path configured for {} and no default checkout location is set.",
        repo
    )))
}

fn ensure_repo_is_tracked(state: &AppState, repo: &str, local_path: &str) -> CommandResult<()> {
    let updated_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| CommandError::InvalidArgument(format!("settings lock poisoned: {e}")))?;
        if settings
            .tracked_github_repos
            .iter()
            .any(|tracked_repo| tracked_repo.repo.eq_ignore_ascii_case(repo))
        {
            return Ok(());
        }
        let mut next_settings = settings.clone();
        next_settings.tracked_github_repos.push(TrackedRepo {
            repo: repo.to_string(),
            local_path: local_path.to_string(),
        });
        next_settings.tracked_github_repos =
            normalize_tracked_repos(next_settings.tracked_github_repos);
        next_settings
    };

    updated_settings
        .save_to_path(&state.settings_path)
        .map_err(CommandError::InvalidArgument)?;
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| CommandError::InvalidArgument(format!("settings lock poisoned: {e}")))?;
    *settings = updated_settings;
    Ok(())
}

fn default_checkout_path(checkout_root: &str, repo: &str) -> CommandResult<String> {
    let (owner, name) = repo
        .split_once('/')
        .ok_or_else(|| CommandError::InvalidArgument("Invalid repo owner/name".into()))?;
    Ok(Path::new(checkout_root)
        .join(owner)
        .join(name)
        .to_string_lossy()
        .to_string())
}

fn ensure_local_repo(repo: &str, local_path: &str, allow_clone: bool) -> CommandResult<()> {
    let repo_path = Path::new(local_path);
    if repo_path.exists() {
        Repository::open(repo_path).map_err(|_| {
            CommandError::InvalidArgument(format!(
                "{} exists but is not a git repository: {}",
                repo, local_path
            ))
        })?;
        return Ok(());
    }

    if !allow_clone {
        return Err(CommandError::InvalidArgument(format!(
            "Local repository path does not exist: {}",
            local_path
        )));
    }

    clone_repo(repo, repo_path)
}

fn clone_repo(repo: &str, destination: &Path) -> CommandResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let url = format!("https://github.com/{repo}.git");
    let destination_str = destination.to_string_lossy().to_string();
    run_git(["clone", url.as_str(), destination_str.as_str()], None)?;
    Ok(())
}

fn load_pr_view(pr: &PrRef, local_repo_path: &str) -> CommandResult<PrViewJson> {
    let output = run_gh_json(
        [
            "pr",
            "view",
            &pr.number.to_string(),
            "--repo",
            pr.repo.as_str(),
            "--json",
            "title,author,body,url,baseRefName,baseRefOid,headRefName,headRefOid",
        ],
        Some(local_repo_path),
    )?;

    serde_json::from_value(output).map_err(CommandError::Json)
}

fn load_viewer_login(local_repo_path: &str) -> CommandResult<String> {
    let output = run_gh_json(["api", "user"], Some(local_repo_path))?;
    output
        .get("login")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .ok_or_else(|| {
            CommandError::InvalidArgument("Could not determine current GitHub user".into())
        })
}

fn ensure_worktree_and_session(
    db: &StateDb,
    pr: &PrRef,
    pr_json: &PrViewJson,
    local_repo_path: &str,
) -> CommandResult<GitHubPrSession> {
    let worktree_path = checkouts_root().map_err(storage_error)?.join(format!(
        "{}-pr-{}-{}",
        pr.repo.replace('/', "-"),
        pr.number,
        short_sha(&pr_json.head_ref_oid)
    ));
    if let Some(parent) = worktree_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let fetch_ref = format!("refs/pull/{}/head", pr.number);
    let _ = run_git(
        [
            "fetch",
            "origin",
            fetch_ref.as_str(),
            pr_json.head_ref_oid.as_str(),
        ],
        Some(local_repo_path),
    );

    if !worktree_path.exists() {
        run_git(
            [
                "worktree",
                "add",
                "--detach",
                worktree_path.to_string_lossy().as_ref(),
                pr_json.head_ref_oid.as_str(),
            ],
            Some(local_repo_path),
        )?;
    }

    let viewer_login = load_viewer_login(local_repo_path)?;

    let session = GitHubPrSession {
        id: format!("{}-{}", pr.repo.replace('/', "__"), pr.number),
        repo: pr.repo.clone(),
        number: pr.number,
        title: pr_json.title.clone(),
        author_login: pr_json.author.login.clone(),
        viewer_login,
        body: pr_json.body.clone(),
        url: pr_json.url.clone(),
        base_ref: pr_json.base_ref_name.clone(),
        base_sha: pr_json.base_ref_oid.clone(),
        head_ref: pr_json.head_ref_name.clone(),
        head_sha: pr_json.head_ref_oid.clone(),
        local_repo_path: local_repo_path.to_string(),
        worktree_path: worktree_path.to_string_lossy().to_string(),
        changed_files: list_changed_files(
            local_repo_path,
            &pr_json.base_ref_oid,
            &pr_json.head_ref_oid,
        )?,
        updated_at: Utc::now().to_rfc3339(),
    };
    save_session(db, &session, ReviewSessionStatus::Active)?;
    Ok(session)
}

fn write_imported_review_state(db: &StateDb, session: &GitHubPrSession) -> CommandResult<()> {
    let imported = import_review_threads(session)?;
    let existing_sidecars = list_session_sidecars(session)?;
    let mut by_file: HashMap<PathBuf, Vec<Annotation>> = HashMap::new();
    for annotation in imported {
        let relative_path = annotation
            .github
            .as_ref()
            .and_then(|metadata| metadata.publishable_reason.clone())
            .map(PathBuf::from)
            .ok_or_else(|| {
                CommandError::InvalidArgument("Imported annotation missing path".into())
            })?;
        let absolute_path = Path::new(&session.worktree_path).join(relative_path);
        by_file.entry(absolute_path).or_default().push(annotation);
    }

    let mut all_files = HashSet::new();
    for (file_path, _) in &existing_sidecars {
        all_files.insert(file_path.clone());
    }
    for file_path in by_file.keys() {
        all_files.insert(file_path.clone());
    }

    for file_path in all_files {
        let mut kept = existing_sidecars
            .iter()
            .find(|(existing_file, _)| existing_file == &file_path)
            .map(|(_, sidecar_path)| SidecarFile::load(sidecar_path))
            .transpose()?
            .unwrap_or_else(|| SidecarFile::new(hash_file(&file_path).unwrap_or_default()));

        kept.annotations.retain(|annotation| {
            annotation
                .github
                .as_ref()
                .and_then(|metadata| metadata.sync_state.clone())
                != Some(GitHubSyncState::Imported)
        });

        let existing_external_ids: HashSet<String> = kept
            .annotations
            .iter()
            .filter_map(|annotation| {
                annotation
                    .github
                    .as_ref()
                    .and_then(|metadata| metadata.external_comment_id.clone())
            })
            .collect();

        for annotation in by_file.remove(&file_path).unwrap_or_default() {
            let external_id = annotation
                .github
                .as_ref()
                .and_then(|metadata| metadata.external_comment_id.clone());
            if external_id
                .as_ref()
                .is_some_and(|id| existing_external_ids.contains(id))
            {
                continue;
            }
            kept.annotations.push(annotation);
        }

        save_session_sidecar_for_file(db, session, &file_path, &kept)?;
    }

    Ok(())
}

fn import_review_threads(session: &GitHubPrSession) -> CommandResult<Vec<Annotation>> {
    let (owner, name) = session
        .repo
        .split_once('/')
        .ok_or_else(|| CommandError::InvalidArgument("Invalid repo owner/name".into()))?;

    let response = run_gh_json(
        [
            "api",
            "graphql",
            "-f",
            &format!("query={}", REVIEW_THREADS_QUERY),
            "-F",
            &format!("owner={owner}"),
            "-F",
            &format!("name={name}"),
            "-F",
            &format!("number={}", session.number),
        ],
        Some(&session.local_repo_path),
    )?;

    let threads = response
        .pointer("/data/repository/pullRequest/reviewThreads/nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut annotations = Vec::new();
    for thread in threads {
        let thread_id = thread.get("id").and_then(Value::as_str).unwrap_or_default();
        let resolved = thread
            .get("isResolved")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let comments = thread
            .pointer("/comments/nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut github_to_local_id = HashMap::new();
        for comment in comments {
            let path = comment
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let line = comment
                .get("line")
                .and_then(Value::as_u64)
                .or_else(|| comment.get("originalLine").and_then(Value::as_u64))
                .unwrap_or(1) as u32;
            let absolute_path = Path::new(&session.worktree_path).join(path);
            let anchor = build_anchor_for_file(&absolute_path, line)?;
            let github_comment_id = comment
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let reply_to_github = comment
                .get("replyTo")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .map(ToString::to_string);

            let mut annotation = Annotation::new(
                AnnotationKind::Comment,
                comment
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                vec![],
                comment
                    .get("author")
                    .and_then(|value| value.get("login"))
                    .and_then(Value::as_str)
                    .unwrap_or("github")
                    .to_string(),
                anchor,
            );
            annotation.created_at = comment
                .get("createdAt")
                .and_then(Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            annotation.updated_at = comment
                .get("updatedAt")
                .and_then(Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            annotation.reply_to = reply_to_github
                .as_ref()
                .and_then(|github_id| github_to_local_id.get(github_id))
                .cloned();
            annotation.resolved = resolved;
            annotation.github = Some(GitHubAnnotationMetadata {
                sync_state: Some(GitHubSyncState::Imported),
                external_comment_id: Some(github_comment_id.clone()),
                external_thread_id: Some(thread_id.to_string()),
                publishable_reason: Some(path.to_string()),
            });
            github_to_local_id.insert(github_comment_id, annotation.id.clone());
            annotations.push(annotation);
        }
    }

    Ok(annotations)
}

fn has_pending_local_annotations(session: &GitHubPrSession) -> CommandResult<bool> {
    for (_, sidecar_path) in list_session_sidecars(session)? {
        let sidecar = SidecarFile::load(&sidecar_path)?;
        if sidecar.annotations.iter().any(|annotation| {
            annotation
                .github
                .as_ref()
                .and_then(|metadata| metadata.sync_state.clone())
                == Some(GitHubSyncState::PendingPublish)
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn discard_pending_annotations(db: &StateDb, session: &GitHubPrSession) -> CommandResult<()> {
    for (file_path, sidecar_path) in list_session_sidecars(session)? {
        let mut sidecar = SidecarFile::load(&sidecar_path)?;
        sidecar.annotations.retain(|annotation| {
            annotation
                .github
                .as_ref()
                .and_then(|metadata| metadata.sync_state.clone())
                != Some(GitHubSyncState::PendingPublish)
                && annotation
                    .github
                    .as_ref()
                    .and_then(|metadata| metadata.sync_state.clone())
                    != Some(GitHubSyncState::LocalOnly)
        });
        save_session_sidecar_for_file(db, session, &file_path, &sidecar)?;
    }
    Ok(())
}

fn collect_pending_replies(
    sidecars: &[(PathBuf, PathBuf)],
) -> CommandResult<Vec<(Annotation, PathBuf)>> {
    let mut pending_replies = Vec::new();

    for (_, sidecar_path) in sidecars {
        let mut sidecar = SidecarFile::load(sidecar_path)?;
        let root_lookup: HashMap<String, Annotation> = sidecar
            .annotations
            .iter()
            .filter(|annotation| annotation.reply_to.is_none())
            .map(|annotation| (annotation.id.clone(), annotation.clone()))
            .collect();
        let mut changed = false;

        for annotation in sidecar.annotations.iter_mut() {
            let sync_state = annotation
                .github
                .as_ref()
                .and_then(|metadata| metadata.sync_state.clone());
            if annotation.reply_to.is_none() || sync_state != Some(GitHubSyncState::PendingPublish)
            {
                continue;
            }

            let Some(reply_to) = &annotation.reply_to else {
                continue;
            };
            let Some(root_annotation) = root_lookup.get(reply_to) else {
                continue;
            };

            let root_metadata = root_annotation.github.as_ref();
            let root_state = root_metadata.and_then(|metadata| metadata.sync_state.clone());
            let root_comment_id =
                root_metadata.and_then(|metadata| metadata.external_comment_id.clone());

            if root_comment_id.is_some()
                && matches!(
                    root_state,
                    Some(GitHubSyncState::Imported) | Some(GitHubSyncState::Published)
                )
            {
                pending_replies.push((annotation.clone(), sidecar_path.clone()));
                continue;
            }

            if root_state == Some(GitHubSyncState::LocalOnly) {
                let metadata = annotation.github.get_or_insert(GitHubAnnotationMetadata {
                    sync_state: Some(GitHubSyncState::LocalOnly),
                    external_comment_id: None,
                    external_thread_id: None,
                    publishable_reason: Some("Parent thread is not publishable to GitHub".into()),
                });
                metadata.sync_state = Some(GitHubSyncState::LocalOnly);
                metadata.publishable_reason =
                    Some("Parent thread is not publishable to GitHub".into());
                changed = true;
            }
        }

        if changed {
            sidecar.save(sidecar_path)?;
        }
    }

    Ok(pending_replies)
}

fn find_parent_external_comment_id(
    sidecars: &[(PathBuf, PathBuf)],
    reply: &Annotation,
) -> CommandResult<String> {
    let Some(parent_id) = &reply.reply_to else {
        return Err(CommandError::InvalidArgument(
            "Reply is missing reply_to".into(),
        ));
    };
    for (_, sidecar_path) in sidecars {
        let sidecar = SidecarFile::load(sidecar_path)?;
        if let Some(parent) = sidecar.get_annotation(parent_id) {
            if let Some(external_comment_id) = parent
                .github
                .as_ref()
                .and_then(|metadata| metadata.external_comment_id.clone())
            {
                return Ok(external_comment_id);
            }
        }
    }
    Err(CommandError::NotFound(
        "Could not resolve parent GitHub comment ID".into(),
    ))
}

fn extract_comment_id(value: &Value) -> Option<String> {
    value.get("id").and_then(|id| {
        id.as_str()
            .map(ToString::to_string)
            .or_else(|| id.as_i64().map(|id| id.to_string()))
            .or_else(|| id.as_u64().map(|id| id.to_string()))
    })
}

fn is_publishable(
    session: &GitHubPrSession,
    relative_path: &str,
    line: u32,
) -> CommandResult<bool> {
    let output = run_git(
        [
            "diff",
            "--unified=0",
            session.base_sha.as_str(),
            session.head_sha.as_str(),
            "--",
            relative_path,
        ],
        Some(&session.worktree_path),
    )?;

    for hunk in output.lines().filter(|line| line.starts_with("@@")) {
        if let Some(range) = parse_new_range(hunk) {
            if line >= range.0 && line < range.0 + range.1.max(1) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn parse_new_range(hunk: &str) -> Option<(u32, u32)> {
    let plus = hunk.split_whitespace().find(|part| part.starts_with('+'))?;
    let range = plus.trim_start_matches('+');
    let (start, count) = range.split_once(',').unwrap_or((range, "1"));
    Some((start.parse().ok()?, count.parse().ok()?))
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(7).collect()
}

fn list_changed_files(
    local_repo_path: &str,
    base_sha: &str,
    head_sha: &str,
) -> CommandResult<Vec<String>> {
    let output = run_git(
        ["diff", "--name-only", base_sha, head_sha],
        Some(local_repo_path),
    )?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

fn run_gh_json<const N: usize>(args: [&str; N], cwd: Option<&str>) -> CommandResult<Value> {
    let output = run_process("gh", &args, cwd)?;
    serde_json::from_str(&output).map_err(CommandError::Json)
}

fn run_git<const N: usize>(args: [&str; N], cwd: Option<&str>) -> CommandResult<String> {
    run_process("git", &args, cwd)
}

fn run_process<const N: usize>(
    program: &str,
    args: &[&str; N],
    cwd: Option<&str>,
) -> CommandResult<String> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().map_err(CommandError::Io)?;
    if !output.status.success() {
        return Err(CommandError::Process(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn build_anchor_for_file(file_path: &Path, line: u32) -> CommandResult<Anchor> {
    let content = fs::read_to_string(file_path)?;
    let source_lines: Vec<&str> = content.lines().collect();
    let line_idx = (line as usize).saturating_sub(1);
    let line_content = source_lines.get(line_idx).unwrap_or(&"").to_string();
    let start = line_idx.saturating_sub(2);
    let end = (line_idx + 3).min(source_lines.len());
    let surrounding_lines: Vec<String> = source_lines[start..end]
        .iter()
        .map(|value| value.to_string())
        .collect();

    Ok(Anchor::TextContext {
        line_content: line_content.clone(),
        surrounding_lines,
        content_hash: hash_string(&line_content),
        range: Range {
            start_line: line,
            start_column: 0,
            end_line: line,
            end_column: line_content.len() as u32,
        },
        last_known_line: line,
    })
}

fn session_directory(session: &GitHubPrSession) -> CommandResult<PathBuf> {
    Ok(sessions_root().map_err(storage_error)?.join(&session.id))
}

fn session_sidecar_path(session: &GitHubPrSession, file_path: &Path) -> CommandResult<PathBuf> {
    let relative = file_path
        .strip_prefix(&session.worktree_path)
        .map_err(|_| CommandError::InvalidArgument("File is not inside PR worktree".into()))?;
    let file_name = relative.file_name().unwrap_or_default().to_string_lossy();
    Ok(session_directory(session)?
        .join("comments")
        .join(relative.with_file_name(format!("{file_name}.json"))))
}

fn save_session(
    db: &StateDb,
    session: &GitHubPrSession,
    status: ReviewSessionStatus,
) -> CommandResult<()> {
    fs::create_dir_all(session_directory(session)?)?;
    db.upsert_review_session(&stored_session_from_github(session, status))
        .map_err(storage_error)?;
    let indexed_files = session
        .changed_files
        .iter()
        .map(|relative_path| IndexedSessionFile {
            file_path: Path::new(&session.worktree_path)
                .join(relative_path)
                .to_string_lossy()
                .to_string(),
            relative_path: Some(relative_path.clone()),
            annotation_count: 0,
            pending_count: 0,
            resolved_count: 0,
        })
        .collect::<Vec<_>>();
    db.replace_session_files(&session.id, &indexed_files)
        .map_err(storage_error)?;
    Ok(())
}

fn load_session_by_id(db: &StateDb, session_id: &str) -> CommandResult<GitHubPrSession> {
    let stored = db
        .get_review_session(session_id)
        .map_err(storage_error)?
        .ok_or_else(|| {
            CommandError::NotFound(format!("GitHub review session {} not found", session_id))
        })?;
    github_session_from_stored(stored)
}

fn list_session_sidecars(session: &GitHubPrSession) -> CommandResult<Vec<(PathBuf, PathBuf)>> {
    let comments_dir = session_directory(session)?.join("comments");
    if !comments_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sidecars = Vec::new();
    for entry in walk_dir_files(&comments_dir)? {
        if entry
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let relative = entry.strip_prefix(&comments_dir).map_err(|_| {
                CommandError::InvalidArgument("Invalid session sidecar path".into())
            })?;
            let source_relative = relative.with_file_name(
                relative
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .trim_end_matches(".json"),
            );
            sidecars.push((
                Path::new(&session.worktree_path).join(source_relative),
                entry,
            ));
        }
    }
    Ok(sidecars)
}

fn walk_dir_files(base: &Path) -> CommandResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !base.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_dir_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

fn storage_error(error: crate::storage::StorageError) -> CommandError {
    CommandError::InvalidArgument(error.to_string())
}

fn stored_session_from_github(
    session: &GitHubPrSession,
    status: ReviewSessionStatus,
) -> StoredReviewSession {
    StoredReviewSession {
        id: session.id.clone(),
        kind: ReviewSessionKind::GitHubPr,
        status,
        repo: Some(session.repo.clone()),
        pr_number: Some(session.number),
        title: Some(session.title.clone()),
        body: Some(session.body.clone()),
        url: Some(session.url.clone()),
        local_repo_path: Some(session.local_repo_path.clone()),
        worktree_path: Some(session.worktree_path.clone()),
        primary_file_path: session.changed_files.first().map(|path| {
            Path::new(&session.worktree_path)
                .join(path)
                .to_string_lossy()
                .to_string()
        }),
        project_root: Some(session.worktree_path.clone()),
        author_login: Some(session.author_login.clone()),
        viewer_login: Some(session.viewer_login.clone()),
        base_ref: Some(session.base_ref.clone()),
        base_sha: Some(session.base_sha.clone()),
        head_ref: Some(session.head_ref.clone()),
        head_sha: Some(session.head_sha.clone()),
        changed_files: session.changed_files.clone(),
        verdict: None,
        created_at: session.updated_at.clone(),
        updated_at: session.updated_at.clone(),
        completed_at: None,
        file_count: session.changed_files.len(),
    }
}

fn github_session_from_stored(stored: StoredReviewSession) -> CommandResult<GitHubPrSession> {
    if stored.kind != ReviewSessionKind::GitHubPr {
        return Err(CommandError::InvalidArgument(format!(
            "Review session {} is not a GitHub PR session",
            stored.id
        )));
    }
    Ok(GitHubPrSession {
        id: stored.id,
        repo: stored.repo.ok_or_else(|| {
            CommandError::InvalidArgument("Stored review session is missing repo".into())
        })?,
        number: stored.pr_number.ok_or_else(|| {
            CommandError::InvalidArgument("Stored review session is missing PR number".into())
        })?,
        title: stored.title.unwrap_or_default(),
        author_login: stored.author_login.unwrap_or_default(),
        viewer_login: stored.viewer_login.unwrap_or_default(),
        body: stored.body.unwrap_or_default(),
        url: stored.url.unwrap_or_default(),
        base_ref: stored.base_ref.unwrap_or_default(),
        base_sha: stored.base_sha.unwrap_or_default(),
        head_ref: stored.head_ref.unwrap_or_default(),
        head_sha: stored.head_sha.unwrap_or_default(),
        local_repo_path: stored.local_repo_path.unwrap_or_default(),
        worktree_path: stored.worktree_path.unwrap_or_default(),
        changed_files: stored.changed_files,
        updated_at: stored.updated_at,
    })
}

fn indexed_session_file(
    session: &GitHubPrSession,
    file_path: &Path,
    sidecar: &SidecarFile,
) -> IndexedSessionFile {
    let relative_path = file_path
        .strip_prefix(&session.worktree_path)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    IndexedSessionFile {
        file_path: file_path.to_string_lossy().to_string(),
        relative_path,
        annotation_count: sidecar.annotations.len(),
        pending_count: sidecar
            .annotations
            .iter()
            .filter(|annotation| {
                annotation
                    .github
                    .as_ref()
                    .and_then(|metadata| metadata.sync_state.clone())
                    == Some(GitHubSyncState::PendingPublish)
            })
            .count(),
        resolved_count: sidecar
            .annotations
            .iter()
            .filter(|annotation| annotation.resolved)
            .count(),
    }
}

trait AnchorLine {
    fn anchor_line(&self) -> u32;
}

impl AnchorLine for Annotation {
    fn anchor_line(&self) -> u32 {
        match &self.anchor {
            Anchor::TextContext { range, .. } => range.start_line,
        }
    }
}

// ---------------------------------------------------------------------------
// CI / Check Runs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct CheckRun {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub details_url: Option<String>,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct CheckRunsResult {
    pub check_runs: Vec<CheckRun>,
    pub total_count: usize,
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
}

#[tauri::command]
pub fn get_pr_check_runs(repo: String, head_sha: String) -> CommandResult<CheckRunsResult> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/commits/{}/check-runs", repo, head_sha),
            "--jq",
            r#"{total_count: .total_count, check_runs: [.check_runs[] | {name: .name, status: .status, conclusion: .conclusion, started_at: .started_at, completed_at: .completed_at, details_url: .details_url, html_url: .html_url}]}"#,
        ])
        .output()
        .map_err(|e| CommandError::External(format!("failed to run gh: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CommandError::External(format!(
            "gh api check-runs failed: {}",
            stderr
        )));
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| CommandError::External(format!("failed to parse check-runs: {}", e)))?;

    let total_count = json["total_count"].as_u64().unwrap_or(0) as usize;
    let check_runs: Vec<CheckRun> = json["check_runs"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let passed = check_runs
        .iter()
        .filter(|c| c.conclusion.as_deref() == Some("success"))
        .count();
    let failed = check_runs
        .iter()
        .filter(|c| {
            matches!(
                c.conclusion.as_deref(),
                Some("failure") | Some("timed_out") | Some("cancelled")
            )
        })
        .count();
    let pending = check_runs
        .iter()
        .filter(|c| c.status != "completed")
        .count();

    Ok(CheckRunsResult {
        check_runs,
        total_count,
        passed,
        failed,
        pending,
    })
}
