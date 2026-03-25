use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

#[tauri::command]
pub fn get_git_root(path: String) -> Result<Option<String>, String> {
    let p = Path::new(&path);
    match git2::Repository::discover(p) {
        Ok(repo) => {
            let workdir = repo.workdir().ok_or("bare repository")?;
            Ok(Some(
                workdir.to_string_lossy().trim_end_matches('/').to_string(),
            ))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub fn get_git_status(directory: String) -> Result<Vec<GitFileStatus>, String> {
    let dir_path = Path::new(&directory);
    let repo = match git2::Repository::discover(dir_path) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };
    let mut statuses_result = Vec::new();
    let statuses = repo
        .statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(false),
        ))
        .map_err(|e| e.to_string())?;
    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();
        let letter =
            if status.contains(git2::Status::WT_NEW) || status.contains(git2::Status::INDEX_NEW) {
                "?"
            } else if status.contains(git2::Status::WT_MODIFIED)
                || status.contains(git2::Status::INDEX_MODIFIED)
            {
                "M"
            } else if status.contains(git2::Status::WT_DELETED)
                || status.contains(git2::Status::INDEX_DELETED)
            {
                "D"
            } else if status.contains(git2::Status::WT_RENAMED)
                || status.contains(git2::Status::INDEX_RENAMED)
            {
                "R"
            } else {
                continue;
            };
        statuses_result.push(GitFileStatus {
            path,
            status: letter.to_string(),
        });
    }
    Ok(statuses_result)
}
