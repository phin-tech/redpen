use crate::commands::error::{CommandError, CommandResult};
use git2::Repository;
use serde::Serialize;
use similar::{Algorithm, ChangeTag, TextDiff};
use std::path::Path;

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct DiffResult {
    pub base_ref: String,
    pub target_ref: String,
    pub hunks: Vec<DiffHunk>,
    pub old_content: String,
    pub new_content: String,
}

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub changes: Vec<DiffChange>,
}

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct DiffChange {
    pub kind: ChangeKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub content: String,
}

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum ChangeKind {
    Equal,
    Insert,
    Delete,
}

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct RefList {
    pub branches: Vec<BranchInfo>,
    pub tags: Vec<String>,
    pub recent_commits: Vec<CommitInfo>,
}

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct CommitInfo {
    pub sha: String,
    pub short_message: String,
}

fn get_file_at_ref(repo: &Repository, file_path: &str, git_ref: &str) -> CommandResult<String> {
    if git_ref == "working-tree" {
        let workdir = repo.workdir().ok_or(CommandError::NotFound(
            "bare repository has no working directory".into(),
        ))?;
        let full_path = workdir.join(file_path);
        return Ok(std::fs::read_to_string(&full_path)?);
    }

    let commit = if git_ref == "HEAD" {
        let head = repo.head()?;
        head.peel_to_commit()?
    } else {
        // Try local branch first
        if let Ok(branch) = repo.find_branch(git_ref, git2::BranchType::Local) {
            branch.get().peel_to_commit()?
        } else if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", git_ref)) {
            reference.peel_to_commit()?
        } else if let Ok(reference) = repo.find_reference(&format!("refs/heads/{}", git_ref)) {
            reference.peel_to_commit()?
        } else {
            // Try as raw SHA
            let oid = git2::Oid::from_str(git_ref)?;
            repo.find_commit(oid)?
        }
    };

    let tree = commit.tree()?;
    let entry = tree
        .get_path(Path::new(file_path))
        .map_err(|e| CommandError::NotFound(format!("file not found at ref {}: {}", git_ref, e)))?;
    let object = entry.to_object(repo)?;
    let blob = object
        .as_blob()
        .ok_or(CommandError::NotFound("not a blob".into()))?;
    std::str::from_utf8(blob.content())
        .map(|s| s.to_string())
        .map_err(|e| CommandError::InvalidArgument(format!("file is not valid UTF-8: {}", e)))
}

#[tauri::command]
pub fn compute_diff(
    directory: String,
    file_path: String,
    base_ref: String,
    target_ref: String,
    algorithm: String,
) -> CommandResult<DiffResult> {
    let repo = Repository::discover(&directory)?;

    // Resolve file_path relative to repo root
    let workdir = repo.workdir().ok_or(CommandError::NotFound(
        "bare repository has no working directory".into(),
    ))?;
    let abs_file = Path::new(&directory).join(&file_path);
    let rel_file = if abs_file.starts_with(workdir) {
        abs_file
            .strip_prefix(workdir)
            .map_err(|e| CommandError::InvalidArgument(e.to_string()))?
            .to_string_lossy()
            .to_string()
    } else {
        file_path.clone()
    };

    let old_content = get_file_at_ref(&repo, &rel_file, &base_ref).unwrap_or_default();
    let new_content = get_file_at_ref(&repo, &rel_file, &target_ref).unwrap_or_default();

    let algo = if algorithm == "myers" {
        Algorithm::Myers
    } else {
        Algorithm::Patience
    };

    let diff = TextDiff::configure()
        .algorithm(algo)
        .diff_lines(&old_content, &new_content);

    let mut hunks: Vec<DiffHunk> = Vec::new();

    for group in diff.grouped_ops(3) {
        let mut changes: Vec<DiffChange> = Vec::new();
        let mut old_start = u32::MAX;
        let mut old_end = 0u32;
        let mut new_start = u32::MAX;
        let mut new_end = 0u32;

        for op in &group {
            for change in diff.iter_changes(op) {
                let tag = change.tag();
                let old_idx = change.old_index().map(|i| i as u32 + 1);
                let new_idx = change.new_index().map(|i| i as u32 + 1);

                if let Some(ol) = old_idx {
                    if ol < old_start {
                        old_start = ol;
                    }
                    if ol > old_end {
                        old_end = ol;
                    }
                }
                if let Some(nl) = new_idx {
                    if nl < new_start {
                        new_start = nl;
                    }
                    if nl > new_end {
                        new_end = nl;
                    }
                }

                let kind = match tag {
                    ChangeTag::Equal => ChangeKind::Equal,
                    ChangeTag::Insert => ChangeKind::Insert,
                    ChangeTag::Delete => ChangeKind::Delete,
                };
                changes.push(DiffChange {
                    kind,
                    old_line: old_idx,
                    new_line: new_idx,
                    content: change.value().to_string(),
                });
            }
        }

        let old_start_final = if old_start == u32::MAX { 1 } else { old_start };
        let new_start_final = if new_start == u32::MAX { 1 } else { new_start };
        let old_count = if old_end >= old_start_final {
            old_end - old_start_final + 1
        } else {
            0
        };
        let new_count = if new_end >= new_start_final {
            new_end - new_start_final + 1
        } else {
            0
        };

        hunks.push(DiffHunk {
            old_start: old_start_final,
            old_count,
            new_start: new_start_final,
            new_count,
            changes,
        });
    }

    Ok(DiffResult {
        base_ref,
        target_ref,
        hunks,
        old_content,
        new_content,
    })
}

#[tauri::command]
pub fn list_refs(directory: String) -> CommandResult<RefList> {
    let repo = Repository::discover(&directory)?;

    // Collect branches
    let branch_iter = repo.branches(Some(git2::BranchType::Local))?;

    let mut branches: Vec<BranchInfo> = branch_iter
        .filter_map(|b| b.ok())
        .filter_map(|(branch, _)| {
            let name = branch.name().ok()??.to_string();
            let is_current = branch.is_head();
            Some(BranchInfo { name, is_current })
        })
        .collect();

    // Sort: current branch first, then alphabetical
    branches.sort_by(|a, b| match (a.is_current, b.is_current) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    // Collect tags
    let mut tags: Vec<String> = Vec::new();
    repo.tag_foreach(|_oid, name| {
        if let Ok(name_str) = std::str::from_utf8(name) {
            let tag_name = name_str.strip_prefix("refs/tags/").unwrap_or(name_str);
            tags.push(tag_name.to_string());
        }
        true
    })?;

    // Collect recent commits from HEAD
    let mut recent_commits: Vec<CommitInfo> = Vec::new();
    if let Ok(mut revwalk) = repo.revwalk() {
        let _ = revwalk.push_head();
        for oid in revwalk.take(10).flatten() {
            if let Ok(commit) = repo.find_commit(oid) {
                let sha = oid.to_string()[..7].to_string();
                let short_message = commit.summary().unwrap_or("").to_string();
                recent_commits.push(CommitInfo { sha, short_message });
            }
        }
    }

    Ok(RefList {
        branches,
        tags,
        recent_commits,
    })
}
