mod server_client;

use clap::{Parser, Subcommand};
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind, Choice, Range, SelectionMode};
use redpen_core::export::export_markdown;
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "redpen", about = "Code annotation tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add an annotation to a file
    Annotate {
        file: PathBuf,
        #[arg(long)]
        line: Option<u32>,
        #[arg(long)]
        range: Option<String>,
        #[arg(long)]
        body: String,
        #[arg(long, action = clap::ArgAction::Append)]
        label: Vec<String>,
        #[arg(long, default_value_t = whoami::username())]
        author: String,
        #[arg(long, default_value = "comment")]
        kind: String,
        /// Reply to an existing annotation by ID (inherits parent's anchor)
        #[arg(long)]
        reply_to: Option<String>,
        /// Add a choice option (can be repeated for multiple choices)
        #[arg(long, action = clap::ArgAction::Append)]
        choice: Vec<String>,
        /// Selection mode for choices: single (radio) or multi (checkbox)
        #[arg(long, default_value = "single")]
        selection_mode: String,
    },
    /// List all annotations as JSON
    List {
        /// File to list annotations for
        file: Option<PathBuf>,
        /// List all annotations for a review session
        #[arg(long)]
        session: Option<String>,
    },
    /// Export annotations as markdown
    Export {
        file: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Show annotation summary
    Status { file: PathBuf },
    /// Open file(s) or directory in the desktop app
    Open {
        /// Files or directories to open (optional when using --diff-base or --pre-push)
        paths: Vec<PathBuf>,
        #[arg(long)]
        line: Option<u32>,
        /// Block until review is complete (combines open + wait)
        #[arg(long)]
        wait: bool,
        /// Timeout in seconds when using --wait (default: no timeout, or 600s with --pre-push)
        #[arg(long)]
        timeout: Option<u64>,
        /// Disable timeout (overrides --timeout and --pre-push default)
        #[arg(long)]
        no_timeout: bool,
        /// Compute changed files by diffing against the given git ref
        #[arg(long, conflicts_with = "paths")]
        diff_base: Option<String>,
        /// Read git pre-push hook stdin to determine changed files. Implies --wait.
        #[arg(long, conflicts_with = "paths", conflicts_with = "diff_base")]
        pre_push: bool,
        /// Diff against the remote tracking branch of the current branch. Implies --wait.
        /// Use this in hook managers (e.g. prek) that don't forward pre-push stdin.
        #[arg(
            long,
            conflicts_with = "paths",
            conflicts_with = "diff_base",
            conflicts_with = "pre_push"
        )]
        diff_remote: bool,
    },
    /// Wait for review to complete (blocks until "Done Reviewing" is clicked in the app)
    Wait {
        /// Files or directories to wait on
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Timeout in seconds (default: no timeout)
        #[arg(long)]
        timeout: Option<u64>,
        /// Reuse an existing session ID for multi-round reviews
        #[arg(long)]
        session: Option<String>,
    },
    /// Open a GitHub PR review in the desktop app
    ReviewPr {
        /// PR reference: owner/repo#number or GitHub PR URL
        pr_ref: String,
        /// Local checkout path for the base repository
        #[arg(long)]
        local_path: Option<PathBuf>,
    },
    /// Print agent usage prompt (system prompt for AI agents using redpen)
    Agents,
}

/// Exit codes for the pre-push review gate.
#[allow(dead_code)]
const EXIT_APPROVED: i32 = 0;
const EXIT_CHANGES_REQUESTED: i32 = 1;
const EXIT_TIMEOUT: i32 = 2;
const EXIT_NO_APP: i32 = 3;

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Annotate {
            file,
            line,
            range,
            body,
            label,
            author,
            kind,
            reply_to,
            choice,
            selection_mode,
        } => cmd_annotate(
            &file,
            line,
            range.as_deref(),
            &body,
            &label,
            &author,
            &kind,
            reply_to.as_deref(),
            &choice,
            &selection_mode,
        ),
        Commands::List { file, session } => {
            if let Some(session_id) = session {
                cmd_list_session(&session_id)
            } else if let Some(file) = file {
                cmd_list(&file)
            } else {
                Err("Either a file path or --session <id> is required".into())
            }
        }
        Commands::Export { file, output } => cmd_export(&file, output.as_deref()),
        Commands::Status { file } => cmd_status(&file),
        Commands::Open {
            paths,
            line,
            wait,
            timeout,
            no_timeout,
            diff_base,
            pre_push,
            diff_remote,
        } => cmd_open_dispatch(
            paths,
            line,
            wait,
            timeout,
            no_timeout,
            diff_base,
            pre_push,
            diff_remote,
        ),
        Commands::Wait {
            paths,
            timeout,
            session,
        } => cmd_wait(&paths, timeout, session.as_deref()),
        Commands::ReviewPr { pr_ref, local_path } => cmd_review_pr(&pr_ref, local_path.as_deref()),
        Commands::Agents => {
            print!("{}", AGENT_PROMPT);
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn parse_range(range_str: &str) -> Result<Range, String> {
    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return Err("Range format: startLine:startCol-endLine:endCol".to_string());
    }
    let start: Vec<u32> = parts[0]
        .split(':')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    let end: Vec<u32> = parts[1]
        .split(':')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    if start.len() != 2 || end.len() != 2 {
        return Err("Range format: startLine:startCol-endLine:endCol".to_string());
    }
    Ok(Range {
        start_line: start[0],
        start_column: start[1],
        end_line: end[0],
        end_column: end[1],
    })
}

fn parse_kind(kind: &str) -> Result<AnnotationKind, String> {
    match kind {
        "comment" => Ok(AnnotationKind::Comment),
        "lineNote" | "line-note" | "linenote" => Ok(AnnotationKind::LineNote),
        "label" => Ok(AnnotationKind::Label),
        "explanation" => Ok(AnnotationKind::Explanation),
        other => Err(format!(
            "Unknown kind: {}. Use comment, lineNote, label, or explanation.",
            other
        )),
    }
}

fn parse_selection_mode(mode: &str) -> Result<SelectionMode, String> {
    match mode {
        "single" => Ok(SelectionMode::Single),
        "multi" => Ok(SelectionMode::Multi),
        other => Err(format!(
            "Unknown selection mode: {}. Use single or multi.",
            other
        )),
    }
}

fn resolve_project_root(source_path: &Path) -> PathBuf {
    let fallback = || dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo
            .workdir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(fallback),
        Err(_) => fallback(),
    }
}

fn load_sidecar_for_file(
    project_root: &Path,
    source_path: &Path,
) -> Result<SidecarFile, Box<dyn std::error::Error>> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if annotation_path.exists() {
        let mut sidecar = SidecarFile::load(&annotation_path)?;
        let current_hash = hash_file(source_path)?;
        if sidecar.source_file_hash != current_hash {
            let content = fs::read_to_string(source_path)?;
            redpen_core::anchor::reanchor_annotations(&mut sidecar.annotations, &content);
            sidecar.source_file_hash = current_hash;
        }
        Ok(sidecar)
    } else {
        Ok(SidecarFile::new(hash_file(source_path)?))
    }
}

fn save_sidecar_for_file(
    project_root: &Path,
    source_path: &Path,
    sidecar: &SidecarFile,
) -> Result<(), Box<dyn std::error::Error>> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if sidecar.annotations.is_empty() {
        if annotation_path.exists() {
            fs::remove_file(annotation_path)?;
        }
        return Ok(());
    }

    sidecar.save(&annotation_path)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_annotate(
    file: &Path,
    line: Option<u32>,
    range_str: Option<&str>,
    body: &str,
    labels: &[String],
    author: &str,
    kind: &str,
    reply_to: Option<&str>,
    choices: &[String],
    selection_mode: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);

    // If replying, clone parent's anchor instead of building a new one
    if let Some(parent_id) = reply_to {
        let mut sidecar = load_sidecar_for_file(&project_root, &abs_path)?;
        let parent = sidecar
            .get_annotation(parent_id)
            .ok_or_else(|| format!("Annotation {} not found", parent_id))?;
        let parent_anchor = parent.anchor.clone();
        let start_line = parent.line();

        let reply = Annotation::new_reply(
            body.to_string(),
            author.to_string(),
            parent_id.to_string(),
            parent_anchor,
        );
        let id = reply.id.clone();
        sidecar.add_annotation(reply);
        save_sidecar_for_file(&project_root, &abs_path, &sidecar)?;
        notify_app("annotation_reply", &abs_path, Some(start_line));
        println!(
            "Created reply {} to {} on line {}",
            id, parent_id, start_line
        );
        return Ok(());
    }

    let content = fs::read_to_string(&abs_path)?;
    let source_lines: Vec<&str> = content.lines().collect();

    let range = if let Some(r) = range_str {
        parse_range(r)?
    } else {
        let l = line.unwrap_or(1);
        let line_len = source_lines
            .get((l - 1) as usize)
            .map(|s| s.len() as u32)
            .unwrap_or(0);
        Range {
            start_line: l,
            start_column: 0,
            end_line: l,
            end_column: line_len,
        }
    };

    let line_idx = (range.start_line as usize).saturating_sub(1);
    let line_content = source_lines.get(line_idx).unwrap_or(&"").to_string();

    let start = line_idx.saturating_sub(2);
    let end = (line_idx + 3).min(source_lines.len());
    let surrounding_lines: Vec<String> = source_lines[start..end]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let start_line = range.start_line;
    let anchor = Anchor::TextContext {
        line_content: line_content.clone(),
        surrounding_lines,
        content_hash: hash_string(&line_content),
        range,
        last_known_line: start_line,
    };

    let mut annotation = Annotation::new(
        parse_kind(kind)?,
        body.to_string(),
        labels.to_vec(),
        author.to_string(),
        anchor,
    );

    if !choices.is_empty() {
        let mode = parse_selection_mode(selection_mode)?;
        let choice_list = choices
            .iter()
            .enumerate()
            .map(|(i, label)| Choice {
                id: format!("c{}", i),
                label: label.clone(),
                selected: false,
            })
            .collect();
        annotation = annotation.with_choices(choice_list, mode);
    }

    let mut sidecar = load_sidecar_for_file(&project_root, &abs_path)?;
    let id = annotation.id.clone();
    sidecar.add_annotation(annotation);
    save_sidecar_for_file(&project_root, &abs_path, &sidecar)?;
    notify_app_refresh(&abs_path);

    println!("Created annotation {} on line {}", id, start_line);
    Ok(())
}

/// Send a refresh to the desktop app — tries HTTP server first, falls back to deep link.
fn notify_app_refresh(file_path: &Path) {
    let path_str = file_path.to_string_lossy();
    if server_client::refresh_file(&path_str) {
        return;
    }
    // Fallback: deep link
    let _url = format!("redpen://refresh?file={}", urlencoding::encode(&path_str));
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&_url).spawn();
    }
}

/// Notify the desktop app — tries HTTP server first (open + refresh), falls back to deep link.
fn notify_app(kind: &str, file_path: &Path, line: Option<u32>) {
    let path_str = file_path.to_string_lossy();
    // The server's open endpoint handles both opening and refreshing
    if server_client::open_file(&path_str, line) {
        return;
    }
    // Fallback: deep link
    let mut url = format!(
        "redpen://notify?kind={}&file={}",
        kind,
        urlencoding::encode(&path_str)
    );
    if let Some(l) = line {
        url.push_str(&format!("&line={}", l));
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }
}

/// Print a human-readable summary to stderr when a review is rejected.
fn print_rejection_summary(
    files: &[PathBuf],
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("\nReview verdict: changes_requested");
    eprintln!("Session: {}", session_id);
    for file in files {
        let project_root = resolve_project_root(file);
        let sidecar = load_sidecar_for_file(&project_root, file)?;
        let count = sidecar.annotations.len();
        if count > 0 {
            eprintln!("{} annotation(s) on {}", count, file.display());
        }
    }
    eprintln!("Push blocked. Fix the flagged issues and push again.\n");
    eprintln!(
        "Run `redpen list --session {}` for full annotation details.",
        session_id
    );
    Ok(())
}

fn cmd_list_session(session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    ensure_server_available()?;
    match server_client::session_annotations(session_id) {
        Some(annotations) => {
            println!("{}", serde_json::to_string_pretty(&annotations)?);
            Ok(())
        }
        None => Err(format!("Could not fetch annotations for session {}", session_id).into()),
    }
}

fn cmd_list(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);
    let sidecar = load_sidecar_for_file(&project_root, &abs_path)?;
    let json = serde_json::to_string_pretty(&sidecar.annotations)?;
    println!("{}", json);
    Ok(())
}

fn cmd_export(file: &Path, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);
    let sidecar = load_sidecar_for_file(&project_root, &abs_path)?;
    let content = fs::read_to_string(&abs_path)?;
    let file_name = abs_path
        .file_name()
        .ok_or("path has no file name")?
        .to_string_lossy()
        .to_string();
    let markdown = export_markdown(&sidecar, &content, &file_name);
    if let Some(out_path) = output {
        fs::write(out_path, &markdown)?;
        println!("Exported to {}", out_path.display());
    } else {
        print!("{}", markdown);
    }
    Ok(())
}

fn cmd_status(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);
    let sidecar = load_sidecar_for_file(&project_root, &abs_path)?;
    let total = sidecar.annotations.len();
    if total == 0 {
        println!("{}: no annotations", file.display());
        return Ok(());
    }
    let orphaned = sidecar.annotations.iter().filter(|a| a.is_orphaned).count();
    if orphaned > 0 {
        println!(
            "{}: {} annotations ({} orphaned)",
            file.display(),
            total,
            orphaned
        );
    } else {
        println!("{}: {} annotations", file.display(), total);
    }
    Ok(())
}

fn cmd_wait(
    paths: &[PathBuf],
    timeout: Option<u64>,
    session: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let files = expand_paths(paths)?;
    if files.is_empty() {
        eprintln!("No files found");
        return Ok(());
    }

    ensure_server_available()?;
    cmd_wait_via_server(&files, timeout, session)
}

fn cmd_wait_via_server(
    files: &[PathBuf],
    timeout: Option<u64>,
    session: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_str = files[0].to_string_lossy();
    eprintln!(
        "Waiting for review of {} file(s) via server...",
        files.len()
    );

    let session_id = match session {
        Some(existing) => existing.to_string(),
        None => server_client::review_start(&file_str, None, None)
            .ok_or("Could not start review session via Red Pen server")?,
    };

    match server_client::review_wait(&session_id, timeout) {
        server_client::ReviewWaitResult::Ok(resp) => {
            let output = review_output(files, resp.verdict, &session_id)?;
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(())
        }
        server_client::ReviewWaitResult::Timeout => Err("Review wait timed out".into()),
        server_client::ReviewWaitResult::ServerUnavailable => {
            Err("Review wait failed — Red Pen server unavailable".into())
        }
    }
}

#[cfg(target_os = "macos")]
fn is_app_running() -> bool {
    std::process::Command::new("pgrep")
        .args(["-f", "red-pen-tauri"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn ensure_app_running() -> Result<(), Box<dyn std::error::Error>> {
    if is_app_running() {
        return Ok(());
    }

    eprintln!("Red Pen is not running, launching...");
    std::process::Command::new("open")
        .args(["-a", "Red Pen"])
        .status()?;

    // Wait for the app to start (up to 5 seconds)
    for _ in 0..25 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if is_app_running() {
            // Extra delay for the app to initialize its deep link handler
            std::thread::sleep(std::time::Duration::from_millis(500));
            return Ok(());
        }
    }

    Err("Could not launch Red Pen. Is it installed? Download it from https://github.com/phin-tech/redpen/releases".into())
}

/// Expand a list of paths: files pass through, directories are recursively expanded to all files.
/// Skips hidden files/dirs and .redpen directory.
fn expand_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    for path in paths {
        let abs = fs::canonicalize(path)?;
        if abs.is_dir() {
            expand_dir(&abs, &mut result)?;
        } else {
            result.push(abs);
        }
    }
    Ok(result)
}

fn expand_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            expand_dir(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Returns true if we should skip the review gate (CI environment or explicit opt-out).
fn should_skip_gate() -> Option<&'static str> {
    if std::env::var("REDPEN_SKIP_GATE").is_ok() {
        return Some("REDPEN_SKIP_GATE is set");
    }
    // Well-known CI environment variables
    let ci_vars = [
        ("CI", "CI environment detected"),
        ("GITHUB_ACTIONS", "GitHub Actions detected"),
        ("GITLAB_CI", "GitLab CI detected"),
        ("CIRCLECI", "CircleCI detected"),
        ("JENKINS_URL", "Jenkins detected"),
        ("BUILDKITE", "Buildkite detected"),
        ("TF_BUILD", "Azure Pipelines detected"),
        ("CODEBUILD_BUILD_ID", "AWS CodeBuild detected"),
    ];
    for (var, reason) in &ci_vars {
        if std::env::var(var).is_ok() {
            return Some(reason);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn cmd_open_dispatch(
    paths: Vec<PathBuf>,
    line: Option<u32>,
    wait: bool,
    timeout: Option<u64>,
    no_timeout: bool,
    diff_base: Option<String>,
    pre_push: bool,
    diff_remote: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Skip the gate entirely in CI or when explicitly opted out
    if pre_push || diff_remote || diff_base.is_some() || wait {
        if let Some(reason) = should_skip_gate() {
            eprintln!("Red Pen: skipping review gate ({})", reason);
            return Ok(());
        }
    }

    let (resolved_paths, effective_wait, effective_timeout) = if pre_push {
        let diff_sha = parse_pre_push_stdin()?;
        let files = git_diff_files(&diff_sha)?;
        if files.is_empty() {
            // No changed files — allow push
            return Ok(());
        }
        let t = if no_timeout {
            None
        } else {
            Some(timeout.unwrap_or(600))
        };
        (files, true, t) // --pre-push implies --wait
    } else if diff_remote {
        let sha = git_remote_tracking_sha()?;
        let files = git_diff_files(&sha)?;
        if files.is_empty() {
            return Ok(());
        }
        let t = if no_timeout {
            None
        } else {
            Some(timeout.unwrap_or(600))
        };
        (files, true, t)
    } else if let Some(ref base) = diff_base {
        let files = git_diff_files(base)?;
        if files.is_empty() {
            eprintln!("No changed files");
            return Ok(());
        }
        let t = if no_timeout { None } else { timeout };
        (files, wait, t)
    } else {
        if paths.is_empty() {
            return Err("Provide file paths, or use --diff-base or --pre-push".into());
        }
        let t = if no_timeout { None } else { timeout };
        (paths, wait, t)
    };

    if effective_wait {
        cmd_open_and_wait(&resolved_paths, line, effective_timeout)
    } else {
        cmd_open(&resolved_paths, line)
    }
}

/// Parse git pre-push stdin to extract the diff base sha.
/// Format: `<local-ref> <local-sha> <remote-ref> <remote-sha>`
/// Returns the remote sha (what to diff against).
fn parse_pre_push_stdin() -> Result<String, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let line = stdin
        .lock()
        .lines()
        .next()
        .ok_or("No input on stdin — is this running as a git pre-push hook?")?
        .map_err(|e| format!("Failed to read stdin: {}", e))?;

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(format!(
            "Expected git pre-push format: <local-ref> <local-sha> <remote-ref> <remote-sha>, got: {}",
            line
        )
        .into());
    }

    let local_sha = parts[1];
    let remote_sha = parts[3];
    let zero_sha = "0000000000000000000000000000000000000000";

    // Push deletion — nothing to review
    if local_sha == zero_sha {
        return Err("Push deletion — nothing to review".into());
    }

    // New branch — diff against merge-base with default branch
    if remote_sha == zero_sha {
        let default_branch = git_default_branch()?;
        let output = process::Command::new("git")
            .args(["merge-base", &default_branch, "HEAD"])
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "Could not find merge-base with {}: {}",
                default_branch,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    Ok(remote_sha.to_string())
}

/// Get the default branch name (main or master).
fn git_default_branch() -> Result<String, Box<dyn std::error::Error>> {
    // Try origin/HEAD first
    let output = process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()?;
    if output.status.success() {
        let full_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(branch) = full_ref.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    // Fallback: check if main or master exist
    for branch in ["main", "master"] {
        let status = process::Command::new("git")
            .args(["rev-parse", "--verify", &format!("origin/{}", branch)])
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status()?;
        if status.success() {
            return Ok(branch.to_string());
        }
    }

    Err("Could not determine default branch".into())
}

/// Get the sha of the remote tracking branch for the current branch.
/// Falls back to origin/HEAD if no tracking branch is configured.
fn git_remote_tracking_sha() -> Result<String, Box<dyn std::error::Error>> {
    // Try @{u} — the upstream of the current branch
    let output = process::Command::new("git")
        .args(["rev-parse", "@{u}"])
        .output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    // Fallback: origin/HEAD
    let output = process::Command::new("git")
        .args(["rev-parse", "origin/HEAD"])
        .output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    Err("Could not determine remote tracking branch. Set one with: git branch --set-upstream-to=origin/<branch>".into())
}

/// Compute changed files by diffing against a git ref.
/// Excludes deleted files and uses new paths for renames.
fn git_diff_files(base: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let output = process::Command::new("git")
        .args([
            "diff",
            "--name-only",
            "--diff-filter=d",
            &format!("{}..HEAD", base),
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    Ok(files)
}

fn cmd_open_and_wait(
    paths: &[PathBuf],
    line: Option<u32>,
    timeout: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let files = expand_paths(paths)?;
    if files.is_empty() {
        eprintln!("No files found");
        return Ok(());
    }

    let file_str = files[0].to_string_lossy();
    if let Err(e) = ensure_server_available() {
        eprintln!("Error: {}", e);
        process::exit(EXIT_NO_APP);
    }

    let session_id = server_client::review_start(&file_str, line, None)
        .ok_or("Could not start review session via Red Pen server")?;

    for file in files.iter().skip(1) {
        let _ = server_client::open_file_in_session(&file.to_string_lossy(), line, &session_id);
    }

    match server_client::review_wait(&session_id, timeout) {
        server_client::ReviewWaitResult::Ok(resp) => {
            let output = review_output(&files, resp.verdict.clone(), &session_id)?;
            println!("{}", serde_json::to_string_pretty(&output)?);
            if resp.verdict == "changes_requested" {
                print_rejection_summary(&files, &session_id)?;
                process::exit(EXIT_CHANGES_REQUESTED);
            }
            Ok(())
        }
        server_client::ReviewWaitResult::Timeout => {
            eprintln!("Review timed out. Push blocked.");
            process::exit(EXIT_TIMEOUT);
        }
        server_client::ReviewWaitResult::ServerUnavailable => {
            eprintln!("Lost connection to Red Pen server.");
            process::exit(EXIT_NO_APP);
        }
    }
}

fn cmd_open(paths: &[PathBuf], line: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    let files = expand_paths(paths)?;
    if files.is_empty() {
        eprintln!("No files found");
        return Ok(());
    }

    if server_client::is_available() {
        for file in &files {
            let path_str = file.to_string_lossy();
            if server_client::open_file(&path_str, line) {
                eprintln!("Opened {}", file.display());
            } else {
                eprintln!("Server failed to open {}", file.display());
            }
        }
        return Ok(());
    }

    // Fallback: deep links
    #[cfg(target_os = "macos")]
    ensure_app_running()?;

    for file in &files {
        let mut url = format!(
            "redpen://open?file={}",
            urlencoding::encode(&file.to_string_lossy())
        );
        if let Some(l) = line {
            url.push_str(&format!("&line={}", l));
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open").arg(&url).spawn()?;
        }
        eprintln!("Opening {}", file.display());
    }
    Ok(())
}

fn cmd_review_pr(
    pr_ref: &str,
    local_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_path_str = local_path.map(|path| path.to_string_lossy().to_string());

    if let Some(resp) = server_client::review_pr(pr_ref, local_path_str.as_deref()) {
        println!("{}", resp.worktree_path);
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        ensure_app_running()?;
        let mut url = format!("redpen://review-pr?pr={}", urlencoding::encode(pr_ref));
        if let Some(local_path) = local_path {
            url.push_str(&format!(
                "&localPath={}",
                urlencoding::encode(&local_path.to_string_lossy())
            ));
        }
        std::process::Command::new("open").arg(&url).spawn()?;
        println!("Opening PR review {}", pr_ref);
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Red Pen app server is not available. Start the app first.".into())
}

fn review_output(
    files: &[PathBuf],
    verdict: String,
    session_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut all_annotations = Vec::new();
    for file in files {
        let project_root = resolve_project_root(file);
        let sidecar = load_sidecar_for_file(&project_root, file)?;
        let mut file_obj = serde_json::Map::new();
        file_obj.insert(
            "file".to_string(),
            serde_json::Value::String(file.to_string_lossy().to_string()),
        );
        file_obj.insert(
            "annotations".to_string(),
            serde_json::to_value(&sidecar.annotations)?,
        );
        all_annotations.push(serde_json::Value::Object(file_obj));
    }

    Ok(serde_json::json!({
        "verdict": verdict,
        "session": session_id,
        "files": all_annotations,
    }))
}

#[cfg(target_os = "macos")]
fn ensure_server_available() -> Result<(), Box<dyn std::error::Error>> {
    if server_client::is_available() {
        return Ok(());
    }

    ensure_app_running()?;
    for _ in 0..25 {
        if server_client::is_available() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Err("Red Pen server did not become available after launching the app".into())
}

#[cfg(not(target_os = "macos"))]
fn ensure_server_available() -> Result<(), Box<dyn std::error::Error>> {
    if server_client::is_available() {
        Ok(())
    } else {
        Err("Red Pen server is not available. Start the app first.".into())
    }
}

const AGENT_PROMPT: &str = r#"# Red Pen — Agent Integration Guide

You have access to `redpen`, a code annotation and review tool with a desktop GUI.
Use it to get structured human feedback on code, plans, and design decisions.

## Core Commands

```bash
# Annotate a specific line (uses your name as author)
redpen annotate <file> --line <N> --body "Your comment" --author "<YourName>" --kind <kind>

# Annotation kinds: comment (default), explanation, lineNote, label
# Labels for categorization:
redpen annotate <file> --line 10 --body "Note" --label "question" --label "perf"

# Pose a question with choices (single-select = radio, multi-select = checkboxes)
redpen annotate <file> --line 42 \
  --body "Which approach do you prefer?" \
  --kind explanation --author "Claude" \
  --choice "Option A: typed errors" \
  --choice "Option B: string errors" \
  --choice "Option C: anyhow" \
  --selection-mode single

# Multi-select choices
redpen annotate <file> --line 20 \
  --body "Which improvements should I tackle?" \
  --kind explanation --author "Claude" \
  --choice "Add validation" \
  --choice "Improve logging" \
  --choice "Add tests" \
  --selection-mode multi

# Reply to an annotation (inherits parent's anchor)
redpen annotate <file> --body "Done — refactored to use typed errors" --reply-to <annotation-id>

# Open file(s) in the desktop app and wait for review
redpen open <file1> <file2> ... --wait --timeout 600

# Open all files changed since a git ref (for pre-push hooks)
redpen open --diff-base <sha> --wait --timeout 600

# List annotations as JSON
redpen list <file>

# List all annotations for a review session
redpen list --session <session-id>

# Check annotation counts
redpen status <file>
```

## The Review Loop

Use this pattern to get iterative human feedback:

```
┌─────────────────────────────────────────┐
│  1. Annotate files with questions,      │
│     explanations, and choices           │
│                                         │
│  2. Open files and wait for review      │
│     redpen open <files> --wait          │
│                                         │
│  3. Parse the JSON output               │
│     - verdict: approved | changes_req   │
│     - annotations with choices filled   │
│                                         │
│  4. Act on feedback                     │
│     - Implement requested changes       │
│     - Read selected choices             │
│     - Reply to each annotation          │
│                                         │
│  5. If not satisfied → go to 1          │
│     If approved → done                  │
└─────────────────────────────────────────┘
```

### Step-by-step

**1. Create annotations before requesting review:**
```bash
# Ask a design question with choices
redpen annotate src/auth.rs --line 15 \
  --body "I see two ways to handle token refresh. Which do you prefer?" \
  --kind explanation --author "Claude" \
  --choice "Eager: refresh 5 min before expiry" \
  --choice "Lazy: refresh on 401 response" \
  --selection-mode single

# Flag something for attention
redpen annotate src/auth.rs --line 42 \
  --body "This unwrap() could panic if the config file is missing. I'll add error handling." \
  --kind comment --author "Claude" --label "safety"
```

**2. Open and wait for the reviewer:**
```bash
output=$(redpen open src/auth.rs --wait --timeout 600)
```

**3. Parse the response:**
The output is JSON:
```json
{
  "verdict": "changes_requested",
  "session_id": "abc-123",
  "annotations": [
    {
      "id": "A1B2C3",
      "kind": "explanation",
      "body": "Which approach do you prefer?",
      "author": "Claude",
      "choices": [
        { "label": "Eager: refresh 5 min before expiry", "selected": true },
        { "label": "Lazy: refresh on 401 response", "selected": false }
      ],
      "selectionMode": "single",
      "anchor": { "range": { "startLine": 15 } }
    },
    {
      "id": "D4E5F6",
      "kind": "comment",
      "body": "Also add retry logic for network failures",
      "author": "reviewer_name",
      "anchor": { "range": { "startLine": 50 } }
    }
  ]
}
```

**4. Act on feedback and reply:**
```bash
# Implement the chosen approach
# ... make code changes ...

# Reply to confirm
redpen annotate src/auth.rs \
  --body "Done — implemented eager refresh with 5-min buffer" \
  --reply-to A1B2C3

# Reply to the reviewer's comment
redpen annotate src/auth.rs \
  --body "Added retry with exponential backoff (3 attempts)" \
  --reply-to D4E5F6
```

**5. Check if another round is needed:**
If the verdict was `changes_requested`, consider opening for review again after
implementing changes. If `approved`, proceed.

## Best Practices

### When to use choices
- Design decisions with clear alternatives
- Prioritization ("which should I do first?")
- Scope confirmation ("should I include X?")
- Use `single` for either/or decisions, `multi` for prioritization/inclusion

### Annotation kinds
- `comment` — general feedback, observations, flags
- `explanation` — agent-authored explanations, questions, proposals (shown with robot icon)
- `lineNote` — brief inline notes
- `label` — tagging/categorization

### Writing good annotations
- Be specific about what you're asking or flagging
- For choices, make each option self-contained and clear
- Use labels to categorize: "question", "safety", "perf", "design", "style"
- Keep bodies concise — the reviewer sees them inline in the code

### The loop
- Don't ask too many questions at once — 3-5 per file is ideal
- Group related questions on nearby lines
- After implementing changes, always reply to acknowledge each annotation
- Use `--timeout 600` (10 min) to avoid blocking indefinitely
- If the reviewer approves with no annotations, proceed with confidence

### Identity
- Always use `--author "<YourName>"` so your annotations show a robot icon
- Known agent names: claude, gpt, copilot, gemini, cursor, codex, agent
"#;
