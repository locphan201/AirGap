use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub last_vault_path: Option<PathBuf>,
    pub theme_dark: bool,
    pub font_size: f32,
    pub autosave_interval_ms: u64,
    pub line_numbers: bool,
    pub word_wrap: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_vault_path: None,
            theme_dark: true,
            font_size: 14.0,
            autosave_interval_ms: 1000,
            line_numbers: true,
            word_wrap: true,
        }
    }
}

pub fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "AirGap", "AirGap").map(|proj| proj.config_dir().to_path_buf())
}

pub fn load_config() -> AppConfig {
    if let Some(config_dir) = get_config_dir() {
        let config_file = config_dir.join("config.json");
        if let Ok(content) = fs::read_to_string(config_file) {
            if let Ok(cfg) = serde_json::from_str(&content) {
                return cfg;
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) -> Result<(), std::io::Error> {
    if let Some(config_dir) = get_config_dir() {
        fs::create_dir_all(&config_dir)?;
        let config_file = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(config)?;
        fs::write(config_file, content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_and_serde() {
        let default_cfg = AppConfig::default();
        let json = serde_json::to_string(&default_cfg).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(default_cfg, deserialized);
    }
}
