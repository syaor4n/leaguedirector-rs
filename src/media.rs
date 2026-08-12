use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// If League left a `.webm.tmp`, remux it to a playable `.webm` next to it
/// (and copy into `dest_dir` if different).
pub fn finalize_recording(path_hint: &str, dest_dir: &str) -> Option<PathBuf> {
    let hint = Path::new(path_hint);
    let parent = hint.parent().unwrap_or(Path::new("."));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if hint.is_file() {
        candidates.push(hint.to_path_buf());
    }
    if let Ok(rd) = fs::read_dir(parent) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".webm") || name.ends_with(".webm.tmp") || name.ends_with(".png") {
                candidates.push(p);
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
        if which_ffmpeg() {
            let _ = Command::new("ffmpeg")
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
    Some(dest)
}

pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
}

fn which_ffmpeg() -> bool {
    ffmpeg_available()
}
