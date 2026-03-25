use redpen_core::anchor::reanchor_annotations;
use redpen_core::annotation::Annotation;
use redpen_core::hash::hash_file;
use redpen_core::sidecar::SidecarFile;
use redpen_server::AppBridge;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Implements `AppBridge` by delegating to Tauri's `AppHandle` and the sidecar store.
pub struct TauriBridge {
    handle: AppHandle,
}

impl TauriBridge {
    pub fn new(handle: AppHandle) -> Arc<Self> {
        Arc::new(Self { handle })
    }
}

impl AppBridge for TauriBridge {
    fn open_file(&self, file: &str, line: Option<u32>) -> Result<(), String> {
        let mut url = format!(
            "redpen://open?file={}",
            urlencoding::encode(file)
        );
        if let Some(l) = line {
            url.push_str(&format!("&line={}", l));
        }
        self.handle
            .emit("deep-link-open", &url)
            .map_err(|e| e.to_string())
    }

    fn refresh_file(&self, file: &str) -> Result<(), String> {
        let url = format!(
            "redpen://refresh?file={}",
            urlencoding::encode(file)
        );
        self.handle
            .emit("deep-link-open", &url)
            .map_err(|e| e.to_string())
    }

    fn get_annotations(&self, file: &str) -> Result<Vec<Annotation>, String> {
        let source_path = Path::new(file);
        let project_root = resolve_project_root(source_path);
        let sidecar = load_sidecar_for_file(&project_root, source_path)
            .map_err(|e| e.to_string())?;
        Ok(sidecar.annotations)
    }
}

fn resolve_project_root(source_path: &Path) -> std::path::PathBuf {
    match git2::Repository::discover(source_path) {
        Ok(repo) => repo.workdir().unwrap().to_path_buf(),
        Err(_) => dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")),
    }
}

fn load_sidecar_for_file(
    project_root: &Path,
    source_path: &Path,
) -> Result<SidecarFile, String> {
    let annotation_path = SidecarFile::annotation_path(project_root, source_path);
    if annotation_path.exists() {
        let mut sidecar = SidecarFile::load(&annotation_path).map_err(|e| e.to_string())?;
        let current_hash = hash_file(source_path).map_err(|e| e.to_string())?;
        if sidecar.source_file_hash != current_hash {
            let content = fs::read_to_string(source_path).map_err(|e| e.to_string())?;
            reanchor_annotations(&mut sidecar.annotations, &content);
            sidecar.source_file_hash = current_hash;
        }
        Ok(sidecar)
    } else {
        let hash = hash_file(source_path).map_err(|e| e.to_string())?;
        Ok(SidecarFile::new(hash))
    }
}
