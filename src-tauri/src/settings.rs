use crate::storage::{
    app_home_path as storage_app_home_path, settings_path as storage_settings_path,
    SETTINGS_FILE_NAME,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[cfg(test)]
pub use crate::storage::APP_HOME_DIRECTORY;

fn default_checkout_root() -> Option<String> {
    storage_app_home_path()
        .ok()
        .map(|home| home.join("checkouts"))
        .map(|path| path.to_string_lossy().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct NotificationSettings {
    pub annotation_reply: bool,
    pub review_complete: bool,
    pub new_annotation: bool,
    pub deep_link: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            annotation_reply: true,
            review_complete: true,
            new_annotation: false,
            deep_link: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct TrackedRepo {
    pub repo: String,
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct AppSettings {
    pub author: String,
    #[serde(default)]
    pub default_labels: Vec<String>,
    #[serde(default)]
    pub ignored_folder_names: Vec<String>,
    #[serde(default = "default_diff_algorithm")]
    pub diff_algorithm: String,
    #[serde(default = "default_checkout_root")]
    pub default_checkout_root: Option<String>,
    #[serde(default)]
    pub tracked_github_repos: Vec<TrackedRepo>,
    #[serde(default)]
    pub notifications: NotificationSettings,
}

fn default_diff_algorithm() -> String {
    "patience".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            author: whoami::username(),
            default_labels: Vec::new(),
            ignored_folder_names: Vec::new(),
            diff_algorithm: default_diff_algorithm(),
            default_checkout_root: default_checkout_root(),
            tracked_github_repos: Vec::new(),
            notifications: NotificationSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct UpdateSettingsRequest {
    pub author: Option<String>,
    pub default_labels: Option<Vec<String>>,
    pub ignored_folder_names: Option<Vec<String>>,
    pub diff_algorithm: Option<String>,
    pub default_checkout_root: Option<String>,
    pub tracked_github_repos: Option<Vec<TrackedRepo>>,
    pub notifications: Option<NotificationSettings>,
}

impl UpdateSettingsRequest {
    pub fn apply(self, settings: &mut AppSettings) {
        if let Some(author) = self.author {
            settings.author = author;
        }
        if let Some(default_labels) = self.default_labels {
            settings.default_labels = default_labels;
        }
        if let Some(ignored_folder_names) = self.ignored_folder_names {
            settings.ignored_folder_names = normalize_ignored_folder_names(ignored_folder_names);
        }
        if let Some(diff_algorithm) = self.diff_algorithm {
            settings.diff_algorithm = diff_algorithm;
        }
        if let Some(default_checkout_root) = self.default_checkout_root {
            settings.default_checkout_root = normalize_optional_path(Some(default_checkout_root));
        }
        if let Some(tracked_github_repos) = self.tracked_github_repos {
            settings.tracked_github_repos = normalize_tracked_repos(tracked_github_repos);
        }
        if let Some(notifications) = self.notifications {
            settings.notifications = notifications;
        }
    }
}

impl AppSettings {
    pub fn load_or_default(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut settings: Self = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        settings.ignored_folder_names =
            normalize_ignored_folder_names(settings.ignored_folder_names.clone());
        settings.default_checkout_root =
            normalize_optional_path(settings.default_checkout_root.clone());
        settings.tracked_github_repos =
            normalize_tracked_repos(settings.tracked_github_repos.clone());
        Ok(settings)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        let directory = path
            .parent()
            .ok_or_else(|| "settings path has no parent directory".to_string())?;
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;

        let temp_path = directory.join(format!(".{}.tmp", SETTINGS_FILE_NAME));
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&temp_path, content).map_err(|e| e.to_string())?;
        fs::rename(&temp_path, path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn app_home_path() -> Result<PathBuf, String> {
    storage_app_home_path().map_err(|e| e.to_string())
}

pub fn settings_path() -> Result<PathBuf, String> {
    storage_settings_path().map_err(|e| e.to_string())
}

pub fn normalize_ignored_folder_names(names: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();

    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            continue;
        }
        let folder_name = trimmed
            .trim_matches(std::path::MAIN_SEPARATOR)
            .trim_matches('/')
            .to_string();
        if folder_name.is_empty() {
            continue;
        }
        if normalized.iter().any(|existing| existing == &folder_name) {
            continue;
        }
        normalized.push(folder_name);
    }

    normalized
}

pub fn normalize_tracked_repos(repos: Vec<TrackedRepo>) -> Vec<TrackedRepo> {
    let mut normalized = Vec::new();

    for repo in repos {
        let repo_name = repo.repo.trim().trim_matches('/').to_string();
        let local_path = normalize_path_string(repo.local_path);

        if repo_name.is_empty() || local_path.is_empty() {
            continue;
        }

        if normalized
            .iter()
            .any(|existing: &TrackedRepo| existing.repo.eq_ignore_ascii_case(&repo_name))
        {
            continue;
        }

        normalized.push(TrackedRepo {
            repo: repo_name,
            local_path,
        });
    }

    normalized
}

pub fn normalize_optional_path(path: Option<String>) -> Option<String> {
    path.and_then(|value| {
        let normalized = normalize_path_string(value);
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

fn normalize_path_string(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed == "~" {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from(trimmed))
            .to_string_lossy()
            .to_string();
    }

    if let Some(rest) = trimmed.strip_prefix("~/") {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from(trimmed))
            .join(rest)
            .to_string_lossy()
            .to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        app_home_path, default_checkout_root, settings_path, AppSettings, NotificationSettings,
        TrackedRepo, UpdateSettingsRequest, APP_HOME_DIRECTORY,
    };
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn loads_defaults_when_settings_file_is_missing() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");

        let settings = AppSettings::load_or_default(&path).unwrap();

        assert_eq!(settings.default_labels, Vec::<String>::new());
        assert_eq!(settings.ignored_folder_names, Vec::<String>::new());
        assert_eq!(settings.default_checkout_root, default_checkout_root());
        assert!(!settings.author.is_empty());
    }

    #[test]
    fn saves_and_reloads_settings() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");

        let settings = AppSettings {
            author: "sam".to_string(),
            default_labels: vec!["todo".to_string(), "bug".to_string()],
            ignored_folder_names: vec!["node_modules".to_string(), ".venv".to_string()],
            default_checkout_root: Some("/tmp/checkouts".to_string()),
            tracked_github_repos: vec![TrackedRepo {
                repo: "phin-tech/redpen".to_string(),
                local_path: "/tmp/redpen".to_string(),
            }],
            ..AppSettings::default()
        };

        settings.save_to_path(&path).unwrap();
        let reloaded = AppSettings::load_or_default(&path).unwrap();

        assert_eq!(reloaded, settings);
    }

    #[test]
    fn update_request_normalizes_ignored_folder_names() {
        let mut settings = AppSettings::default();

        UpdateSettingsRequest {
            author: None,
            default_labels: None,
            ignored_folder_names: Some(vec![
                "node_modules".to_string(),
                "/node_modules/".to_string(),
                " .venv ".to_string(),
                "".to_string(),
            ]),
            diff_algorithm: None,
            default_checkout_root: None,
            tracked_github_repos: None,
            notifications: None,
        }
        .apply(&mut settings);

        assert_eq!(
            settings.ignored_folder_names,
            vec!["node_modules".to_string(), ".venv".to_string()]
        );
    }

    #[test]
    fn settings_path_uses_dot_config_redpen() {
        let path = settings_path().unwrap();
        let suffix = PathBuf::from(APP_HOME_DIRECTORY).join("settings.json");

        assert!(path.ends_with(suffix));
    }

    #[test]
    fn app_home_path_uses_dot_config_redpen() {
        let path = app_home_path().unwrap();
        let suffix = PathBuf::from(APP_HOME_DIRECTORY);

        assert!(path.ends_with(suffix));
    }

    #[test]
    fn notification_settings_serde_roundtrip() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");

        let settings = AppSettings {
            author: "sam".to_string(),
            default_labels: vec![],
            ignored_folder_names: vec![],
            default_checkout_root: None,
            tracked_github_repos: vec![],
            notifications: NotificationSettings {
                annotation_reply: false,
                review_complete: true,
                new_annotation: true,
                deep_link: false,
            },
            ..AppSettings::default()
        };

        settings.save_to_path(&path).unwrap();
        let reloaded = AppSettings::load_or_default(&path).unwrap();
        assert_eq!(reloaded.notifications, settings.notifications);
    }

    #[test]
    fn loads_settings_without_notifications_key() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");

        // Write a settings file without the notifications key (old format)
        let json = r#"{"author":"sam","defaultLabels":[],"ignoredFolderNames":[]}"#;
        std::fs::write(&path, json).unwrap();

        let settings = AppSettings::load_or_default(&path).unwrap();
        // Should get defaults
        assert!(settings.notifications.annotation_reply);
        assert!(!settings.notifications.new_annotation);
    }

    #[test]
    fn tracked_repos_are_normalized() {
        let mut settings = AppSettings::default();

        UpdateSettingsRequest {
            author: None,
            default_labels: None,
            ignored_folder_names: None,
            diff_algorithm: None,
            default_checkout_root: None,
            tracked_github_repos: Some(vec![
                TrackedRepo {
                    repo: " phin-tech/redpen ".to_string(),
                    local_path: " /tmp/redpen ".to_string(),
                },
                TrackedRepo {
                    repo: "phin-tech/redpen".to_string(),
                    local_path: "/tmp/other".to_string(),
                },
            ]),
            notifications: None,
        }
        .apply(&mut settings);

        assert_eq!(
            settings.tracked_github_repos,
            vec![TrackedRepo {
                repo: "phin-tech/redpen".to_string(),
                local_path: "/tmp/redpen".to_string(),
            }]
        );
    }

    #[test]
    fn default_checkout_root_is_normalized() {
        let mut settings = AppSettings::default();

        UpdateSettingsRequest {
            author: None,
            default_labels: None,
            ignored_folder_names: None,
            diff_algorithm: None,
            default_checkout_root: Some(" /tmp/checkouts ".to_string()),
            tracked_github_repos: None,
            notifications: None,
        }
        .apply(&mut settings);

        assert_eq!(
            settings.default_checkout_root,
            Some("/tmp/checkouts".to_string())
        );
    }

    #[test]
    fn tracked_repo_paths_expand_home_shortcut() {
        let mut settings = AppSettings::default();

        UpdateSettingsRequest {
            author: None,
            default_labels: None,
            ignored_folder_names: None,
            diff_algorithm: None,
            default_checkout_root: None,
            tracked_github_repos: Some(vec![TrackedRepo {
                repo: "phin-tech/test-repo".to_string(),
                local_path: "~/src/test-repo".to_string(),
            }]),
            notifications: None,
        }
        .apply(&mut settings);

        assert!(settings.tracked_github_repos[0].local_path.starts_with('/'));
        assert!(settings.tracked_github_repos[0]
            .local_path
            .ends_with("/src/test-repo"));
    }

    #[test]
    fn notification_settings_defaults() {
        let settings = AppSettings::default();
        assert!(settings.notifications.annotation_reply);
        assert!(settings.notifications.review_complete);
        assert!(!settings.notifications.new_annotation);
        assert!(settings.notifications.deep_link);
    }
}
