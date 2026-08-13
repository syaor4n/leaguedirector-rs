use crate::bindings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub record_dir: String,
    pub last_tab: usize,
    pub show_hud: bool,
    pub sequence_dir: String,
    pub last_sequence: String,
    pub last_look: String,
    pub bindings: BTreeMap<String, String>,
    pub clips: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            record_dir: crate::detect::preferred_record_dir().display().to_string(),
            last_tab: 0,
            show_hud: true,
            sequence_dir: sequences_dir().display().to_string(),
            last_sequence: String::new(),
            last_look: String::new(),
            bindings: bindings::default_map(),
            clips: Vec::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|u| u.home_dir().join("Documents/LeagueDirector"))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn sequences_dir() -> PathBuf {
    config_dir().join("sequences")
}

#[allow(dead_code)]
pub fn default_record_dir() -> PathBuf {
    config_dir().join("recordings")
}

impl Settings {
    pub fn load() -> Self {
        let path = config_dir().join("config.json");
        let mut s: Settings = fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        bindings::merge_defaults(&mut s.bindings);
        if s.sequence_dir.trim().is_empty() {
            s.sequence_dir = sequences_dir().display().to_string();
        }
        s
    }

    pub fn save(&self) {
        let dir = config_dir();
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(&self.record_dir);
        let _ = fs::create_dir_all(&self.sequence_dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(dir.join("config.json"), json);
        }
    }

    pub fn remember_clip(&mut self, path: &std::path::Path) {
        let s = path.display().to_string();
        self.clips.retain(|p| p != &s);
        self.clips.insert(0, s);
        if self.clips.len() > 24 {
            self.clips.truncate(24);
        }
        self.save();
    }
}
