use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Long webm encodes hang the Replay API. Keep clips short.
pub const MAX_WEBM_SECS: f64 = 16.0;

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
    on_done: impl FnOnce(Option<PathBuf>, Option<f64>) + Send + 'static,
) {
    thread::spawn(move || {
        let min_wait = expected_secs.max(1.0);
        thread::sleep(Duration::from_secs_f64(min_wait));
        // Wait until the newest capture file stops growing (encode often outlives the API).
        let deadline = Instant::now() + Duration::from_secs_f64((expected_secs + 30.0).clamp(8.0, 90.0));
        let mut last_size = 0u64;
        let mut stable = 0u8;
        while Instant::now() < deadline {
            let size = newest_capture_size(&hint, &dest_dir).unwrap_or(0);
            if size > 0 && size == last_size {
                stable += 1;
                if stable >= 4 {
                    break;
                }
            } else {
                stable = 0;
                last_size = size;
            }
            thread::sleep(Duration::from_millis(500));
        }
        let path = finalize_recording(&hint, &dest_dir);
        let secs = path.as_ref().and_then(|p| probe_duration(p));
        on_done(path, secs);
    });
}

fn newest_capture_size(hint: &str, dest_dir: &str) -> Option<u64> {
    let mut dirs = vec![Path::new(hint).parent().unwrap_or(Path::new(".")).to_path_buf()];
    if !dest_dir.is_empty() {
        dirs.push(PathBuf::from(dest_dir));
    }
    let mut best: Option<(std::time::SystemTime, u64)> = None;
    for dir in dirs {
        let Ok(rd) = fs::read_dir(dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !(name.ends_with(".webm") || name.ends_with(".webm.tmp") || name.ends_with(".png")) {
                continue;
            }
            let Ok(meta) = e.metadata() else {
                continue;
            };
            let Ok(t) = meta.modified() else {
                continue;
            };
            let sz = meta.len();
            if best.as_ref().map(|(ot, _)| t > *ot).unwrap_or(true) {
                best = Some((t, sz));
            }
        }
    }
    best.map(|(_, s)| s)
}

pub fn probe_duration(path: &Path) -> Option<f64> {
    let ff = ffmpeg_bin()?;
    // ffprobe may not be bundled; use ffmpeg -i and parse Duration.
    let out = Command::new(&ff)
        .args(["-hide_banner", "-i"])
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    for part in text.split("Duration:") {
        let token = part.split(',').next()?.trim();
        let mut hms = token.split(':');
        let h: f64 = hms.next()?.parse().ok()?;
        let m: f64 = hms.next()?.parse().ok()?;
        let s: f64 = hms.next()?.parse().ok()?;
        return Some(h * 3600.0 + m * 60.0 + s);
    }
    None
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

pub fn leftover_tmp(dest_dir: &str) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let Ok(rd) = fs::read_dir(dest_dir) else {
        return None;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.ends_with(".webm.tmp") {
            continue;
        }
        let Ok(t) = e.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if best.as_ref().map(|(ot, _)| t > *ot).unwrap_or(true) {
            best = Some((t, p));
        }
    }
    best.map(|(_, p)| p)
}

pub fn clamp_webm_span(start: f64, end: f64) -> (f64, f64, bool) {
    let span = (end - start).abs();
    if span <= MAX_WEBM_SECS {
        let (a, b) = if end >= start { (start, end) } else { (end, start) };
        return (a, b.max(a + 0.5), false);
    }
    let a = start.min(end);
    (a, a + MAX_WEBM_SECS, true)
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
