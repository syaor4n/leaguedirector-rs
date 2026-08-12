use eframe::egui;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    PlayPause,
    SeekBack,
    SeekFwd,
    Keyframe,
    PlaySeq,
    Undo,
    Redo,
    ToggleHud,
    ToggleFow,
    ToggleAttach,
    TimeMinus5,
    TimePlus5,
    RecordToggle,
    Cinematic,
    Gameplay,
    NextKf,
    PrevKf,
    DeleteKf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub key: u16,
    pub cmd: bool,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl Action {
    pub const ALL: &[Action] = &[
        Action::PlayPause,
        Action::SeekBack,
        Action::SeekFwd,
        Action::Keyframe,
        Action::PlaySeq,
        Action::Undo,
        Action::Redo,
        Action::ToggleHud,
        Action::ToggleFow,
        Action::ToggleAttach,
        Action::TimeMinus5,
        Action::TimePlus5,
        Action::RecordToggle,
        Action::Cinematic,
        Action::Gameplay,
        Action::NextKf,
        Action::PrevKf,
        Action::DeleteKf,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Action::PlayPause => "play_pause",
            Action::SeekBack => "seek_back",
            Action::SeekFwd => "seek_fwd",
            Action::Keyframe => "keyframe",
            Action::PlaySeq => "play_seq",
            Action::Undo => "undo",
            Action::Redo => "redo",
            Action::ToggleHud => "toggle_hud",
            Action::ToggleFow => "toggle_fow",
            Action::ToggleAttach => "toggle_attach",
            Action::TimeMinus5 => "time_minus_5",
            Action::TimePlus5 => "time_plus_5",
            Action::RecordToggle => "record_toggle",
            Action::Cinematic => "cinematic",
            Action::Gameplay => "gameplay",
            Action::NextKf => "next_kf",
            Action::PrevKf => "prev_kf",
            Action::DeleteKf => "delete_kf",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::PlayPause => "Play / Pause",
            Action::SeekBack => "Seek −5s",
            Action::SeekFwd => "Seek +5s",
            Action::Keyframe => "Camera keyframe",
            Action::PlaySeq => "Play sequence",
            Action::Undo => "Undo",
            Action::Redo => "Redo",
            Action::ToggleHud => "Toggle game HUD",
            Action::ToggleFow => "Toggle fog of war",
            Action::ToggleAttach => "Toggle camera attach",
            Action::TimeMinus5 => "Time −5s",
            Action::TimePlus5 => "Time +5s",
            Action::RecordToggle => "Start / stop record",
            Action::Cinematic => "Cinematic preset",
            Action::Gameplay => "Gameplay preset",
            Action::NextKf => "Next keyframe",
            Action::PrevKf => "Previous keyframe",
            Action::DeleteKf => "Delete selected keyframe",
        }
    }

    pub fn default_chord(self) -> &'static str {
        match self {
            Action::PlayPause => "Space",
            Action::SeekBack => "Left",
            Action::SeekFwd => "Right",
            Action::Keyframe => "K",
            Action::PlaySeq => "Enter",
            Action::Undo => "Cmd+Z",
            Action::Redo => "Cmd+Shift+Z",
            Action::ToggleHud => "H",
            Action::ToggleFow => "F",
            Action::ToggleAttach => "A",
            Action::TimeMinus5 => "Shift+Left",
            Action::TimePlus5 => "Shift+Right",
            Action::RecordToggle => "R",
            Action::Cinematic => "Shift+C",
            Action::Gameplay => "Shift+G",
            Action::NextKf => "]",
            Action::PrevKf => "[",
            Action::DeleteKf => "Delete",
        }
    }

    #[allow(dead_code)]
    pub fn from_id(id: &str) -> Option<Self> {
        Action::ALL.iter().copied().find(|a| a.id() == id)
    }
}

pub fn default_map() -> BTreeMap<String, String> {
    Action::ALL
        .iter()
        .map(|a| (a.id().to_string(), a.default_chord().to_string()))
        .collect()
}

pub fn merge_defaults(map: &mut BTreeMap<String, String>) {
    for a in Action::ALL {
        map.entry(a.id().to_string())
            .or_insert_with(|| a.default_chord().to_string());
    }
}

pub fn chord_of(map: &BTreeMap<String, String>, action: Action) -> Chord {
    parse_chord(map.get(action.id()).map(|s| s.as_str()).unwrap_or(action.default_chord()))
        .unwrap_or_else(|| parse_chord(action.default_chord()).expect("default chord"))
}

pub fn parse_chord(raw: &str) -> Option<Chord> {
    let mut cmd = false;
    let mut shift = false;
    let mut alt = false;
    let mut ctrl = false;
    let mut key = None;
    for part in raw.split('+') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        match p.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "super" | "meta" => cmd = true,
            "shift" => shift = true,
            "alt" | "option" => alt = true,
            "ctrl" | "control" => ctrl = true,
            other => key = Some(key_code(other)?),
        }
    }
    Some(Chord {
        key: key?,
        cmd,
        shift,
        alt,
        ctrl,
    })
}

pub fn key_code(name: &str) -> Option<u16> {
    let n = name.to_ascii_lowercase();
    Some(match n.as_str() {
        "space" => 49,
        "enter" | "return" => 36,
        "left" | "arrowleft" => 123,
        "right" | "arrowright" => 124,
        "down" | "arrowdown" => 125,
        "up" | "arrowup" => 126,
        "delete" | "backspace" => 51,
        "fwddelete" | "forwarddelete" => 117,
        "escape" | "esc" => 53,
        "tab" => 48,
        "a" => 0,
        "s" => 1,
        "d" => 2,
        "f" => 3,
        "h" => 4,
        "g" => 5,
        "z" => 6,
        "x" => 7,
        "c" => 8,
        "v" => 9,
        "b" => 11,
        "q" => 12,
        "w" => 13,
        "e" => 14,
        "r" => 15,
        "y" => 16,
        "t" => 17,
        "1" => 18,
        "2" => 19,
        "3" => 20,
        "4" => 21,
        "6" => 22,
        "5" => 23,
        "=" | "equal" => 24,
        "9" => 25,
        "7" => 26,
        "-" | "minus" => 27,
        "8" => 28,
        "0" => 29,
        "]" | "rightbracket" => 30,
        "o" => 31,
        "u" => 32,
        "[" | "leftbracket" => 33,
        "i" => 34,
        "p" => 35,
        "l" => 37,
        "j" => 38,
        "'" | "quote" => 39,
        "k" => 40,
        ";" | "semicolon" => 41,
        "\\" | "backslash" => 42,
        "," | "comma" => 43,
        "/" | "slash" => 44,
        "n" => 45,
        "m" => 46,
        "." | "period" => 47,
        "`" | "backquote" => 50,
        _ => return None,
    })
}

pub fn egui_pressed(ctx: &egui::Context, chord: Chord) -> bool {
    let key = egui_key(chord.key);
    let Some(key) = key else {
        return false;
    };
    ctx.input(|i| {
        let mods = i.modifiers;
        let cmd_ok = if chord.cmd {
            mods.command
        } else {
            !mods.command
        };
        let shift_ok = if chord.shift { mods.shift } else { !mods.shift };
        let alt_ok = if chord.alt { mods.alt } else { !mods.alt };
        let ctrl_ok = if chord.ctrl {
            mods.ctrl && !mods.command
        } else {
            !mods.ctrl || mods.command
        };
        cmd_ok && shift_ok && alt_ok && ctrl_ok && i.key_pressed(key)
    })
}

fn egui_key(code: u16) -> Option<egui::Key> {
    use egui::Key;
    Some(match code {
        49 => Key::Space,
        36 => Key::Enter,
        123 => Key::ArrowLeft,
        124 => Key::ArrowRight,
        125 => Key::ArrowDown,
        126 => Key::ArrowUp,
        51 => Key::Backspace,
        117 => Key::Delete,
        53 => Key::Escape,
        48 => Key::Tab,
        0 => Key::A,
        1 => Key::S,
        2 => Key::D,
        3 => Key::F,
        4 => Key::H,
        5 => Key::G,
        6 => Key::Z,
        7 => Key::X,
        8 => Key::C,
        9 => Key::V,
        11 => Key::B,
        12 => Key::Q,
        13 => Key::W,
        14 => Key::E,
        15 => Key::R,
        16 => Key::Y,
        17 => Key::T,
        18 => Key::Num1,
        19 => Key::Num2,
        20 => Key::Num3,
        21 => Key::Num4,
        22 => Key::Num6,
        23 => Key::Num5,
        25 => Key::Num9,
        26 => Key::Num7,
        28 => Key::Num8,
        29 => Key::Num0,
        31 => Key::O,
        32 => Key::U,
        34 => Key::I,
        35 => Key::P,
        37 => Key::L,
        38 => Key::J,
        40 => Key::K,
        45 => Key::N,
        46 => Key::M,
        30 => Key::CloseBracket,
        33 => Key::OpenBracket,
        42 => Key::Backslash,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cmd_shift_z() {
        let c = parse_chord("Cmd+Shift+Z").unwrap();
        assert!(c.cmd && c.shift && !c.alt && c.key == 6);
    }

    #[test]
    fn parses_brackets() {
        assert_eq!(parse_chord("]").unwrap().key, 30);
        assert_eq!(parse_chord("[").unwrap().key, 33);
    }
}
