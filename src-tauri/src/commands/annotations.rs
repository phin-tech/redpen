use crate::commands::error::{CommandError, CommandResult};
use crate::notification::{NotificationKind, NotificationService};
use crate::settings::{AppSettings, UpdateSettingsRequest};
use crate::state::AppState;
use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind, FileAnnotations, Range};
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
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
}

fn default_kind() -> AnnotationKind {
    AnnotationKind::Comment
}

fn resolve_project_root(source_path: &Path) -> PathBuf {
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    }
}

fn load_sidecar_for_file(project_root: &Path, source_path: &Path) -> CommandResult<SidecarFile> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if annotation_path.exists() {
        let mut sidecar = SidecarFile::load(&annotation_path)?;
        let current_hash = hash_file(source_path)?;
        if sidecar.source_file_hash != current_hash {
            let content = fs::read_to_string(source_path)?;
            reanchor_annotations(&mut sidecar.annotations, &content);
            sidecar.source_file_hash = current_hash;
        }
        Ok(sidecar)
    } else {
        let hash = hash_file(source_path)?;
        Ok(SidecarFile::new(hash))
    }
}

fn save_sidecar(
    sidecar: &SidecarFile,
    project_root: &Path,
    source_path: &Path,
) -> CommandResult<()> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if sidecar.annotations.is_empty() {
        if annotation_path.exists() {
            fs::remove_file(&annotation_path)?;
        }
        Ok(())
    } else {
        sidecar.save(&annotation_path)?;
        Ok(())
    }
}

#[tauri::command]
pub fn get_annotations(file_path: String) -> CommandResult<SidecarFile> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    load_sidecar_for_file(&project_root, source_path)
}

#[tauri::command]
pub fn get_all_annotations(root_folder: String) -> CommandResult<Vec<FileAnnotations>> {
    let root = Path::new(&root_folder);
    let project_root = resolve_project_root(root);
    let comments_dir = project_root.join(".redpen").join("comments");

    if !comments_dir.exists() {
        return Ok(vec![]);
    }

    let mut results = Vec::new();
    collect_sidecar_files(&comments_dir, &project_root, &mut results)?;
    results.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    Ok(results)
}

fn collect_sidecar_files(
    dir: &Path,
    project_root: &Path,
    results: &mut Vec<FileAnnotations>,
) -> CommandResult<()> {
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_sidecar_files(&path, project_root, results)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            if let Ok(sidecar) = SidecarFile::load(&path) {
                if !sidecar.annotations.is_empty() {
                    // Reconstruct source path from sidecar path
                    let relative = path
                        .strip_prefix(project_root.join(".redpen").join("comments"))
                        .unwrap_or(&path);
                    // The sidecar file is named "filename.ext.json", so strip the trailing .json
                    let source_relative = relative.with_file_name(
                        relative
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .trim_end_matches(".json"),
                    );
                    let source_path = project_root.join(&source_relative);
                    let file_name = source_relative
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    results.push(FileAnnotations {
                        file_path: source_path.to_string_lossy().to_string(),
                        file_name,
                        annotations: sidecar.annotations,
                    });
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn create_annotation(
    request: CreateAnnotationRequest,
    state: State<'_, AppState>,
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

    let author = state.settings.lock().unwrap().author.clone();
    let annotation = Annotation::new(request.kind, request.body, request.labels, author, anchor);

    let project_root = resolve_project_root(source_path);
    let mut sidecar = load_sidecar_for_file(&project_root, source_path)?;
    sidecar.add_annotation(annotation.clone());
    save_sidecar(&sidecar, &project_root, source_path)?;
    Ok(annotation)
}

#[tauri::command]
pub fn update_annotation(
    file_path: String,
    annotation_id: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
) -> CommandResult<Annotation> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    let mut sidecar = load_sidecar_for_file(&project_root, source_path)?;
    let annotation = sidecar
        .get_annotation_mut(&annotation_id)
        .ok_or(CommandError::NotFound("Annotation not found".into()))?;
    if let Some(b) = body {
        annotation.body = b;
    }
    if let Some(l) = labels {
        annotation.labels = l;
    }
    annotation.updated_at = Some(chrono::Utc::now());
    let result = annotation.clone();
    save_sidecar(&sidecar, &project_root, source_path)?;
    Ok(result)
}

#[tauri::command]
pub fn delete_annotation(file_path: String, annotation_id: String) -> CommandResult<()> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    let mut sidecar = load_sidecar_for_file(&project_root, source_path)?;
    sidecar
        .remove_annotation(&annotation_id)
        .ok_or(CommandError::NotFound("Annotation not found".into()))?;
    save_sidecar(&sidecar, &project_root, source_path)?;
    Ok(())
}

#[tauri::command]
pub fn clear_annotations(file_path: String) -> CommandResult<()> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);
    let mut sidecar = load_sidecar_for_file(&project_root, source_path)?;
    sidecar.annotations.clear();
    save_sidecar(&sidecar, &project_root, source_path)?;
    Ok(())
}

#[tauri::command]
pub fn update_settings(
    request: UpdateSettingsRequest,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let updated_settings = {
        let settings = state.settings.lock().unwrap();
        let mut next_settings = settings.clone();
        request.apply(&mut next_settings);
        next_settings
    };

    updated_settings.save_to_path(&state.settings_path)?;

    {
        let mut settings = state.settings.lock().unwrap();
        *settings = updated_settings.clone();
    }

    state.workspace_index.refresh_all();

    Ok(updated_settings)
}

#[tauri::command]
pub fn signal_review_done(
    file_path: String,
    verdict: Option<String>,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> CommandResult<()> {
    let source_path = Path::new(&file_path);
    let project_root = resolve_project_root(source_path);

    // Read session ID (written by `redpen wait`) and include it in the signal
    let session_file = SidecarFile::session_file_path(&project_root);
    let session_id = fs::read_to_string(&session_file).unwrap_or_default();
    let verdict_str = verdict.as_deref().unwrap_or("approved");
    let signal_content = format!("{}\n{}", session_id.trim(), verdict_str);

    let signal_path = SidecarFile::session_signal_path(&project_root);
    if let Some(parent) = signal_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&signal_path, signal_content)?;

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

    // Also POST annotations to the redpen channel (if running)
    let sidecar = load_sidecar_for_file(&project_root, source_path)?;
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
    Ok(state.settings.lock().unwrap().clone())
}
