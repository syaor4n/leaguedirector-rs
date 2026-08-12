use crate::detect;
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Lcu {
    http: Client,
    port: String,
    password: String,
}

impl Lcu {
    pub fn connect() -> Result<Self, String> {
        let lock = find_lockfile().ok_or_else(|| {
            "League Client not found (launch League, not just the replay).".to_string()
        })?;
        let raw = fs::read_to_string(&lock).map_err(|e| e.to_string())?;
        let parts: Vec<&str> = raw.trim().split(':').collect();
        if parts.len() < 5 {
            return Err(format!("invalid lockfile: {}", lock.display()));
        }
        let http = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(20))
            .no_proxy()
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            http,
            port: parts[2].to_string(),
            password: parts[3].to_string(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("https://127.0.0.1:{}{path}", self.port)
    }

    fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(self.url(path))
            .basic_auth("riot", Some(&self.password))
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("LCU GET {path} {}", resp.status()));
        }
        let text = resp.text().map_err(|e| e.to_string())?;
        if text.trim().is_empty() {
            return Ok(json!(null));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    fn post(&self, path: &str, body: &serde_json::Value) -> Result<(), String> {
        let resp = self
            .http
            .post(self.url(path))
            .basic_auth("riot", Some(&self.password))
            .json(body)
            .send()
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() || resp.status().as_u16() == 204 {
            Ok(())
        } else {
            let t = resp.text().unwrap_or_default();
            Err(format!("LCU POST {path}: {t}"))
        }
    }

    /// Copy the rofl into a League-readable folder and ask LCU to watch it.
    pub fn watch_rofl(&self, src: &Path) -> Result<PathBuf, String> {
        if !src.is_file() {
            return Err(format!("file not found: {}", src.display()));
        }
        let dest_dir = detect::preferred_replay_dir()
            .ok_or_else(|| "League Replays folder not found (install missing)".to_string())?;
        fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        let dest = dest_dir.join(src.file_name().unwrap_or_default());
        if dest != src {
            fs::copy(src, &dest).map_err(|e| format!("copy rofl: {e}"))?;
        }
        let game_id = game_id_from_rofl(&dest)
            .ok_or_else(|| format!("unexpected .rofl name: {}", dest.display()))?;

        let _ = self.post("/lol-replays/v1/rofls/scan", &json!({}));
        let _ = self.post(
            &format!("/lol-replays/v2/metadata/{game_id}/create"),
            &json!({
                "gameVersion": "",
                "gameType": "CLASSIC",
                "queueId": 420
            }),
        );
        self.post(
            &format!("/lol-replays/v1/rofls/{game_id}/watch"),
            &json!({ "contextData": "matchHistory" }),
        )?;
        Ok(dest)
    }

    pub fn replay_path(&self) -> Option<String> {
        self.get("/lol-replays/v1/rofls/path")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
    }
}

pub fn game_id_from_rofl(path: &Path) -> Option<u64> {
    path.file_stem()?
        .to_str()?
        .rsplit('-')
        .next()?
        .parse()
        .ok()
}

fn find_lockfile() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = detect::install_roots()
        .into_iter()
        .map(|root| {
            if root.extension().and_then(|s| s.to_str()) == Some("app") {
                root.join("Contents/LoL/lockfile")
            } else {
                root.join("lockfile")
            }
        })
        .collect();
    candidates.extend([
        PathBuf::from("/Applications/League of Legends.app/Contents/LoL/lockfile"),
        PathBuf::from("/Applications/League of Legends (PBE).app/Contents/LoL/lockfile"),
        PathBuf::from(
            "/Users/Shared/Riot Games/Metadata/league_of_legends.live/league_of_legends.live.lockfile",
        ),
    ]);
    candidates
        .into_iter()
        .find(|p| p.is_file() && p.metadata().map(|m| m.len() > 0).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_euw_replay_name() {
        assert_eq!(
            game_id_from_rofl(Path::new("/tmp/EUW1-7948514464.rofl")),
            Some(7948514464)
        );
    }
}
