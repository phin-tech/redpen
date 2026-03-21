use crate::state::AppState;
use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind, Range};
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use std::fs;
use std::path::Path;
use tauri::State;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub file_path: String,
    pub kind: String,
    pub body: String,
    pub labels: Vec<String>,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

fn load_sidecar_for_file(source_path: &Path) -> Result<SidecarFile, String> {
    let sidecar_path = SidecarFile::sidecar_path(source_path);
    if sidecar_path.exists() {
        let mut sidecar = SidecarFile::load(&sidecar_path).map_err(|e| e.to_string())?;
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

fn save_sidecar(sidecar: &SidecarFile, source_path: &Path) -> Result<(), String> {
    sidecar
        .save_for_source(source_path)
        .map_err(|e| e.to_string())
}

fn parse_kind(kind: &str) -> Result<AnnotationKind, String> {
    match kind {
        "comment" => Ok(AnnotationKind::Comment),
        "lineNote" => Ok(AnnotationKind::LineNote),
        "label" => Ok(AnnotationKind::Label),
        other => Err(format!("Unknown kind: {}", other)),
    }
}

#[tauri::command]
pub fn get_annotations(file_path: String) -> Result<SidecarFile, String> {
    let source_path = Path::new(&file_path);
    load_sidecar_for_file(source_path)
}

#[tauri::command]
pub fn create_annotation(
    request: CreateAnnotationRequest,
    state: State<'_, AppState>,
) -> Result<Annotation, String> {
    let source_path = Path::new(&request.file_path);
    let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
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

    let author = state.author.lock().unwrap().clone();
    let annotation = Annotation::new(
        parse_kind(&request.kind)?,
        request.body,
        request.labels,
        author,
        anchor,
    );

    let mut sidecar = load_sidecar_for_file(source_path)?;
    sidecar.add_annotation(annotation.clone());
    save_sidecar(&sidecar, source_path)?;
    Ok(annotation)
}

#[tauri::command]
pub fn update_annotation(
    file_path: String,
    annotation_id: String,
    body: Option<String>,
    labels: Option<Vec<String>>,
) -> Result<Annotation, String> {
    let source_path = Path::new(&file_path);
    let mut sidecar = load_sidecar_for_file(source_path)?;
    let annotation = sidecar
        .get_annotation_mut(&annotation_id)
        .ok_or("Annotation not found")?;
    if let Some(b) = body {
        annotation.body = b;
    }
    if let Some(l) = labels {
        annotation.labels = l;
    }
    annotation.updated_at = Some(chrono::Utc::now());
    let result = annotation.clone();
    save_sidecar(&sidecar, source_path)?;
    Ok(result)
}

#[tauri::command]
pub fn delete_annotation(file_path: String, annotation_id: String) -> Result<(), String> {
    let source_path = Path::new(&file_path);
    let mut sidecar = load_sidecar_for_file(source_path)?;
    sidecar
        .remove_annotation(&annotation_id)
        .ok_or("Annotation not found")?;
    save_sidecar(&sidecar, source_path)?;
    Ok(())
}

#[tauri::command]
pub fn update_settings(
    author: Option<String>,
    default_labels: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(a) = author {
        *state.author.lock().unwrap() = a;
    }
    if let Some(l) = default_labels {
        *state.default_labels.lock().unwrap() = l;
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<(String, Vec<String>), String> {
    let author = state.author.lock().unwrap().clone();
    let labels = state.default_labels.lock().unwrap().clone();
    Ok((author, labels))
}
