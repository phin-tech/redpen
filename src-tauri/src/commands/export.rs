use crate::commands::error::CommandResult;
use redpen_core::export::export_markdown;
use redpen_core::sidecar::SidecarFile;
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn export_annotations(file_path: String) -> CommandResult<String> {
    let source_path = Path::new(&file_path);
    let project_root = match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    };
    let annotation_path = SidecarFile::annotation_path(&project_root, source_path);
    if !annotation_path.exists() {
        return Ok(String::new());
    }
    let sidecar = SidecarFile::load(&annotation_path)?;
    let content = fs::read_to_string(source_path)?;
    let file_name = source_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    Ok(export_markdown(&sidecar, &content, &file_name))
}
