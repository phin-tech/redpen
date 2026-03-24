use crate::settings::{AppSettings, NotificationSettings};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    AnnotationReply,
    ReviewComplete,
    NewAnnotation,
    DeepLink,
}

impl NotificationSettings {
    pub fn is_enabled(&self, kind: NotificationKind) -> bool {
        match kind {
            NotificationKind::AnnotationReply => self.annotation_reply,
            NotificationKind::ReviewComplete => self.review_complete,
            NotificationKind::NewAnnotation => self.new_annotation,
            NotificationKind::DeepLink => self.deep_link,
        }
    }
}

impl NotificationKind {
    /// Generate default notification title and body for a given kind.
    pub fn default_title_body(&self, file_name: &str, line: Option<u32>) -> (String, String) {
        let line_str = line.map(|l| format!(":{}", l)).unwrap_or_default();
        match self {
            NotificationKind::AnnotationReply => (
                "Agent replied".to_string(),
                format!("New reply on {}{}", file_name, line_str),
            ),
            NotificationKind::ReviewComplete => (
                "Review complete".to_string(),
                "Changes applied, ready for re-review".to_string(),
            ),
            NotificationKind::NewAnnotation => (
                "New annotation".to_string(),
                format!("New comment on {}{}", file_name, line_str),
            ),
            NotificationKind::DeepLink => (
                "Opening file".to_string(),
                format!("{}{}", file_name, line_str),
            ),
        }
    }
}

pub struct NotificationService {
    app_handle: AppHandle,
}

impl NotificationService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Send an OS notification if the given kind is enabled in settings.
    /// Note: NotificationService is intentionally not stored in managed state —
    /// it's a thin wrapper around AppHandle with no internal state, so creating
    /// it fresh per call is fine. The spec's deep_link parameter is omitted
    /// because Tauri v2 desktop notifications don't support click actions.
    pub fn send(
        &self,
        kind: NotificationKind,
        title: &str,
        body: &str,
        settings: &AppSettings,
    ) -> Result<(), String> {
        if !settings.notifications.is_enabled(kind) {
            return Ok(());
        }
        self.app_handle
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::NotificationSettings;

    #[test]
    fn is_enabled_respects_settings() {
        let settings = NotificationSettings {
            annotation_reply: true,
            review_complete: false,
            new_annotation: false,
            deep_link: true,
        };

        assert!(settings.is_enabled(NotificationKind::AnnotationReply));
        assert!(!settings.is_enabled(NotificationKind::ReviewComplete));
        assert!(!settings.is_enabled(NotificationKind::NewAnnotation));
        assert!(settings.is_enabled(NotificationKind::DeepLink));
    }

    #[test]
    fn is_enabled_defaults_match_spec() {
        let settings = NotificationSettings::default();

        assert!(settings.is_enabled(NotificationKind::AnnotationReply));
        assert!(settings.is_enabled(NotificationKind::ReviewComplete));
        assert!(!settings.is_enabled(NotificationKind::NewAnnotation));
        assert!(settings.is_enabled(NotificationKind::DeepLink));
    }
}
