use redpen_runtime::event_bus::{AppEvent, EventBus};
use tauri::{AppHandle, Emitter};

pub struct TauriEventBus {
    app_handle: AppHandle,
}

impl TauriEventBus {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl EventBus for TauriEventBus {
    fn emit(&self, event: AppEvent) {
        match &event {
            AppEvent::AnnotationsChanged { file_path } => {
                let _ = self.app_handle.emit("annotations-changed", file_path);
            }
            AppEvent::SettingsChanged => {
                let _ = self.app_handle.emit("settings-changed", ());
            }
            AppEvent::ReviewDone { file_path, verdict } => {
                let _ = self.app_handle.emit("review-done", (file_path, verdict));
            }
        }
    }
}
