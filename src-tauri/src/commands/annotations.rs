use crate::commands::error::{CommandError, CommandResult};
use crate::commands::github_review::{
    collect_session_annotations, load_session_sidecar_for_file, resolve_github_session_for_file,
    save_session_sidecar_for_file,
};
use crate::event_bus::TauriEventBus;
use crate::notification::{NotificationKind, NotificationService};
use crate::settings::{AppSettings, UpdateSettingsRequest};
use crate::state::AppState;
use redpen_core::annotation::{
    Anchor, Annotation, AnnotationKind, Choice, FileAnnotations, GitHubAnnotationMetadata,
    GitHubSyncState, Range,
};
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use redpen_runtime::annotations::AnnotationService;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub file_path: String,
    pub body: String,
    pub labels: Vec<String>,
    #[serde(default = "default_kind")]
    pub kind: AnnotationKind,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    #[serde(default)]
    pub reply_to: Option<String>,
}

fn default_kind() -> AnnotationKind {
    AnnotationKind::Comment
}

fn resolve_project_root(source_path: &Path) -> PathBuf {
    let fallback = || dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo
            .workdir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(fallback),
        Err(_) => fallback(),
    }
}

#[tauri::command]
pub fn get_annotations(
    file_path: String,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> CommandResult<SidecarFile> {
    let source_path = Path::new(&file_path);
    if let Some(session) = resolve_github_session_for_file(&state.storage, source_path)? {
        return load_session_sidecar_for_file(&session, source_path);
    }
    let project_root = resolve_project_root(source_path);
    svc.get_annotations_from_session(source_path, &project_root)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_all_annotations(
    root_folder: String,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> CommandResult<Vec<FileAnnotations>> {
    let root = Path::new(&root_folder);
    if let Some(session) = resolve_github_session_for_file(&state.storage, root)? {
        return collect_session_annotations(&session);
    }
    let project_root = resolve_project_root(root);
    svc.get_all_annotations_from_session(&project_root)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn create_annotation(
    request: CreateAnnotationRequest,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> CommandResult<Annotation> {
    let source_path = Path::new(&request.file_path);
    let content = fs::read_to_string(source_path)?;
    let source_lines: Vec<&str> = content.lines().collect();

    let line_idx = (request.start_line as usize).saturating_sub(1);
    let line_content = source_lines.get(line_idx).unwrap_or(&"").to_string();

    let start = line_idx.saturating_sub(2);
    let end = (line_idx + 3).min(source_lines.len());
    let surrounding_lines: Vec<String> = source_lines[start..end]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let range = Range {
        start_line: request.start_line,
        start_column: request.start_column,
        end_line: request.end_line,
        end_column: request.end_column,
    };

    let anchor = Anchor::TextContext {
        line_content: line_content.clone(),
        surrounding_lines,
        content_hash: hash_string(&line_content),
        range,
        last_known_line: request.start_line,
    };
    let kind = request.kind.clone();
    let reply_to = request.reply_to.clone();

    let author = state
        .settings
        .lock()
        .map_err(|e| CommandError::InvalidArgument(format!("settings lock poisoned: {e}")))?
        .author
        .clone();
    if let Some(session) = resolve_github_session_for_file(&state.storage, source_path)? {
        let mut sidecar = load_session_sidecar_for_file(&session, source_path)?;
        let mut annotation = if let Some(reply_to) = reply_to.clone() {
            Annotation::new_reply(request.body.clone(), author, reply_to, anchor)
        } else {
            Annotation::new(
                kind.clone(),
                request.body.clone(),
                request.labels.clone(),
                author,
                anchor,
            )
        };
        annotation.github = Some(GitHubAnnotationMetadata {
            sync_state: Some(GitHubSyncState::PendingPublish),
            external_comment_id: None,
            external_thread_id: None,
            publishable_reason: None,
        });
        sidecar.add_annotation(annotation.clone());
        save_session_sidecar_for_file(&state.storage, &session, source_path, &sidecar)?;
        return Ok(annotation);
    }
    let project_root = resolve_project_root(source_path);
    svc.create_annotation_in_session(
        &project_root,
        source_path,
        kind,
        &request.body,
        request.labels,
        &author,
        anchor,
        reply_to,
    )
    .map_err(CommandError::from)
}

#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct FileSnippet {
    pub lines: Vec<String>,
    pub start_line: u32,
    pub total_lines: u32,
}

#[tauri::command]
pub fn read_file_lines(
    file_path: String,
    center_line: u32,
    context: u32,
) -> Result<FileSnippet, String> {
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len() as u32;

    let start = if center_line <= context + 1 {
        0u32
    } else {
        center_line - context - 1
    };
    let end = ((center_line + context) as usize).min(all_lines.len());

    let lines: Vec<String> = all_lines[start as usize..end]
        .iter()
        .map(|l| l.to_string())
        .collect();

    Ok(FileSnippet {
        lines,
        start_line: start + 1,
        total_lines,
    })
}

#[tauri::command]
pub fn update_annotation(
    file_path: String,
    annotation_id: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
    choices: Option<Vec<Choice>>,
    resolved: Option<bool>,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> CommandResult<Annotation> {
    let source_path = Path::new(&file_path);
    if let Some(session) = resolve_github_session_for_file(&state.storage, source_path)? {
        let mut sidecar = load_session_sidecar_for_file(&session, source_path)?;
        let annotation = sidecar
            .get_annotation_mut(&annotation_id)
            .ok_or_else(|| CommandError::NotFound(format!("annotation {}", annotation_id)))?;
        if let Some(body) = body {
            annotation.body = body;
        }
        if let Some(labels) = labels {
            annotation.labels = labels;
        }
        if let Some(choices) = choices {
            annotation.choices = Some(choices);
        }
        if let Some(resolved) = resolved {
            annotation.resolved = resolved;
        }
        if let Some(metadata) = &mut annotation.github {
            if metadata.sync_state.is_none() {
                metadata.sync_state = Some(GitHubSyncState::PendingPublish);
            }
        }
        annotation.updated_at = Some(chrono::Utc::now());
        let updated = annotation.clone();
        save_session_sidecar_for_file(&state.storage, &session, source_path, &sidecar)?;
        return Ok(updated);
    }
    let project_root = resolve_project_root(source_path);
    svc.update_annotation_in_session(
        &project_root,
        source_path,
        &annotation_id,
        body.as_deref(),
        labels,
        choices,
        resolved,
    )
    .map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_annotation(
    file_path: String,
    annotation_id: String,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> CommandResult<()> {
    let source_path = Path::new(&file_path);
    if let Some(session) = resolve_github_session_for_file(&state.storage, source_path)? {
        let mut sidecar = load_session_sidecar_for_file(&session, source_path)?;
        sidecar
            .remove_annotation(&annotation_id)
            .ok_or_else(|| CommandError::NotFound(format!("annotation {}", annotation_id)))?;
        save_session_sidecar_for_file(&state.storage, &session, source_path, &sidecar)?;
        return Ok(());
    }
    let project_root = resolve_project_root(source_path);
    svc.delete_annotation_from_session(&project_root, source_path, &annotation_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn clear_annotations(
    file_path: String,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
) -> CommandResult<()> {
    let source_path = Path::new(&file_path);
    if let Some(session) = resolve_github_session_for_file(&state.storage, source_path)? {
        save_session_sidecar_for_file(
            &state.storage,
            &session,
            source_path,
            &SidecarFile::new(hash_file(source_path)?),
        )?;
        return Ok(());
    }
    let project_root = resolve_project_root(source_path);
    svc.clear_annotations_in_session(&project_root, source_path)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn update_settings(
    request: UpdateSettingsRequest,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let updated_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| format!("settings lock poisoned: {e}"))?;
        let mut next_settings = settings.clone();
        request.apply(&mut next_settings);
        next_settings
    };

    updated_settings.save_to_path(&state.settings_path)?;

    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|e| format!("settings lock poisoned: {e}"))?;
        *settings = updated_settings.clone();
    }

    state.workspace_index.refresh_all();

    Ok(updated_settings)
}

#[tauri::command]
pub fn signal_review_done(
    file_path: String,
    verdict: Option<String>,
    session_id: Option<String>,
    state: State<'_, AppState>,
    svc: State<'_, AnnotationService<TauriEventBus>>,
    app_handle: tauri::AppHandle,
) -> CommandResult<()> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    let verdict_str = verdict.as_deref().unwrap_or("approved");

    if let Some(session_id) = session_id.as_deref() {
        let _ =
            tauri::async_runtime::block_on(state.review_sessions.complete(session_id, verdict_str));
        let _ = state
            .storage
            .complete_review_session(session_id, verdict_str);
    }

    // Fire OS notification for review complete
    let settings = state.settings.lock().unwrap();
    let service = NotificationService::new(app_handle);
    let _ = service.send(
        NotificationKind::ReviewComplete,
        "Review complete",
        &format!("Verdict: {}", verdict_str),
        &settings,
    );
    drop(settings);

    // Also POST annotations from session
    let sidecar = svc
        .get_annotations_from_session(source_path, &project_root)
        .map_err(CommandError::from)?;
    let json = serde_json::to_string(&sidecar.annotations)?;
    let port = std::env::var("REDPEN_CHANNEL_PORT").unwrap_or_else(|_| "8789".to_string());
    let encoded_path: String = file_path
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                format!("{}", b as char)
            } else {
                format!("%{:02X}", b)
            }
        })
        .collect();
    // Fire and forget — channel may not be running
    std::thread::spawn(move || {
        use std::io::Write;
        use std::net::TcpStream;
        if let Ok(mut stream) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            let request = format!(
                "POST /?file={} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                encoded_path, port, json.len(), json
            );
            let _ = stream.write_all(request.as_bytes());
        }
    });

    Ok(())
}

#[tauri::command]
pub fn send_notification(
    kind: String,
    file_name: String,
    line: Option<u32>,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let notification_kind = match kind.as_str() {
        "annotation_reply" => NotificationKind::AnnotationReply,
        "review_complete" => NotificationKind::ReviewComplete,
        "new_annotation" => NotificationKind::NewAnnotation,
        "deep_link" => NotificationKind::DeepLink,
        _ => return Err(format!("Unknown notification kind: {}", kind)),
    };

    let (title, body) = notification_kind.default_title_body(&file_name, line);

    let settings = state.settings.lock().unwrap();
    let service = NotificationService::new(app_handle);
    service.send(notification_kind, &title, &body, &settings)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state
        .settings
        .lock()
        .map_err(|e| format!("settings lock poisoned: {e}"))?
        .clone())
}
