use crate::settings::{settings_path, AppSettings};
use crate::storage::StateDb;
use crate::workspace_index::WorkspaceIndexService;
use redpen_server::ReviewSessions;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub settings: Arc<Mutex<AppSettings>>,
    pub settings_path: PathBuf,
    pub pending_deep_links: Mutex<Vec<String>>,
    pub workspace_index: WorkspaceIndexService,
    pub storage: StateDb,
    pub review_sessions: Arc<ReviewSessions>,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let settings_path = settings_path()?;
        let settings = Arc::new(Mutex::new(AppSettings::load_or_default(&settings_path)?));
        let workspace_index = WorkspaceIndexService::new(settings.clone());
        let storage = StateDb::new().map_err(|e| e.to_string())?;
        let review_sessions = Arc::new(ReviewSessions::new());

        Ok(Self {
            settings,
            settings_path,
            pending_deep_links: Mutex::new(Vec::new()),
            workspace_index,
            storage,
            review_sessions,
        })
    }
}
