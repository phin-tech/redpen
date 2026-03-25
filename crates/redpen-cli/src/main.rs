mod server_client;

use clap::{Parser, Subcommand};
use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind, Range};
use redpen_core::export::export_markdown;
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use std::fs;
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
    },
    /// List all annotations as JSON
    List { file: PathBuf },
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
        /// Files or directories to open
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        #[arg(long)]
        line: Option<u32>,
        /// Block until review is complete (combines open + wait)
        #[arg(long)]
        wait: bool,
        /// Timeout in seconds when using --wait (default: no timeout)
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Wait for review to complete (blocks until "Done Reviewing" is clicked in the app)
    Wait {
        /// Files or directories to wait on
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Timeout in seconds (default: no timeout)
        #[arg(long)]
        timeout: Option<u64>,
    },
}

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
        } => cmd_annotate(
            &file,
            line,
            range.as_deref(),
            &body,
            &label,
            &author,
            &kind,
            reply_to.as_deref(),
        ),
        Commands::List { file } => cmd_list(&file),
        Commands::Export { file, output } => cmd_export(&file, output.as_deref()),
        Commands::Status { file } => cmd_status(&file),
        Commands::Open {
            paths,
            line,
            wait,
            timeout,
        } => {
            if wait {
                // Combined open + wait — server handles both in one call
                cmd_open_and_wait(&paths, line, timeout)
            } else {
                cmd_open(&paths, line)
            }
        }
        Commands::Wait { paths, timeout } => cmd_wait(&paths, timeout),
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
        other => Err(format!(
            "Unknown kind: {}. Use comment, lineNote, or label.",
            other
        )),
    }
}

fn resolve_project_root(source_path: &Path) -> PathBuf {
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    }
}

fn load_or_create_sidecar(
    project_root: &Path,
    source_path: &Path,
) -> Result<SidecarFile, Box<dyn std::error::Error>> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if annotation_path.exists() {
        let mut sidecar = SidecarFile::load(&annotation_path)?;
        let current_hash = hash_file(source_path)?;
        if sidecar.source_file_hash != current_hash {
            let content = fs::read_to_string(source_path)?;
            reanchor_annotations(&mut sidecar.annotations, &content);
            sidecar.source_file_hash = current_hash;
        }
        Ok(sidecar)
    } else {
        let hash = hash_file(source_path)?;
        Ok(SidecarFile::new(hash))
    }
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
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);

    // If replying, clone parent's anchor instead of building a new one
    if let Some(parent_id) = reply_to {
        let mut sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
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
        sidecar.save_for_source(&project_root, &abs_path)?;
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

    let annotation = Annotation::new(
        parse_kind(kind)?,
        body.to_string(),
        labels.to_vec(),
        author.to_string(),
        anchor,
    );

    let mut sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
    let id = annotation.id.clone();
    sidecar.add_annotation(annotation);
    sidecar.save_for_source(&project_root, &abs_path)?;
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

fn cmd_list(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);
    let sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
    let json = serde_json::to_string_pretty(&sidecar.annotations)?;
    println!("{}", json);
    Ok(())
}

fn cmd_export(file: &Path, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);
    let sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
    let content = fs::read_to_string(&abs_path)?;
    let file_name = abs_path.file_name().unwrap().to_string_lossy().to_string();
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
    let annotation_path = SidecarFile::annotation_path(&project_root, &abs_path);
    if !annotation_path.exists() {
        println!("{}: no annotations", file.display());
        return Ok(());
    }
    let sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
    let total = sidecar.annotations.len();
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

fn cmd_wait(paths: &[PathBuf], timeout: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
    let files = expand_paths(paths)?;
    if files.is_empty() {
        eprintln!("No files found");
        return Ok(());
    }

    // Try the HTTP server first — no signal files, no polling
    if server_client::is_available() {
        return cmd_wait_via_server(&files, timeout);
    }

    // Fallback: signal file polling
    cmd_wait_via_signals(&files, timeout)
}

fn cmd_wait_via_server(
    files: &[PathBuf],
    timeout: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_str = files[0].to_string_lossy();
    eprintln!(
        "Waiting for review of {} file(s) via server...",
        files.len()
    );

    match server_client::review(&file_str, None, timeout) {
        Some(resp) => {
            let output = serde_json::json!({
                "verdict": resp.verdict,
                "session_id": resp.session_id,
                "annotations": resp.annotations,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(())
        }
        None => {
            eprintln!("Server review failed, falling back to signal files...");
            cmd_wait_via_signals(files, timeout)
        }
    }
}

fn cmd_wait_via_signals(
    files: &[PathBuf],
    timeout: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_root = resolve_project_root(&files[0]);
    let session_signal = SidecarFile::session_signal_path(&project_root);
    let session_file = SidecarFile::session_file_path(&project_root);

    let session_id = uuid::Uuid::new_v4().to_string();
    if let Some(parent) = session_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&session_file, &session_id)?;

    let _ = fs::remove_file(&session_signal);

    let start = std::time::Instant::now();
    let timeout_duration = timeout.map(std::time::Duration::from_secs);

    eprintln!(
        "Waiting for review of {} file(s)... [session {}]",
        files.len(),
        &session_id[..8]
    );

    loop {
        if session_signal.exists() {
            let content = fs::read_to_string(&session_signal).unwrap_or_default();
            let mut lines = content.lines();
            let signal_session = lines.next().unwrap_or("");
            let verdict = lines.next().unwrap_or("approved");

            if signal_session != session_id {
                let _ = fs::remove_file(&session_signal);
                continue;
            }

            let _ = fs::remove_file(&session_signal);
            let _ = fs::remove_file(&session_file);

            let mut all_annotations: Vec<serde_json::Value> = Vec::new();
            for file in files {
                let annotation_path = SidecarFile::annotation_path(&project_root, file);
                if annotation_path.exists() {
                    let sidecar = load_or_create_sidecar(&project_root, file)?;
                    if !sidecar.annotations.is_empty() {
                        let relative = file.strip_prefix(&project_root).unwrap_or(file);
                        let mut file_obj = serde_json::Map::new();
                        file_obj.insert(
                            "file".to_string(),
                            serde_json::Value::String(relative.to_string_lossy().to_string()),
                        );
                        file_obj.insert(
                            "annotations".to_string(),
                            serde_json::to_value(&sidecar.annotations)?,
                        );
                        all_annotations.push(serde_json::Value::Object(file_obj));
                    }
                }
            }

            let mut output = serde_json::Map::new();
            output.insert(
                "verdict".to_string(),
                serde_json::Value::String(verdict.trim().to_string()),
            );
            output.insert(
                "files".to_string(),
                serde_json::Value::Array(all_annotations),
            );

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::Value::Object(output))?
            );
            return Ok(());
        }

        if let Some(dur) = timeout_duration {
            if start.elapsed() > dur {
                let _ = fs::remove_file(&session_file);
                eprintln!("Timed out waiting for review");
                process::exit(2);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
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

    // Try the server's combined review endpoint (opens + blocks in one call)
    let file_str = files[0].to_string_lossy();
    if let Some(resp) = server_client::review(&file_str, line, timeout) {
        let output = serde_json::json!({
            "verdict": resp.verdict,
            "session_id": resp.session_id,
            "annotations": resp.annotations,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Fallback: open via deep link, then wait via signal files
    cmd_open(
        &files.iter().map(|f| f.to_path_buf()).collect::<Vec<_>>(),
        line,
    )?;
    cmd_wait_via_signals(&files, timeout)
}

fn cmd_open(paths: &[PathBuf], line: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    let files = expand_paths(paths)?;
    if files.is_empty() {
        eprintln!("No files found");
        return Ok(());
    }

    // Try the HTTP server first
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
