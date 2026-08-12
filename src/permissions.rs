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

pub fn reveal_in_finder(path: &std::path::Path) {
    let _ = Command::new("open").args(["-R", &path.display().to_string()]).status();
}

pub fn open_folder(path: &std::path::Path) {
    let _ = Command::new("open").arg(path).status();
}
