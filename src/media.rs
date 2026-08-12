use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

/// If League left a `.webm.tmp`, remux it to a playable `.webm` next to it
/// (and copy into `dest_dir` if different).
pub fn finalize_recording(path_hint: &str, dest_dir: &str) -> Option<PathBuf> {
    let hint = Path::new(path_hint);
    let mut parents = vec![hint.parent().unwrap_or(Path::new(".")).to_path_buf()];
    if !dest_dir.is_empty() {
        parents.push(PathBuf::from(dest_dir));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if hint.is_file() {
        candidates.push(hint.to_path_buf());
    }
    for parent in parents {
        if let Ok(rd) = fs::read_dir(&parent) {
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name.ends_with(".webm") || name.ends_with(".webm.tmp") || name.ends_with(".png")
                {
                    candidates.push(p);
                }
            }
        }
    }
    candidates.sort_by_key(|p| {
        p.metadata()
            .and_then(|m| m.modified())
            .ok()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
            .unwrap_or(0)
    });
    let src = candidates.pop()?;
    let dest = if src.extension().and_then(|e| e.to_str()) == Some("tmp") {
        src.with_extension("").with_extension("webm")
    } else {
        src.clone()
    };
    if src != dest {
        if let Some(ff) = ffmpeg_bin() {
            let _ = Command::new(ff)
                .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
                .arg(&src)
                .args(["-c", "copy"])
                .arg(&dest)
                .status();
        } else {
            let _ = fs::copy(&src, &dest);
        }
    }
    if !dest_dir.is_empty() {
        if let Some(name) = dest.file_name() {
            let copy = Path::new(dest_dir).join(name);
            if copy != dest {
                let _ = fs::create_dir_all(dest_dir);
                let _ = fs::copy(&dest, &copy);
                return Some(copy);
            }
        }
    }
    dest.is_file().then_some(dest)
}

pub fn spawn_watchdog(
    hint: String,
    dest_dir: String,
    expected_secs: f64,
    on_done: impl FnOnce(Option<PathBuf>) + Send + 'static,
) {
    thread::spawn(move || {
        let wait = (expected_secs + 5.0).clamp(3.0, 180.0);
        thread::sleep(Duration::from_secs_f64(wait));
        on_done(finalize_recording(&hint, &dest_dir));
    });
}

pub fn ffmpeg_bin() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("ffmpeg");
            if sibling.is_file() {
                return Some(sibling);
            }
            if let Some(contents) = dir.parent() {
                let res = contents.join("Resources/ffmpeg");
                if res.is_file() {
                    return Some(res);
                }
            }
        }
    }
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .ok()
        .and_then(|o| o.status.success().then(|| PathBuf::from("ffmpeg")))
}

pub fn ffmpeg_available() -> bool {
    ffmpeg_bin().is_some()
}

pub fn list_clips(dirs: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for dir in dirs {
        let Ok(rd) = fs::read_dir(dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".webm") || name.ends_with(".png") {
                let key = p.to_string_lossy().to_lowercase();
                if seen.insert(key) {
                    files.push(p);
                }
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
