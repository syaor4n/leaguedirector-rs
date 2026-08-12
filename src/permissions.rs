use std::path::{Path, PathBuf};
use std::process::Command;

const ACCESSIBILITY: &[&str] = &[
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility",
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
];

const INPUT: &[&str] = &[
    "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_ListenEvent",
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
];

pub fn open_privacy_settings() {
    if let Some(url) = ACCESSIBILITY.first() {
        let _ = Command::new("open").arg(url).status();
    }
    if let Some(url) = INPUT.first() {
        let _ = Command::new("open").arg(url).status();
    }
}

pub fn open_files_privacy() {
    let urls = [
        "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_FilesAndFolders",
        "x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders",
    ];
    for url in urls {
        if Command::new("open").arg(url).status().map(|s| s.success()).unwrap_or(false) {
            break;
        }
    }
}

pub fn reveal_in_finder(path: &Path) {
    let _ = Command::new("open")
        .args(["-R", &path.display().to_string()])
        .status();
}

pub fn open_folder(path: &Path) {
    let _ = Command::new("open").arg(path).status();
}

#[cfg(target_os = "macos")]
pub fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn input_monitoring_ok() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}

#[cfg(target_os = "macos")]
pub fn request_hotkey_permissions() {
    unsafe {
        let opts = std::ptr::null();
        AXIsProcessTrustedWithOptions(opts);
        let _ = CGRequestListenEventAccess();
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_hotkey_permissions() {}

#[cfg(not(target_os = "macos"))]
pub fn input_monitoring_ok() -> bool {
    true
}

pub fn documents_ok() -> bool {
    let dir = crate::settings::config_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let probe = dir.join(".write_probe");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(probe);
    ok
}

pub fn running_from_bundle() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .map(|s| s.contains(".app/Contents/MacOS/"))
        .unwrap_or(false)
}

pub fn current_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .map(|p| p.to_path_buf())
}

pub fn bundled_app_in_target() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/bundle/osx/League Director.app");
    p.is_dir().then_some(p)
}

pub fn install_to_applications() -> Result<PathBuf, String> {
    let src = current_app_bundle()
        .or_else(bundled_app_in_target)
        .ok_or_else(|| {
            "No .app to install. Run ./scripts/macos-bundle.sh first, then open that app.".to_string()
        })?;
    let dest = PathBuf::from("/Applications/League Director.app");
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    let status = Command::new("ditto")
        .arg(&src)
        .arg(&dest)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("ditto failed".into());
    }
    let _ = Command::new("codesign")
        .args(["--force", "--deep", "--sign", "-", dest.to_str().unwrap_or("")])
        .status();
    Ok(dest)
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}
