use std::sync::Mutex;

pub struct AppState {
    pub author: Mutex<String>,
    pub default_labels: Mutex<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            author: Mutex::new(whoami::username()),
            default_labels: Mutex::new(Vec::new()),
        }
    }
}
