use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;

pub const CONFIG_DIRECTORY: &str = ".config/redpen";
pub const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/bindings/")]
pub struct AppSettings {
    pub author: String,
    #[serde(default)]
    pub default_labels: Vec<String>,
    #[serde(default)]
    pub ignored_folder_names: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            author: whoami::username(),
            default_labels: Vec::new(),
            ignored_folder_names: Vec::new(),
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

pub fn settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "could not resolve home directory".to_string())?;
    Ok(home.join(CONFIG_DIRECTORY).join(SETTINGS_FILE_NAME))
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

#[cfg(test)]
mod tests {
    use super::{settings_path, AppSettings, UpdateSettingsRequest, CONFIG_DIRECTORY};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn loads_defaults_when_settings_file_is_missing() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");

        let settings = AppSettings::load_or_default(&path).unwrap();

        assert_eq!(settings.default_labels, Vec::<String>::new());
        assert_eq!(settings.ignored_folder_names, Vec::<String>::new());
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
        let suffix = PathBuf::from(CONFIG_DIRECTORY).join("settings.json");

        assert!(path.ends_with(suffix));
    }
}
