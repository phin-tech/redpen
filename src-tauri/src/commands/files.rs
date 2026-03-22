use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub has_sidecar: bool,
}

#[tauri::command]
pub fn read_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = PathBuf::from(&path);
    let project_root = match git2::Repository::discover(&dir) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    };
    let mut entries = Vec::new();
    let read_dir = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in read_dir.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let full_path = entry.path();
        let has_sidecar = if file_type.is_file() {
            let annotation_path = redpen_core::sidecar::SidecarFile::annotation_path(&project_root, &full_path);
            annotation_path.exists()
        } else {
            false
        };
        entries.push(FileEntry {
            name: file_name,
            path: full_path.to_string_lossy().to_string(),
            is_dir: file_type.is_dir(),
            has_sidecar,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}
