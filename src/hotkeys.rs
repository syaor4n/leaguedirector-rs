use crate::api::{upsert_kf, Render, ReplayClient};
use crate::bindings::{self, Action, Chord};
use crate::edits::{Edit, EditStack};
use crate::permissions;
use serde_json::json;
use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

static TAP_LIVE: AtomicBool = AtomicBool::new(false);

pub fn tap_is_live() -> bool {
    TAP_LIVE.load(Ordering::Relaxed)
}

pub type BindingsMap = std::collections::BTreeMap<String, String>;

pub enum HotkeyNews {
    Fired(Action),
    TapFailed,
}

pub struct HotkeyBus {
    rx: mpsc::Receiver<HotkeyNews>,
    bindings: Arc<Mutex<BindingsMap>>,
}

struct TapCtx {
    tx: mpsc::Sender<HotkeyNews>,
    bindings: Arc<Mutex<BindingsMap>>,
    client: ReplayClient,
    edits: Arc<Mutex<EditStack>>,
    tap: std::sync::atomic::AtomicPtr<c_void>,
}

impl HotkeyBus {
    pub fn start(initial: BindingsMap, edits: Arc<Mutex<EditStack>>) -> Self {
        let (tx, rx) = mpsc::channel();
        let bindings = Arc::new(Mutex::new(initial));
        let shared = Arc::clone(&bindings);
        let tap_tx = tx.clone();
        let edits_tap = Arc::clone(&edits);
        thread::spawn(move || {
            if !install_event_tap(tap_tx.clone(), Arc::clone(&shared), edits_tap) {
                let _ = tap_tx.send(HotkeyNews::TapFailed);
                poll_loop(tap_tx, shared, edits);
            }
        });
        Self { rx, bindings }
    }

    pub fn update_bindings(&self, map: BindingsMap) {
        if let Ok(mut g) = self.bindings.lock() {
            *g = map;
        }
    }

    pub fn try_recv(&self) -> Option<HotkeyNews> {
        self.rx.try_recv().ok()
    }
}

/// Run the Replay API call on this thread. The UI loop often sleeps while
/// League is focused, so waiting for egui `update()` made Space look dead.
fn fire(action: Action, client: &ReplayClient, edits: &Arc<Mutex<EditStack>>) {
    match action {
        Action::PlayPause => {
            if let Ok(p) = client.playback() {
                let paused = !p.paused;
                let _ = client.set_playback(&json!({ "paused": paused }));
                notify(
                    "League Director",
                    if paused { "Paused" } else { "Playing" },
                );
            }
        }
        Action::Undo => {
            apply_undo(client, edits);
        }
        Action::Redo => {
            apply_redo(client, edits);
        }
        Action::Cinematic => {
            push_render_checkpoint(client, edits);
            let _ = client.set_render(&crate::presets::cinematic());
            notify("League Director", "Cinematic");
        }
        Action::Gameplay => {
            push_render_checkpoint(client, edits);
            let _ = client.set_render(&crate::presets::gameplay());
            notify("League Director", "Gameplay look");
        }
        Action::Keyframe => {
            if let (Ok(p), Ok(r)) = (client.playback(), client.render()) {
                let mut seq = client.sequence().unwrap_or_default();
                if let Ok(mut g) = edits.lock() {
                    g.push(Edit::Sequence(seq.clone()));
                }
                upsert_kf(&mut seq.camera_position, p.time, r.camera_position, "smoothStep");
                upsert_kf(&mut seq.camera_rotation, p.time, r.camera_rotation, "smoothStep");
                upsert_kf(&mut seq.field_of_view, p.time, r.field_of_view, "linear");
                let _ = client.set_sequence(&seq);
                notify("League Director", "Keyframe");
            }
        }
        Action::SeekBack | Action::TimeMinus5 => {
            if let Ok(p) = client.playback() {
                let _ = client.set_playback(&json!({ "time": (p.time - 5.0).max(0.0) }));
            }
        }
        Action::SeekFwd | Action::TimePlus5 => {
            if let Ok(p) = client.playback() {
                let _ = client.set_playback(&json!({ "time": (p.time + 5.0).min(p.length) }));
            }
        }
        Action::ToggleHud => {
            push_render_checkpoint(client, edits);
            if let Ok(r) = client.render() {
                let _ = client.set_render(&json!({ "interfaceAll": !r.interface_all }));
            }
        }
        Action::ToggleFow => {
            push_render_checkpoint(client, edits);
            if let Ok(r) = client.render() {
                let _ = client.set_render(&json!({ "fogOfWar": !r.fog_of_war }));
            }
        }
        _ => {}
    }
}

fn push_render_checkpoint(client: &ReplayClient, edits: &Arc<Mutex<EditStack>>) {
    if let Ok(r) = client.render() {
        if let Ok(mut g) = edits.lock() {
            g.push(Edit::Render(r));
        }
    }
}

pub fn apply_undo(client: &ReplayClient, edits: &Arc<Mutex<EditStack>>) {
    let item = edits.lock().ok().and_then(|mut g| g.pop_undo());
    let Some(item) = item else {
        notify("League Director", "Nothing to undo");
        return;
    };
    match item {
        Edit::Render(prev) => {
            if let Ok(cur) = client.render() {
                if let Ok(mut g) = edits.lock() {
                    g.push_redo(Edit::Render(cur));
                }
            }
            let _ = post_render(client, &prev);
            notify("League Director", "Undo look");
        }
        Edit::Sequence(prev) => {
            if let Ok(cur) = client.sequence() {
                if let Ok(mut g) = edits.lock() {
                    g.push_redo(Edit::Sequence(cur));
                }
            }
            let _ = client.set_sequence(&prev);
            notify("League Director", "Undo sequence");
        }
    }
}

pub fn apply_redo(client: &ReplayClient, edits: &Arc<Mutex<EditStack>>) {
    let item = edits.lock().ok().and_then(|mut g| g.pop_redo());
    let Some(item) = item else {
        return;
    };
    match item {
        Edit::Render(next) => {
            if let Ok(cur) = client.render() {
                if let Ok(mut g) = edits.lock() {
                    g.push(Edit::Render(cur));
                }
            }
            let _ = post_render(client, &next);
            notify("League Director", "Redo look");
        }
        Edit::Sequence(next) => {
            let _ = client.set_sequence(&next);
        }
    }
}

fn post_render(client: &ReplayClient, render: &Render) -> crate::api::error::Result<()> {
    let body = serde_json::to_value(render).unwrap_or(json!({}));
    client.set_render(&body)
}

fn maybe_fire(
    action: Action,
    client: &ReplayClient,
    tx: &mpsc::Sender<HotkeyNews>,
    edits: &Arc<Mutex<EditStack>>,
) {
    if !should_arm(&frontmost_display_name()) {
        return;
    }
    fire(action, client, edits);
    let _ = tx.send(HotkeyNews::Fired(action));
}

fn poll_loop(
    tx: mpsc::Sender<HotkeyNews>,
    bindings: Arc<Mutex<BindingsMap>>,
    edits: Arc<Mutex<EditStack>>,
) {
    let client = match ReplayClient::new() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut prev: HashMap<Action, bool> = HashMap::new();
    loop {
        let map = bindings.lock().ok().map(|g| g.clone()).unwrap_or_default();
        let cmd = key_down(55) || key_down(54);
        let shift = key_down(56) || key_down(60);
        let alt = key_down(58) || key_down(61);
        let ctrl = key_down(59) || key_down(62);
        for action in Action::ALL {
            let chord = bindings::chord_of(&map, *action);
            let down = chord_held(chord, cmd, shift, alt, ctrl);
            let was = prev.get(action).copied().unwrap_or(false);
            if down && !was {
                maybe_fire(*action, &client, &tx, &edits);
            }
            prev.insert(*action, down);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn chord_held(c: Chord, cmd: bool, shift: bool, alt: bool, ctrl: bool) -> bool {
    key_down(c.key as i32) && c.cmd == cmd && c.shift == shift && c.alt == alt && c.ctrl == ctrl
}

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

pub fn is_director_main(name: &str) -> bool {
    name.to_ascii_lowercase().contains("league director")
}

pub fn is_director_hud(name: &str) -> bool {
    let s = name.to_ascii_lowercase();
    (s.contains("director") && !s.contains("league director")) || s.contains("director hud")
}

fn is_blocked_desktop(name: &str) -> bool {
    let s = name.to_ascii_lowercase();
    [
        "google chrome",
        "safari",
        "firefox",
        "discord",
        "iterm",
        "terminal",
        "code",
        "cursor",
        "slack",
        "finder",
        "mail",
        "spotify",
        "notes",
        "promethee",
        "league director",
    ]
    .iter()
    .any(|n| s.contains(n))
}

pub fn should_arm(front: &str) -> bool {
    if is_director_main(front) {
        return false;
    }
    if is_league_name(front) || is_director_hud(front) {
        return true;
    }
    crate::detect::game_pid().is_some() && !is_blocked_desktop(front)
}

#[allow(dead_code)]
pub fn debug_snapshot() -> String {
    let front = frontmost_display_name();
    format!(
        "front={} · armed={} · tap/hid space={} k={}",
        front.replace('\n', " ").trim(),
        if should_arm(&front) { "yes" } else { "no" },
        key_down(49) as u8,
        key_down(40) as u8
    )
}

const KCG_SESSION_EVENT_TAP: u32 = 1;
const KCG_HEAD_INSERT: u32 = 0;
const KCG_TAP_DEFAULT: u32 = 0;
const KCG_EVENT_KEY_DOWN: u32 = 10;
const KCG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
const KCG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
const KCG_SHIFT: u64 = 0x0002_0000;
const KCG_CTRL: u64 = 0x0004_0000;
const KCG_ALT: u64 = 0x0008_0000;
const KCG_CMD: u64 = 0x0010_0000;

#[cfg(target_os = "macos")]
fn install_event_tap(
    tx: mpsc::Sender<HotkeyNews>,
    bindings: Arc<Mutex<BindingsMap>>,
    edits: Arc<Mutex<EditStack>>,
) -> bool {
    permissions::request_hotkey_permissions();
    let Ok(client) = ReplayClient::new() else {
        return false;
    };
    let ctx = Box::new(TapCtx {
        tx,
        bindings,
        client,
        edits,
        tap: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
    });
    let info = Box::into_raw(ctx) as *mut c_void;
    let mask: u64 = 1u64 << KCG_EVENT_KEY_DOWN;
    unsafe {
        let tap = CGEventTapCreate(
            KCG_SESSION_EVENT_TAP,
            KCG_HEAD_INSERT,
            KCG_TAP_DEFAULT,
            mask,
            tap_callback,
            info,
        );
        if tap.is_null() {
            let _ = Box::from_raw(info as *mut TapCtx);
            return false;
        }
        (*info.cast::<TapCtx>()).tap.store(tap, std::sync::atomic::Ordering::SeqCst);
        let src = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        if src.is_null() {
            CFRelease(tap);
            let _ = Box::from_raw(info as *mut TapCtx);
            return false;
        }
        CFRunLoopAddSource(CFRunLoopGetCurrent(), src, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        TAP_LIVE.store(true, Ordering::Relaxed);
        CFRunLoopRun();
        TAP_LIVE.store(false, Ordering::Relaxed);
    }
    true
}

#[cfg(not(target_os = "macos"))]
fn install_event_tap(
    _tx: mpsc::Sender<HotkeyNews>,
    _bindings: Arc<Mutex<BindingsMap>>,
    _edits: Arc<Mutex<EditStack>>,
) -> bool {
    false
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn tap_callback(
    _proxy: *mut c_void,
    ty: u32,
    event: *mut c_void,
    info: *mut c_void,
) -> *mut c_void {
    const TAP_DISABLED_TIMEOUT: u32 = 0xFFFF_FFFE;
    const TAP_DISABLED_USER: u32 = 0xFFFF_FFFD;
    if info.is_null() {
        return event;
    }
    let ctx = unsafe { &*(info as *const TapCtx) };
    if ty == TAP_DISABLED_TIMEOUT || ty == TAP_DISABLED_USER {
        let tap = ctx.tap.load(std::sync::atomic::Ordering::SeqCst);
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, true) };
        }
        return event;
    }
    if ty != KCG_EVENT_KEY_DOWN || event.is_null() {
        return event;
    }
    let repeat = unsafe { CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_AUTOREPEAT) };
    if repeat != 0 {
        return event;
    }
    let code = unsafe { CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_KEYCODE) } as u16;
    let flags = unsafe { CGEventGetFlags(event) };
    let map = ctx.bindings.lock().ok().map(|g| g.clone()).unwrap_or_default();
    let cmd = flags & KCG_CMD != 0;
    let shift = flags & KCG_SHIFT != 0;
    let alt = flags & KCG_ALT != 0;
    let ctrl = flags & KCG_CTRL != 0;
    for action in Action::ALL {
        let chord = bindings::chord_of(&map, *action);
        if chord.key == code && chord.cmd == cmd && chord.shift == shift && chord.alt == alt && chord.ctrl == ctrl
        {
            let client = ctx.client.clone();
            let tx = ctx.tx.clone();
            let edits = Arc::clone(&ctx.edits);
            thread::spawn(move || maybe_fire(*action, &client, &tx, &edits));
            break;
        }
    }
    event
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    fn CGEventGetFlags(event: *mut c_void) -> u64;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: i64,
    ) -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    fn CFRelease(cf: *mut c_void);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_game_not_client() {
        assert!(is_league_name(r#""LSDisplayName"="League of Legends""#));
        assert!(is_league_name(r#""LSDisplayName"="League Of Legends""#));
        assert!(!is_league_name(r#""LSDisplayName"="LeagueClient""#));
        assert!(!is_league_name(r#""LSDisplayName"="Discord""#));
        assert!(should_arm(r#""LSDisplayName"="League Of Legends""#));
        assert!(should_arm(r#""LSDisplayName"="Director""#));
        assert!(!should_arm(r#""LSDisplayName"="League Director""#));
        assert!(!should_arm(r#""LSDisplayName"="Google Chrome""#));
    }
}

pub fn notify(title: &str, body: &str) {
    let title = title.replace('"', "'");
    let body = body.replace('"', "'");
    let script = format!(r#"display notification "{body}" with title "{title}" sound name "Tink""#);
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn();
}
