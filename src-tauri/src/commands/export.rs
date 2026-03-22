use redpen_core::export::export_markdown;
use redpen_core::sidecar::SidecarFile;
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn export_annotations(file_path: String) -> Result<String, String> {
    let source_path = Path::new(&file_path);
    let project_root = match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    };
    let annotation_path = SidecarFile::annotation_path(&project_root, source_path);
    if !annotation_path.exists() {
        return Ok(String::new());
    }
    let sidecar = SidecarFile::load(&annotation_path).map_err(|e| e.to_string())?;
    let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
    let file_name = source_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    Ok(export_markdown(&sidecar, &content, &file_name))
}
