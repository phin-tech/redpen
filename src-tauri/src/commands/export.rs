use redpen_core::export::export_markdown;
use redpen_core::sidecar::SidecarFile;
use std::fs;
use std::path::Path;

#[tauri::command]
pub fn export_annotations(file_path: String) -> Result<String, String> {
    let source_path = Path::new(&file_path);
    let sidecar_path = SidecarFile::sidecar_path(source_path);
    if !sidecar_path.exists() {
        return Ok(String::new());
    }
    let sidecar = SidecarFile::load(&sidecar_path).map_err(|e| e.to_string())?;
    let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
    let file_name = source_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    Ok(export_markdown(&sidecar, &content, &file_name))
}
