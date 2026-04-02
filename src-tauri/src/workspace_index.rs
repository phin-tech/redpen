use crate::settings::AppSettings;
use chrono::{DateTime, Utc};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

const WATCH_DEBOUNCE_MS: u64 = 300;
const DEFAULT_QUERY_LIMIT: usize = 150;
const GIT_ROOT_MAX_FILES: usize = 25_000;
const NON_GIT_ROOT_MAX_FILES: usize = 10_000;
const BUILT_IN_IGNORED_FOLDER_NAMES: &[&str] = &[
    ".git",
    ".venv",
    "venv",
    "node_modules",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "coverage",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub enum WorkspaceIndexState {
    Indexing,
    Ready,
    Stale,
    Error,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct WorkspaceIndexStatus {
    pub root: String,
    pub state: WorkspaceIndexState,
    pub indexed_count: usize,
    pub truncated: bool,
    pub last_updated: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct WorkspaceFileMatch {
    pub root: String,
    pub path: String,
    pub name: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct WorkspaceFileQueryResponse {
    pub results: Vec<WorkspaceFileMatch>,
    pub statuses: Vec<WorkspaceIndexStatus>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWorkspaceFilesRequest {
    pub query: String,
    pub roots: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRootsRequest {
    pub roots: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct IndexedFile {
    root: String,
    path: String,
    name: String,
    relative_path: String,
}

struct WatcherHandle {
    stop: Arc<AtomicBool>,
    join: thread::JoinHandle<()>,
}

struct RootEntry {
    status: WorkspaceIndexStatus,
    files: Vec<IndexedFile>,
    context: Option<RootContext>,
    refresh_requested: bool,
    rebuilding: bool,
    watcher: Option<WatcherHandle>,
}

struct WorkspaceIndexInner {
    settings: Arc<Mutex<AppSettings>>,
    roots: Mutex<HashMap<String, RootEntry>>,
}

#[derive(Clone)]
pub struct WorkspaceIndexService {
    inner: Arc<WorkspaceIndexInner>,
}

impl WorkspaceIndexService {
    pub fn new(settings: Arc<Mutex<AppSettings>>) -> Self {
        Self {
            inner: Arc::new(WorkspaceIndexInner {
                settings,
                roots: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn register_root(&self, root: &str) -> Result<(), String> {
        let normalized_root = normalize_root(root);
        let root_path = PathBuf::from(&normalized_root);
        if !root_path.is_dir() {
            return Err(format!("root is not a directory: {}", normalized_root));
        }

        {
            let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
            if roots.contains_key(&normalized_root) {
                return Ok(());
            }

            roots.insert(
                normalized_root.clone(),
                RootEntry {
                    status: WorkspaceIndexStatus {
                        root: normalized_root.clone(),
                        state: WorkspaceIndexState::Indexing,
                        indexed_count: 0,
                        truncated: false,
                        last_updated: None,
                        error: None,
                    },
                    files: Vec::new(),
                    context: None,
                    refresh_requested: false,
                    rebuilding: false,
                    watcher: None,
                },
            );
        }

        self.start_root_watcher(normalized_root.clone());
        self.request_refresh(&normalized_root, false);

        Ok(())
    }

    pub fn unregister_root(&self, root: &str) {
        let normalized_root = normalize_root(root);
        let watcher = {
            let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
            roots
                .remove(&normalized_root)
                .and_then(|mut entry| entry.watcher.take())
        };

        if let Some(watcher) = watcher {
            watcher.stop.store(true, Ordering::SeqCst);
            let _ = watcher.join.join();
        }
    }

    pub fn get_statuses(&self, roots: Option<&[String]>) -> Vec<WorkspaceIndexStatus> {
        let requested_roots = roots.map(normalize_roots).unwrap_or_default();
        let roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
        let mut statuses = roots
            .values()
            .filter(|entry| {
                requested_roots.is_empty() || requested_roots.contains(&entry.status.root)
            })
            .map(|entry| entry.status.clone())
            .collect::<Vec<_>>();
        statuses.sort_by(|a, b| a.root.cmp(&b.root));
        statuses
    }

    pub fn query(&self, request: QueryWorkspaceFilesRequest) -> WorkspaceFileQueryResponse {
        let requested_roots = request
            .roots
            .as_deref()
            .map(normalize_roots)
            .unwrap_or_default();
        let limit = request
            .limit
            .unwrap_or(DEFAULT_QUERY_LIMIT)
            .clamp(1, DEFAULT_QUERY_LIMIT);
        let query = request.query.trim().to_lowercase();

        let roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
        let mut root_entries = roots
            .values()
            .filter(|entry| {
                requested_roots.is_empty() || requested_roots.contains(&entry.status.root)
            })
            .collect::<Vec<_>>();
        root_entries.sort_by(|left, right| left.status.root.cmp(&right.status.root));

        let statuses = root_entries
            .iter()
            .map(|entry| entry.status.clone())
            .collect::<Vec<_>>();

        let results = if query.is_empty() {
            root_entries
                .iter()
                .flat_map(|entry| entry.files.iter())
                .take(limit)
                .map(|file| to_workspace_match(file.clone()))
                .collect::<Vec<_>>()
        } else {
            let mut ranked = Vec::with_capacity(limit);
            for entry in &root_entries {
                for file in &entry.files {
                    let Some(score) = score_match(file, &query) else {
                        continue;
                    };
                    insert_ranked_match(&mut ranked, limit, score, file);
                }
            }

            ranked
                .into_iter()
                .map(|(_, file)| to_workspace_match(file.clone()))
                .collect::<Vec<_>>()
        };

        WorkspaceFileQueryResponse { results, statuses }
    }

    pub fn refresh_all(&self) {
        let roots = self
            .inner
            .roots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for root in roots {
            self.request_refresh(&root, true);
        }
    }

    fn start_root_watcher(&self, root: String) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let service = self.clone();
        let root_for_thread = root.clone();

        let join = thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let watcher_result = RecommendedWatcher::new(
                move |result| {
                    let _ = tx.send(result);
                },
                Config::default(),
            );

            let mut watcher = match watcher_result {
                Ok(watcher) => watcher,
                Err(error) => {
                    service.set_error(&root_for_thread, error.to_string());
                    return;
                }
            };

            if let Err(error) = watcher.watch(Path::new(&root_for_thread), RecursiveMode::Recursive)
            {
                service.set_error(&root_for_thread, error.to_string());
                return;
            }

            let mut pending_refresh = false;
            let mut last_relevant_event_at = Instant::now();

            while !stop_for_thread.load(Ordering::SeqCst) {
                match rx.recv_timeout(Duration::from_millis(WATCH_DEBOUNCE_MS)) {
                    Ok(Ok(event)) => {
                        if !service.event_is_relevant(&root_for_thread, &event.paths) {
                            continue;
                        }
                        pending_refresh = true;
                        last_relevant_event_at = Instant::now();
                    }
                    Ok(Err(error)) => {
                        service.set_error(&root_for_thread, error.to_string());
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if pending_refresh
                            && last_relevant_event_at.elapsed()
                                >= Duration::from_millis(WATCH_DEBOUNCE_MS)
                        {
                            pending_refresh = false;
                            service.request_refresh(&root_for_thread, true);
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = roots.get_mut(&root) {
            entry.watcher = Some(WatcherHandle { stop, join });
        }
    }

    fn event_is_relevant(&self, root: &str, paths: &[PathBuf]) -> bool {
        let settings = self
            .inner
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let ignored_names = ignored_folder_names(&settings);
        let context = {
            let roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
            roots
                .get(root)
                .and_then(|entry| entry.context.clone())
                .unwrap_or_else(|| RootContext::resolve(Path::new(root)))
        };

        paths.iter().any(|path| {
            let canonical_path = canonicalize_path(path);
            if !canonical_path.starts_with(&context.root_path) {
                return false;
            }
            !is_path_ignored(
                &canonical_path,
                &context.root_path,
                context.project_root.as_deref(),
                &context.git_ignore_matcher,
                &ignored_names,
            )
        })
    }

    fn set_error(&self, root: &str, error: String) {
        let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = roots.get_mut(root) {
            entry.status.state = WorkspaceIndexState::Error;
            entry.status.error = Some(error);
        }
    }

    fn request_refresh(&self, root: &str, mark_stale: bool) {
        let should_spawn = {
            let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
            let Some(entry) = roots.get_mut(root) else {
                return;
            };
            entry.refresh_requested = true;
            if mark_stale
                && entry.status.last_updated.is_some()
                && entry.status.state == WorkspaceIndexState::Ready
            {
                entry.status.state = WorkspaceIndexState::Stale;
            }
            if entry.rebuilding {
                false
            } else {
                entry.rebuilding = true;
                true
            }
        };

        if should_spawn {
            let service = self.clone();
            let root = root.to_string();
            thread::spawn(move || {
                service.refresh_until_idle(root);
            });
        }
    }

    fn refresh_until_idle(&self, root: String) {
        loop {
            {
                let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
                let Some(entry) = roots.get_mut(&root) else {
                    return;
                };
                entry.refresh_requested = false;
                if entry.status.last_updated.is_none() {
                    entry.status.state = WorkspaceIndexState::Indexing;
                }
                entry.status.error = None;
            }

            let settings = self
                .inner
                .settings
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let snapshot = build_index_snapshot(Path::new(&root), &settings, None);

            let should_continue = {
                let mut roots = self.inner.roots.lock().unwrap_or_else(|e| e.into_inner());
                let Some(entry) = roots.get_mut(&root) else {
                    return;
                };

                match snapshot {
                    Ok(snapshot) => {
                        entry.files = snapshot.files;
                        entry.context = Some(snapshot.context);
                        entry.status.state = WorkspaceIndexState::Ready;
                        entry.status.indexed_count = entry.files.len();
                        entry.status.truncated = snapshot.truncated;
                        entry.status.last_updated = Some(Utc::now());
                        entry.status.error = None;
                    }
                    Err(error) => {
                        entry.status.state = WorkspaceIndexState::Error;
                        entry.status.error = Some(error);
                    }
                }

                if entry.refresh_requested {
                    true
                } else {
                    entry.rebuilding = false;
                    false
                }
            };

            if !should_continue {
                return;
            }
        }
    }
}

struct IndexSnapshot {
    files: Vec<IndexedFile>,
    truncated: bool,
    context: RootContext,
}

#[derive(Clone, Default)]
struct GitIgnoreMatcher {
    ignored_paths: Vec<String>,
}

impl GitIgnoreMatcher {
    fn from_repo(project_root: Option<&Path>) -> Self {
        #[cfg(test)]
        GIT_IGNORE_MATCHER_BUILD_COUNT.fetch_add(1, Ordering::SeqCst);
        let Some(project_root) = project_root else {
            return Self::default();
        };

        let ignored_paths = ProcessCommand::new("git")
            .arg("-C")
            .arg(project_root)
            .args([
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "--directory",
                "-z",
            ])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| {
                output
                    .stdout
                    .split(|byte| *byte == 0)
                    .filter(|entry| !entry.is_empty())
                    .filter_map(|entry| std::str::from_utf8(entry).ok())
                    .map(normalize_ignored_path)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self { ignored_paths }
    }

    fn matches(&self, relative_path: &Path) -> bool {
        let relative_path = normalize_ignored_path(relative_path.to_string_lossy().as_ref());

        self.ignored_paths.iter().any(|ignored_path| {
            relative_path == *ignored_path
                || relative_path
                    .strip_prefix(ignored_path.as_str())
                    .map(|suffix| suffix.starts_with('/'))
                    .unwrap_or(false)
        })
    }
}

#[derive(Clone)]
struct RootContext {
    root_path: PathBuf,
    project_root: Option<PathBuf>,
    git_ignore_matcher: GitIgnoreMatcher,
}

impl RootContext {
    fn resolve(root: &Path) -> Self {
        let root_path = canonicalize_path(root);
        let repo = git2::Repository::discover(&root_path).ok();
        let project_root = repo
            .as_ref()
            .and_then(|repository| repository.workdir().map(canonicalize_path));
        let git_ignore_matcher = GitIgnoreMatcher::from_repo(project_root.as_deref());

        Self {
            root_path,
            project_root,
            git_ignore_matcher,
        }
    }
}

fn build_index_snapshot(
    root: &Path,
    settings: &AppSettings,
    max_files_override: Option<usize>,
) -> Result<IndexSnapshot, String> {
    let context = RootContext::resolve(root);
    let repo = git2::Repository::discover(&context.root_path).ok();
    let ignored_names = ignored_folder_names(settings);
    let max_files = max_files_override.unwrap_or_else(|| {
        if repo.is_some() {
            GIT_ROOT_MAX_FILES
        } else {
            NON_GIT_ROOT_MAX_FILES
        }
    });

    let mut files = Vec::new();
    let mut truncated = false;

    let walker = WalkDir::new(&context.root_path).into_iter().filter_entry(|entry| {
        should_visit_entry(
            entry,
            &context.root_path,
            context.project_root.as_deref(),
            &context.git_ignore_matcher,
            &ignored_names,
        )
    });

    for entry in walker {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.depth() == 0 || !entry.file_type().is_file() {
            continue;
        }

        if files.len() >= max_files {
            truncated = true;
            break;
        }

        let path = entry.path();
        let relative_path = path
            .strip_prefix(&context.root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let name = entry.file_name().to_string_lossy().to_string();

        files.push(IndexedFile {
            root: root.to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            name,
            relative_path,
        });
    }

    files.sort_by(|a, b| {
        a.root
            .cmp(&b.root)
            .then(a.relative_path.cmp(&b.relative_path))
    });

    Ok(IndexSnapshot {
        files,
        truncated,
        context,
    })
}

#[cfg(test)]
static GIT_IGNORE_MATCHER_BUILD_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn reset_git_ignore_matcher_build_count() {
    GIT_IGNORE_MATCHER_BUILD_COUNT.store(0, Ordering::SeqCst);
}

#[cfg(test)]
fn git_ignore_matcher_build_count() -> usize {
    GIT_IGNORE_MATCHER_BUILD_COUNT.load(Ordering::SeqCst)
}

fn ignored_folder_names(settings: &AppSettings) -> HashSet<String> {
    let mut names = BUILT_IN_IGNORED_FOLDER_NAMES
        .iter()
        .map(|name| name.to_string())
        .collect::<HashSet<_>>();

    for name in &settings.ignored_folder_names {
        names.insert(name.clone());
    }

    names
}

fn should_visit_entry(
    entry: &DirEntry,
    scan_root: &Path,
    project_root: Option<&Path>,
    git_ignore_matcher: &GitIgnoreMatcher,
    ignored_names: &HashSet<String>,
) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    !is_path_ignored(
        entry.path(),
        scan_root,
        project_root,
        git_ignore_matcher,
        ignored_names,
    )
}

fn is_path_ignored(
    path: &Path,
    scan_root: &Path,
    project_root: Option<&Path>,
    git_ignore_matcher: &GitIgnoreMatcher,
    ignored_names: &HashSet<String>,
) -> bool {
    if let Ok(relative_to_scan_root) = path.strip_prefix(scan_root) {
        if relative_to_scan_root
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .any(|component| component.starts_with('.') || ignored_names.contains(component))
        {
            return true;
        }
    }

    let Some(project_root) = project_root else {
        return false;
    };

    let Ok(relative_path) = path.strip_prefix(project_root) else {
        return false;
    };

    git_ignore_matcher.matches(relative_path)
}

fn normalize_ignored_path(path: &str) -> String {
    path.trim_matches(['/', '\\']).to_string()
}

fn canonicalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn normalize_root(root: &str) -> String {
    let trimmed = root.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        root.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_roots(roots: &[String]) -> HashSet<String> {
    roots.iter().map(|root| normalize_root(root)).collect()
}

fn to_workspace_match(file: IndexedFile) -> WorkspaceFileMatch {
    WorkspaceFileMatch {
        root: file.root,
        path: file.path,
        name: file.name,
        relative_path: file.relative_path,
    }
}

fn insert_ranked_match<'a>(
    ranked: &mut Vec<(usize, &'a IndexedFile)>,
    limit: usize,
    score: usize,
    file: &'a IndexedFile,
) {
    if limit == 0 {
        return;
    }

    let insert_at = ranked
        .iter()
        .position(|(existing_score, existing_file)| {
            compare_ranked_matches(score, file, *existing_score, existing_file).is_lt()
        })
        .unwrap_or(ranked.len());

    if insert_at >= limit {
        return;
    }

    ranked.insert(insert_at, (score, file));
    if ranked.len() > limit {
        ranked.pop();
    }
}

fn compare_ranked_matches(
    left_score: usize,
    left_file: &IndexedFile,
    right_score: usize,
    right_file: &IndexedFile,
) -> std::cmp::Ordering {
    right_score
        .cmp(&left_score)
        .then(left_file.root.cmp(&right_file.root))
        .then(left_file.relative_path.cmp(&right_file.relative_path))
}

fn score_match(file: &IndexedFile, query: &str) -> Option<usize> {
    let name = file.name.to_lowercase();
    let relative_path = file.relative_path.to_lowercase();
    let tokens = query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return Some(0);
    }

    if tokens
        .iter()
        .any(|token| !name.contains(token) && !relative_path.contains(token))
    {
        return None;
    }

    let mut score = 0;
    if name == query {
        score += 1_200;
    }
    if name.starts_with(query) {
        score += 800;
    }
    if relative_path.starts_with(query) {
        score += 500;
    }
    if name.contains(query) {
        score += 300;
    }
    if relative_path.contains(query) {
        score += 150;
    }

    for token in tokens {
        if name.starts_with(token) {
            score += 120;
        } else if name.contains(token) {
            score += 60;
        }

        if relative_path.starts_with(token) {
            score += 40;
        } else if relative_path.contains(token) {
            score += 20;
        }
    }

    score += 100usize.saturating_sub(file.relative_path.len().min(100));
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::{
        build_index_snapshot, git_ignore_matcher_build_count, normalize_root,
        reset_git_ignore_matcher_build_count, score_match, IndexedFile, WorkspaceIndexService,
        WorkspaceIndexState,
    };
    use crate::settings::AppSettings;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[test]
    fn built_in_and_user_ignored_folders_are_excluded_from_index() {
        let directory = tempdir().unwrap();
        fs::create_dir_all(directory.path().join("src")).unwrap();
        fs::create_dir_all(directory.path().join("node_modules")).unwrap();
        fs::create_dir_all(directory.path().join("generated")).unwrap();
        fs::write(directory.path().join("src").join("main.ts"), "ok").unwrap();
        fs::write(
            directory.path().join("node_modules").join("lib.js"),
            "ignored",
        )
        .unwrap();
        fs::write(
            directory.path().join("generated").join("code.ts"),
            "ignored",
        )
        .unwrap();

        let snapshot = build_index_snapshot(
            directory.path(),
            &AppSettings {
                author: "sam".to_string(),
                default_labels: Vec::new(),
                ignored_folder_names: vec!["generated".to_string()],
                ..AppSettings::default()
            },
            None,
        )
        .unwrap();

        let indexed_paths = snapshot
            .files
            .into_iter()
            .map(|file| file.relative_path)
            .collect::<Vec<_>>();

        assert_eq!(indexed_paths, vec!["src/main.ts".to_string()]);
    }

    #[test]
    fn git_ignored_and_hidden_entries_are_excluded() {
        let directory = tempdir().unwrap();
        git2::Repository::init(directory.path()).unwrap();
        fs::create_dir_all(directory.path().join("src")).unwrap();
        fs::write(directory.path().join(".gitignore"), "ignored/\n").unwrap();
        fs::create_dir_all(directory.path().join("ignored")).unwrap();
        fs::write(directory.path().join("src").join("main.ts"), "ok").unwrap();
        fs::write(directory.path().join("ignored").join("skip.ts"), "ignored").unwrap();
        fs::write(directory.path().join(".hidden.ts"), "hidden").unwrap();

        let snapshot =
            build_index_snapshot(directory.path(), &AppSettings::default(), None).unwrap();
        let indexed_paths = snapshot
            .files
            .into_iter()
            .map(|file| file.relative_path)
            .collect::<Vec<_>>();

        assert_eq!(indexed_paths, vec!["src/main.ts".to_string()]);
    }

    #[test]
    fn hidden_ancestor_of_workspace_root_does_not_hide_project_files() {
        let directory = tempdir().unwrap();
        let hidden_parent = directory.path().join(".hidden-parent");
        let project_root = hidden_parent.join("project");
        fs::create_dir_all(project_root.join("src")).unwrap();
        fs::write(project_root.join("src").join("main.ts"), "ok").unwrap();

        let snapshot = build_index_snapshot(&project_root, &AppSettings::default(), None).unwrap();
        let indexed_paths = snapshot
            .files
            .into_iter()
            .map(|file| file.relative_path)
            .collect::<Vec<_>>();

        assert_eq!(indexed_paths, vec!["src/main.ts".to_string()]);
    }

    #[test]
    fn non_git_roots_respect_caps_and_report_truncation() {
        let directory = tempdir().unwrap();
        for index in 0..5 {
            fs::write(directory.path().join(format!("file-{index}.txt")), "ok").unwrap();
        }

        let snapshot =
            build_index_snapshot(directory.path(), &AppSettings::default(), Some(3)).unwrap();

        assert_eq!(snapshot.files.len(), 3);
        assert!(snapshot.truncated);
    }

    #[test]
    fn query_scores_exact_and_prefix_matches_higher() {
        let exact = IndexedFile {
            root: "/repo".to_string(),
            path: "/repo/src/app.ts".to_string(),
            name: "app.ts".to_string(),
            relative_path: "src/app.ts".to_string(),
        };
        let fuzzy = IndexedFile {
            root: "/repo".to_string(),
            path: "/repo/src/my-app.ts".to_string(),
            name: "my-app.ts".to_string(),
            relative_path: "src/my-app.ts".to_string(),
        };

        assert!(score_match(&exact, "app").unwrap() > score_match(&fuzzy, "app").unwrap());
    }

    #[test]
    fn query_respects_result_limit() {
        let directory = tempdir().unwrap();
        for index in 0..5 {
            fs::write(directory.path().join(format!("file-{index}.ts")), "ok").unwrap();
        }

        let settings = Arc::new(Mutex::new(AppSettings::default()));
        let service = WorkspaceIndexService::new(settings);
        let root = directory.path().to_string_lossy().to_string();

        service.register_root(&root).unwrap();
        wait_for_state(&service, &normalize_root(&root), WorkspaceIndexState::Ready);

        let response = service.query(super::QueryWorkspaceFilesRequest {
            query: "file".to_string(),
            roots: Some(vec![root.clone()]),
            limit: Some(2),
        });

        assert_eq!(response.results.len(), 2);

        service.unregister_root(&root);
    }

    #[test]
    fn empty_query_orders_results_by_root_then_relative_path() {
        let parent = tempdir().unwrap();
        let root_a_path = parent.path().join("root-a");
        let root_b_path = parent.path().join("root-b");
        fs::create_dir_all(root_b_path.join("src")).unwrap();
        fs::create_dir_all(root_a_path.join("src")).unwrap();
        fs::write(root_b_path.join("src").join("z.ts"), "ok").unwrap();
        fs::write(root_a_path.join("a.ts"), "ok").unwrap();
        fs::write(root_a_path.join("src").join("b.ts"), "ok").unwrap();

        let settings = Arc::new(Mutex::new(AppSettings::default()));
        let service = WorkspaceIndexService::new(settings);
        let root_b = root_b_path.to_string_lossy().to_string();
        let root_a = root_a_path.to_string_lossy().to_string();

        service.register_root(&root_b).unwrap();
        service.register_root(&root_a).unwrap();
        wait_for_state(&service, &normalize_root(&root_a), WorkspaceIndexState::Ready);
        wait_for_state(&service, &normalize_root(&root_b), WorkspaceIndexState::Ready);

        let response = service.query(super::QueryWorkspaceFilesRequest {
            query: String::new(),
            roots: Some(vec![root_b.clone(), root_a.clone()]),
            limit: Some(10),
        });

        let ordered = response
            .results
            .into_iter()
            .map(|file| (file.root, file.relative_path))
            .collect::<Vec<_>>();

        assert_eq!(
            ordered,
            vec![
                (normalize_root(&root_a), "a.ts".to_string()),
                (normalize_root(&root_a), "src/b.ts".to_string()),
                (normalize_root(&root_b), "src/z.ts".to_string()),
            ]
        );

        service.unregister_root(&root_a);
        service.unregister_root(&root_b);
    }

    #[test]
    fn search_query_breaks_ties_by_root_then_relative_path() {
        let parent = tempdir().unwrap();
        let root_a_path = parent.path().join("root-a");
        let root_b_path = parent.path().join("root-b");
        fs::create_dir_all(root_b_path.join("a")).unwrap();
        fs::create_dir_all(root_a_path.join("a")).unwrap();
        fs::create_dir_all(root_a_path.join("b")).unwrap();
        fs::write(root_b_path.join("a").join("app.ts"), "ok").unwrap();
        fs::write(root_a_path.join("a").join("app.ts"), "ok").unwrap();
        fs::write(root_a_path.join("b").join("app.ts"), "ok").unwrap();

        let settings = Arc::new(Mutex::new(AppSettings::default()));
        let service = WorkspaceIndexService::new(settings);
        let root_b = root_b_path.to_string_lossy().to_string();
        let root_a = root_a_path.to_string_lossy().to_string();

        service.register_root(&root_b).unwrap();
        service.register_root(&root_a).unwrap();
        wait_for_state(&service, &normalize_root(&root_a), WorkspaceIndexState::Ready);
        wait_for_state(&service, &normalize_root(&root_b), WorkspaceIndexState::Ready);

        let response = service.query(super::QueryWorkspaceFilesRequest {
            query: "app.ts".to_string(),
            roots: Some(vec![root_b.clone(), root_a.clone()]),
            limit: Some(10),
        });

        let ordered = response
            .results
            .into_iter()
            .map(|file| (file.root, file.relative_path))
            .collect::<Vec<_>>();

        assert_eq!(
            ordered,
            vec![
                (normalize_root(&root_a), "a/app.ts".to_string()),
                (normalize_root(&root_a), "b/app.ts".to_string()),
                (normalize_root(&root_b), "a/app.ts".to_string()),
            ]
        );

        service.unregister_root(&root_a);
        service.unregister_root(&root_b);
    }

    #[test]
    fn refresh_request_rebuilds_index_after_file_change() {
        let directory = tempdir().unwrap();
        for index in 0..4_000 {
            fs::write(directory.path().join(format!("file-{index}.txt")), "ok").unwrap();
        }

        let settings = Arc::new(Mutex::new(AppSettings::default()));
        let service = WorkspaceIndexService::new(settings);
        let root = directory.path().to_string_lossy().to_string();

        service.register_root(&root).unwrap();
        wait_for_state(&service, &normalize_root(&root), WorkspaceIndexState::Ready);
        let initial_last_updated = service
            .get_statuses(Some(&[root.clone()]))
            .first()
            .and_then(|status| status.last_updated)
            .expect("workspace index should have last_updated once ready");

        fs::write(directory.path().join("file-0.txt"), "updated").unwrap();
        let normalized_root = normalize_root(&root);
        service.request_refresh(&normalized_root, true);

        let stale_status = service
            .get_statuses(Some(&[root.clone()]))
            .into_iter()
            .next()
            .expect("workspace index status should exist");
        assert_eq!(stale_status.state, WorkspaceIndexState::Stale);

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5) {
            let statuses = service.get_statuses(Some(&[root.clone()]));
            if let Some(status) = statuses.first() {
                if status.state == WorkspaceIndexState::Ready
                    && status
                        .last_updated
                        .map(|last_updated| last_updated > initial_last_updated)
                        .unwrap_or(false)
                {
                    service.unregister_root(&root);
                    return;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }

        service.unregister_root(&root);
        panic!("workspace index did not refresh after a file change");
    }

    #[test]
    fn event_relevance_uses_cached_root_context() {
        let directory = tempdir().unwrap();
        git2::Repository::init(directory.path()).unwrap();
        fs::create_dir_all(directory.path().join("src")).unwrap();
        fs::write(directory.path().join("src").join("main.ts"), "ok").unwrap();

        let settings = Arc::new(Mutex::new(AppSettings::default()));
        let service = WorkspaceIndexService::new(settings);
        let root = directory.path().to_string_lossy().to_string();
        let normalized_root = normalize_root(&root);

        service.register_root(&root).unwrap();
        wait_for_state(&service, &normalized_root, WorkspaceIndexState::Ready);

        reset_git_ignore_matcher_build_count();

        let tracked_file = directory.path().join("src").join("main.ts");
        assert!(service.event_is_relevant(&normalized_root, &[tracked_file.clone()]));
        assert!(service.event_is_relevant(&normalized_root, &[tracked_file]));
        assert_eq!(git_ignore_matcher_build_count(), 0);

        service.unregister_root(&root);
    }

    #[test]
    fn refresh_updates_cached_git_ignore_matcher() {
        let directory = tempdir().unwrap();
        git2::Repository::init(directory.path()).unwrap();
        fs::create_dir_all(directory.path().join("src")).unwrap();
        fs::write(directory.path().join("src").join("main.ts"), "ok").unwrap();

        let settings = Arc::new(Mutex::new(AppSettings::default()));
        let service = WorkspaceIndexService::new(settings);
        let root = directory.path().to_string_lossy().to_string();
        let normalized_root = normalize_root(&root);

        service.register_root(&root).unwrap();
        wait_for_state(&service, &normalized_root, WorkspaceIndexState::Ready);

        fs::write(directory.path().join(".gitignore"), "ignored/\n").unwrap();
        fs::create_dir_all(directory.path().join("ignored")).unwrap();
        fs::write(directory.path().join("ignored").join("skip.ts"), "ignored").unwrap();
        service.request_refresh(&normalized_root, true);
        wait_for_state(&service, &normalized_root, WorkspaceIndexState::Ready);

        reset_git_ignore_matcher_build_count();

        let ignored_file = directory.path().join("ignored").join("skip.ts");
        assert!(!service.event_is_relevant(&normalized_root, &[ignored_file]));
        assert_eq!(git_ignore_matcher_build_count(), 0);

        service.unregister_root(&root);
    }

    fn wait_for_state(
        service: &WorkspaceIndexService,
        root: &str,
        expected_state: WorkspaceIndexState,
    ) {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5) {
            let statuses = service.get_statuses(Some(&[root.to_string()]));
            if statuses
                .first()
                .map(|status| status.state == expected_state)
                .unwrap_or(false)
            {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("workspace index did not reach expected state");
    }
}
