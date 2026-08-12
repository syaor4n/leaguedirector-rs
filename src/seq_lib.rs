use crate::api::Sequence;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list(dir: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return files;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("json") {
            files.push(p);
        }
    }
    files.sort();
    files
}

pub fn save(dir: &str, name: &str, sequence: &Sequence) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stem = sanitize(name);
    if stem.is_empty() {
        return Err("name required".into());
    }
    let path = Path::new(dir).join(format!("{stem}.json"));
    let json = serde_json::to_string_pretty(sequence).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn load(path: &Path) -> Result<Sequence, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}
