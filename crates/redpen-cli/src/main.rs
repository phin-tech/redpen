use clap::{Parser, Subcommand};
use redpen_core::annotation::{Anchor, Annotation, AnnotationKind, Range};
use redpen_core::hash::{hash_file, hash_string};
use redpen_core::sidecar::SidecarFile;
use redpen_core::export::export_markdown;
use redpen_core::anchor::reanchor_annotations;
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
    /// Open a file in the desktop app
    Open {
        file: PathBuf,
        #[arg(long)]
        line: Option<u32>,
    },
    /// Wait for review to complete (blocks until "Done Reviewing" is clicked in the app)
    Wait {
        file: PathBuf,
        /// Timeout in seconds (default: no timeout)
        #[arg(long)]
        timeout: Option<u64>,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Annotate { file, line, range, body, label, author, kind } => {
            cmd_annotate(&file, line, range.as_deref(), &body, &label, &author, &kind)
        }
        Commands::List { file } => cmd_list(&file),
        Commands::Export { file, output } => cmd_export(&file, output.as_deref()),
        Commands::Status { file } => cmd_status(&file),
        Commands::Open { file, line } => cmd_open(&file, line),
        Commands::Wait { file, timeout } => cmd_wait(&file, timeout),
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
    let start: Vec<u32> = parts[0].split(':').map(|s| s.parse().unwrap_or(0)).collect();
    let end: Vec<u32> = parts[1].split(':').map(|s| s.parse().unwrap_or(0)).collect();
    if start.len() != 2 || end.len() != 2 {
        return Err("Range format: startLine:startCol-endLine:endCol".to_string());
    }
    Ok(Range { start_line: start[0], start_column: start[1], end_line: end[0], end_column: end[1] })
}

fn parse_kind(kind: &str) -> Result<AnnotationKind, String> {
    match kind {
        "comment" => Ok(AnnotationKind::Comment),
        "lineNote" | "line-note" | "linenote" => Ok(AnnotationKind::LineNote),
        "label" => Ok(AnnotationKind::Label),
        other => Err(format!("Unknown kind: {}. Use comment, lineNote, or label.", other)),
    }
}

fn resolve_project_root(source_path: &Path) -> PathBuf {
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    }
}

fn load_or_create_sidecar(project_root: &Path, source_path: &Path) -> Result<SidecarFile, Box<dyn std::error::Error>> {
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

fn cmd_annotate(
    file: &Path, line: Option<u32>, range_str: Option<&str>,
    body: &str, labels: &[String], author: &str, kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let content = fs::read_to_string(&abs_path)?;
    let source_lines: Vec<&str> = content.lines().collect();

    let range = if let Some(r) = range_str {
        parse_range(r)?
    } else {
        let l = line.unwrap_or(1);
        let line_len = source_lines.get((l - 1) as usize).map(|s| s.len() as u32).unwrap_or(0);
        Range { start_line: l, start_column: 0, end_line: l, end_column: line_len }
    };

    let line_idx = (range.start_line as usize).saturating_sub(1);
    let line_content = source_lines.get(line_idx).unwrap_or(&"").to_string();

    let start = line_idx.saturating_sub(2);
    let end = (line_idx + 3).min(source_lines.len());
    let surrounding_lines: Vec<String> = source_lines[start..end].iter().map(|s| s.to_string()).collect();

    // Range is Copy, so range.start_line works after moving range into the struct
    let start_line = range.start_line;
    let anchor = Anchor::TextContext {
        line_content: line_content.clone(),
        surrounding_lines,
        content_hash: hash_string(&line_content),
        range,
        last_known_line: start_line,
    };

    let annotation = Annotation::new(
        parse_kind(kind)?, body.to_string(), labels.to_vec(), author.to_string(), anchor,
    );

    let project_root = resolve_project_root(&abs_path);
    let mut sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
    let id = annotation.id.clone();
    sidecar.add_annotation(annotation);
    sidecar.save_for_source(&project_root, &abs_path)?;

    println!("Created annotation {} on line {}", id, start_line);
    Ok(())
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
        println!("{}: {} annotations ({} orphaned)", file.display(), total, orphaned);
    } else {
        println!("{}: {} annotations", file.display(), total);
    }
    Ok(())
}

fn cmd_wait(file: &Path, timeout: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let project_root = resolve_project_root(&abs_path);
    let signal_path = SidecarFile::signal_path(&project_root, &abs_path);

    let start = std::time::Instant::now();
    let timeout_duration = timeout.map(std::time::Duration::from_secs);

    eprintln!("Waiting for review of {}...", file.display());

    loop {
        if signal_path.exists() {
            // Clean up signal file
            let _ = fs::remove_file(&signal_path);

            // Output annotations as JSON
            let sidecar = load_or_create_sidecar(&project_root, &abs_path)?;
            let json = serde_json::to_string_pretty(&sidecar.annotations)?;
            println!("{}", json);
            return Ok(());
        }

        if let Some(dur) = timeout_duration {
            if start.elapsed() > dur {
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
        .args(["-x", "Red Pen"])
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

fn cmd_open(file: &Path, line: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    let abs_path = fs::canonicalize(file)?;
    let mut url = format!("redpen://open?file={}", urlencoding::encode(&abs_path.to_string_lossy()));
    if let Some(l) = line {
        url.push_str(&format!("&line={}", l));
    }
    #[cfg(target_os = "macos")]
    {
        ensure_app_running()?;
        std::process::Command::new("open").arg(&url).spawn()?;
    }
    println!("Opening {}", url);
    Ok(())
}
