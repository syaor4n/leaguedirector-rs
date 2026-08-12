use crate::bindings::{self, Action, Chord};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct HotkeyBus {
    rx: mpsc::Receiver<Action>,
    bindings: Arc<Mutex<BindingsMap>>,
}

pub type BindingsMap = std::collections::BTreeMap<String, String>;

impl HotkeyBus {
    pub fn start(initial: BindingsMap) -> Self {
        let (tx, rx) = mpsc::channel();
        let bindings = Arc::new(Mutex::new(initial));
        let shared = Arc::clone(&bindings);
        thread::spawn(move || poll_loop(tx, shared));
        Self { rx, bindings }
    }

    pub fn update_bindings(&self, map: BindingsMap) {
        if let Ok(mut g) = self.bindings.lock() {
            *g = map;
        }
    }

    pub fn try_recv(&self) -> Option<Action> {
        self.rx.try_recv().ok()
    }
}

fn poll_loop(tx: mpsc::Sender<Action>, bindings: Arc<Mutex<BindingsMap>>) {
    let mut prev: HashMap<Action, bool> = HashMap::new();
    loop {
        let front = frontmost_display_name();
        let armed = should_arm(&front);
        let map = bindings.lock().ok().map(|g| g.clone()).unwrap_or_default();
        let cmd = key_down(55) || key_down(54);
        let shift = key_down(56) || key_down(60);
        let alt = key_down(58) || key_down(61);
        let ctrl = key_down(59) || key_down(62);

        for action in Action::ALL {
            let chord = bindings::chord_of(&map, *action);
            let down = chord_held(chord, cmd, shift, alt, ctrl);
            let was = prev.get(action).copied().unwrap_or(false);
            if armed && down && !was {
                let _ = tx.send(*action);
            }
            prev.insert(*action, down);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn chord_held(c: Chord, cmd: bool, shift: bool, alt: bool, ctrl: bool) -> bool {
    key_down(c.key as i32) && c.cmd == cmd && c.shift == shift && c.alt == alt && c.ctrl == ctrl
}

/// Hardware HID state (1). Combined session state (0) often stays false for
/// keys that went to another app — which is why Space/K did nothing in League.
const HID_SYSTEM_STATE: i32 = 1;

#[cfg(target_os = "macos")]
fn key_down(code: i32) -> bool {
    unsafe { CGEventSourceKeyState(HID_SYSTEM_STATE, code as u16) }
}

#[cfg(not(target_os = "macos"))]
fn key_down(_code: i32) -> bool {
    false
}

pub fn frontmost_display_name() -> String {
    #[cfg(target_os = "macos")]
    {
        let asn = std::process::Command::new("lsappinfo")
            .arg("front")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        if asn.is_empty() {
            return String::new();
        }
        std::process::Command::new("lsappinfo")
            .args(["info", "-only", "name", &asn])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "macos"))]
    {
        String::new()
    }
}

pub fn is_league_name(name: &str) -> bool {
    let s = name.to_ascii_lowercase();
    if s.contains("riot client") || s.contains("leagueclient") {
        return false;
    }
    s.contains("league of legends") || s.contains("leagueoflegends")
}

#[allow(dead_code)]
pub fn is_director_main(name: &str) -> bool {
    name.to_ascii_lowercase().contains("league director")
}

/// HUD window title is just "Director" (not "League Director").
pub fn is_director_hud(name: &str) -> bool {
    let s = name.to_ascii_lowercase();
    (s.contains("director") && !s.contains("league director")) || s.contains("director hud")
}

/// Global poll fires in League. It also fires for the HUD, because the HUD
/// viewport often does not receive egui key events. It does *not* fire for
/// the main Director window (egui handles those — both would toggle twice).
pub fn should_arm(front: &str) -> bool {
    is_league_name(front) || is_director_hud(front)
}

pub fn debug_snapshot() -> String {
    let front = frontmost_display_name();
    let armed = should_arm(&front);
    format!(
        "front={} · armed={} · hid space={} k={}",
        front.replace('\n', " ").trim(),
        if armed { "yes" } else { "no" },
        key_down(49) as u8,
        key_down(40) as u8
    )
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_game_not_client() {
        assert!(is_league_name(r#""LSDisplayName"="League of Legends""#));
        assert!(is_league_name(r#""LSDisplayName"="League Of Legends""#));
        assert!(!is_league_name(r#""LSDisplayName"="LeagueClient""#));
        assert!(!is_league_name(r#""LSDisplayName"="Riot Client""#));
        assert!(!is_league_name(r#""LSDisplayName"="Discord""#));
        assert!(should_arm(r#""LSDisplayName"="League Of Legends""#));
        assert!(should_arm(r#""LSDisplayName"="Director""#));
        assert!(!should_arm(r#""LSDisplayName"="League Director""#));
        assert!(!should_arm(r#""LSDisplayName"="Google Chrome""#));
    }
}
