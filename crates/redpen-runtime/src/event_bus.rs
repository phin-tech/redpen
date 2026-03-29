#[derive(Debug, Clone)]
pub enum AppEvent {
    AnnotationsChanged {
        file_path: String,
    },
    SettingsChanged,
    ReviewDone {
        file_path: String,
        verdict: Option<String>,
    },
}

pub trait EventBus: Send + Sync {
    fn emit(&self, event: AppEvent);
}

pub struct NoOpEventBus;

impl EventBus for NoOpEventBus {
    fn emit(&self, _event: AppEvent) {}
}
