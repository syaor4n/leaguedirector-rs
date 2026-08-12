use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    PlayPause,
    SeekBack,
    SeekFwd,
    Keyframe,
    PlaySeq,
    Undo,
    Redo,
}

pub struct HotkeyBus {
    rx: Receiver<Hotkey>,
}

impl HotkeyBus {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || poll_loop(tx));
        Self { rx }
    }

    pub fn try_recv(&self) -> Option<Hotkey> {
        match self.rx.try_recv() {
            Ok(k) => Some(k),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

fn poll_loop(tx: Sender<Hotkey>) {
    let mut prev_space = false;
    let mut prev_k = false;
    let mut prev_left = false;
    let mut prev_right = false;
    let mut prev_enter = false;
    let mut prev_z = false;
    let mut prev_y = false;
    loop {
        if !league_is_frontmost() {
            prev_space = key_down(49);
            prev_k = key_down(40);
            prev_left = key_down(123);
            prev_right = key_down(124);
            prev_enter = key_down(36);
            prev_z = key_down(6);
            prev_y = key_down(16);
            thread::sleep(Duration::from_millis(40));
            continue;
        }
        let space = key_down(49);
        let k = key_down(40);
        let left = key_down(123);
        let right = key_down(124);
        let enter = key_down(36);
        let z = key_down(6);
        let y = key_down(16);
        let cmd = key_down(55) || key_down(54);
        let shift = key_down(56) || key_down(60);

        if space && !prev_space {
            let _ = tx.send(Hotkey::PlayPause);
        }
        if k && !prev_k && !cmd {
            let _ = tx.send(Hotkey::Keyframe);
        }
        if left && !prev_left {
            let _ = tx.send(Hotkey::SeekBack);
        }
        if right && !prev_right {
            let _ = tx.send(Hotkey::SeekFwd);
        }
        if enter && !prev_enter {
            let _ = tx.send(Hotkey::PlaySeq);
        }
        if z && !prev_z && cmd && !shift {
            let _ = tx.send(Hotkey::Undo);
        }
        if ((z && cmd && shift) || (y && cmd)) && !(prev_z && cmd && shift) && !(prev_y && cmd) {
            let _ = tx.send(Hotkey::Redo);
        }

        prev_space = space;
        prev_k = k;
        prev_left = left;
        prev_right = right;
        prev_enter = enter;
        prev_z = z;
        prev_y = y;
        thread::sleep(Duration::from_millis(30));
    }
}

#[cfg(target_os = "macos")]
fn key_down(code: i32) -> bool {
    unsafe { cg_event_source_key_state(0, code as u16) }
}

#[cfg(not(target_os = "macos"))]
fn key_down(_code: i32) -> bool {
    false
}

#[cfg(target_os = "macos")]
fn league_is_frontmost() -> bool {
    let out = std::process::Command::new("lsappinfo")
        .args(["info", "-only", "name", "front"])
        .output();
    let Ok(out) = out else {
        return false;
    };
    let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
    s.contains("league of legends") || s.contains("leagueoflegends")
}

#[cfg(not(target_os = "macos"))]
fn league_is_frontmost() -> bool {
    false
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state_id: u32, key: u16) -> bool;
}

#[cfg(target_os = "macos")]
unsafe fn cg_event_source_key_state(state_id: u32, key: u16) -> bool {
    unsafe { CGEventSourceKeyState(state_id, key) }
}
