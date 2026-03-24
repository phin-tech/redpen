use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub base_ref: String,
    pub target_ref: String,
    pub hunks: Vec<DiffHunk>,
    pub old_content: String,
    pub new_content: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub changes: Vec<DiffChange>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffChange {
    pub kind: ChangeKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub content: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Equal,
    Insert,
    Delete,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RefList {
    pub branches: Vec<BranchInfo>,
    pub tags: Vec<String>,
    pub recent_commits: Vec<CommitInfo>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub sha: String,
    pub short_message: String,
}

#[tauri::command]
pub fn compute_diff(
    directory: String,
    file_path: String,
    base_ref: String,
    target_ref: String,
    algorithm: String,
) -> Result<DiffResult, String> {
    todo!()
}

#[tauri::command]
pub fn list_refs(directory: String) -> Result<RefList, String> {
    todo!()
}
