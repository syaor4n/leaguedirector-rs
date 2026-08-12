use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_APPS: &[&str] = &[
    "/Applications/League of Legends.app",
    "/Applications/League of Legends (PBE).app",
];

#[derive(Debug, Clone)]
pub struct GameInstall {
    pub cfg: PathBuf,
    pub enabled: bool,
}

pub fn install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for id in [
        "com.riotgames.leagueoflegends",
        "com.riotgames.LeagueofLegends.LeagueClient",
    ] {
        if let Ok(out) = Command::new("mdfind")
            .arg(format!("kMDItemCFBundleIdentifier=={id}"))
            .output()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if !line.is_empty() {
                    roots.push(PathBuf::from(line));
                }
            }
        }
    }
    roots.extend(DEFAULT_APPS.iter().map(PathBuf::from));
    roots
}

pub fn lol_dir_from_cfg(cfg: &Path) -> Option<PathBuf> {
    let mut p = cfg.parent()?.to_path_buf();
    if p.file_name().and_then(|s| s.to_str()) == Some("Config") {
        p.pop();
        return Some(p);
    }
    None
}

pub fn game_pid() -> Option<u32> {
    let out = Command::new("pgrep").args(["-x", "LeagueofLegends"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().parse().ok())
}

pub fn latest_r3dlog() -> Option<PathBuf> {
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for inst in find_installs() {
        let Some(root) = lol_dir_from_cfg(&inst.cfg) else {
            continue;
        };
        let logs = root.join("Logs/GameLogs");
        let Ok(rd) = fs::read_dir(logs) else {
            continue;
        };
        for entry in rd.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let Ok(inner) = fs::read_dir(&dir) else {
                continue;
            };
            for f in inner.flatten() {
                let p = f.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !name.ends_with("r3dlog.txt") {
                    continue;
                }
                let modified = p.metadata().and_then(|m| m.modified()).ok();
                if let Some(t) = modified {
                    if newest.as_ref().map(|(ot, _)| t > *ot).unwrap_or(true) {
                        newest = Some((t, p));
                    }
                }
            }
        }
    }
    newest.map(|(_, p)| p)
}

pub fn latest_r3dlog_tail(lines: usize) -> String {
    let Some(path) = latest_r3dlog() else {
        return "(no r3dlog found)".into();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return format!("(could not read {})", path.display());
    };
    let collected: Vec<&str> = text.lines().rev().take(lines).collect();
    let mut out = format!("{}\n", path.display());
    for line in collected.into_iter().rev() {
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn preferred_replay_dir() -> Option<PathBuf> {
    for inst in find_installs() {
        if let Some(root) = lol_dir_from_cfg(&inst.cfg) {
            let dir = root.join("Replays");
            let _ = fs::create_dir_all(&dir);
            return Some(dir);
        }
    }
    if let Some(home) = directories::UserDirs::new() {
        let docs = home.home_dir().join("Documents/League of Legends/Replays");
        if docs.is_dir() {
            return Some(docs);
        }
    }
    None
}

pub fn preferred_record_dir() -> PathBuf {
    if let Some(replays) = preferred_replay_dir() {
        let dir = replays.join("director-captures");
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    crate::settings::config_dir().join("recordings")
}

pub fn replay_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(p) = preferred_replay_dir() {
        dirs.push(p);
    }
    if let Some(home) = directories::UserDirs::new() {
        dirs.push(home.home_dir().join("Documents/League of Legends/Replays"));
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

pub fn list_rofls() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for dir in replay_search_dirs() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("rofl")) != Some(true)
            {
                continue;
            }
            let key = path.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                files.push(path);
            }
        }
    }
    files.sort_by_key(|p| {
        std::cmp::Reverse(
            p.metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        )
    });
    files
}

pub fn skybox_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![crate::settings::config_dir().join("skyboxes")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos) = exe.parent() {
            if let Some(contents) = macos.parent() {
                dirs.push(contents.join("Resources/skyboxes"));
                dirs.push(contents.join("Resources/assets/skyboxes"));
            }
            dirs.push(macos.join("../assets/skyboxes"));
            dirs.push(macos.join("../../assets/skyboxes"));
            dirs.push(macos.join("../../../assets/skyboxes"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets/skyboxes"));
        dirs.push(cwd.join("../leaguedirector/resources/skyboxes"));
    }
    dirs.into_iter()
        .filter(|d| d.is_dir())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn list_skyboxes() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for dir in skybox_dirs() {
        let Ok(rd) = fs::read_dir(dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !ext.eq_ignore_ascii_case("dds") {
                continue;
            }
            let key = path.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                let label = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("skybox")
                    .to_string();
                out.push((label, path));
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

pub fn find_installs() -> Vec<GameInstall> {
    let roots = install_roots();

    let mut found = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for root in roots {
        if let Some(cfg) = config_path(&root) {
            let key = cfg.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                let enabled = is_enabled(&cfg).unwrap_or(false);
                found.push(GameInstall { cfg, enabled });
            }
        }
    }
    found
}

fn config_path(root: &Path) -> Option<PathBuf> {
    let mut bases = Vec::new();
    let s = root.to_string_lossy();
    if let Some(idx) = s.find("/Contents/LoL") {
        bases.push(PathBuf::from(&s[..=idx + "/Contents/LoL".len() - 1]));
    }
    if root.extension().and_then(|e| e.to_str()) == Some("app") {
        bases.push(root.join("Contents/LoL"));
    }
    bases.push(root.to_path_buf());

    for base in bases {
        for rel in ["Config/game.cfg", "DATA/CFG/game.cfg", "Game/Config/game.cfg"] {
            let cfg = base.join(rel);
            if cfg.is_file() {
                return Some(cfg);
            }
        }
    }
    None
}

pub fn is_enabled(cfg: &Path) -> io::Result<bool> {
    let text = fs::read_to_string(cfg)?;
    for raw in text.lines() {
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.to_ascii_lowercase().starts_with("enablereplayapi") {
            let value = line.split_once('=').map(|(_, v)| v.trim()).unwrap_or("");
            return Ok(matches!(value.to_ascii_lowercase().as_str(), "1" | "true"));
        }
    }
    Ok(false)
}

pub fn set_enabled(cfg: &Path, enabled: bool) -> io::Result<()> {
    let text = fs::read_to_string(cfg)?;
    let value = if enabled { "1" } else { "0" };
    let mut out = Vec::new();
    let mut in_general = false;
    let mut replaced = false;
    let mut general_idx: Option<usize> = None;

    for line in text.lines() {
        let stripped = line.trim();
        if stripped.starts_with('[') && stripped.ends_with(']') {
            if in_general && !replaced {
                out.push(format!("EnableReplayApi={value}"));
                replaced = true;
            }
            in_general = stripped.eq_ignore_ascii_case("[General]");
            if in_general {
                general_idx = Some(out.len());
            }
            out.push(line.to_string());
            continue;
        }
        if in_general && stripped.to_ascii_lowercase().starts_with("enablereplayapi") {
            out.push(format!("EnableReplayApi={value}"));
            replaced = true;
            continue;
        }
        out.push(line.to_string());
    }
    if !replaced {
        if let Some(idx) = general_idx {
            out.insert(idx + 1, format!("EnableReplayApi={value}"));
        } else {
            out.insert(0, "[General]".into());
            out.insert(1, format!("EnableReplayApi={value}"));
        }
    }
    let mut file = fs::File::create(cfg)?;
    for (i, line) in out.iter().enumerate() {
        if i + 1 == out.len() {
            write!(file, "{line}")?;
        } else {
            writeln!(file, "{line}")?;
        }
    }
    Ok(())
}
