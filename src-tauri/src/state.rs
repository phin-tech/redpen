use crate::settings::{settings_path, AppSettings};
use crate::workspace_index::WorkspaceIndexService;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub settings: Arc<Mutex<AppSettings>>,
    pub settings_path: PathBuf,
    pub pending_deep_links: Mutex<Vec<String>>,
    pub workspace_index: WorkspaceIndexService,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let settings_path = settings_path()?;
        let settings = Arc::new(Mutex::new(AppSettings::load_or_default(&settings_path)?));
        let workspace_index = WorkspaceIndexService::new(settings.clone());

        Ok(Self {
            settings,
            settings_path,
            pending_deep_links: Mutex::new(Vec::new()),
            workspace_index,
        })
    }
}
