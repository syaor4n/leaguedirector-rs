use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub record_dir: String,
    pub last_tab: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            record_dir: crate::detect::preferred_record_dir().display().to_string(),
            last_tab: 0,
        }
    }
}

pub fn config_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|u| u.home_dir().join("Documents/LeagueDirector"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[allow(dead_code)]
pub fn default_record_dir() -> PathBuf {
    config_dir().join("recordings")
}

impl Settings {
    pub fn load() -> Self {
        let path = config_dir().join("config.json");
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let dir = config_dir();
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(&self.record_dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(dir.join("config.json"), json);
        }
    }
}
