use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub db_path: String,
    pub auto_lock_minutes: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            auto_lock_minutes: 5,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "cosmic-keepass")?;
    Some(dirs.config_dir().join("config.json"))
}

pub fn load_config() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    if !path.exists() {
        return Config::default();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_config(config: &Config) {
    let Some(path) = config_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(&path, json);
    }
}
