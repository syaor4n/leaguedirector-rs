use crate::api::{
    upsert_kf, Color, Playback, Particles, Recording, Render, ReplayClient, Sequence, Vec3,
};
use crate::bindings::{self, Action};
use crate::detect::{self, GameInstall};
use crate::handshake::{self, WatchOutcome};
use crate::hotkeys::HotkeyBus;
use crate::lcu;
use crate::media;
use crate::permissions;
use crate::presets;
use crate::seq_lib;
use crate::sequence_ui::{self, TrackEvent, TrackId};
use crate::settings::Settings;
use eframe::egui::{self, Color32, RichText, Slider, Ui};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

enum BgEvent {
    Status(String),
    WatchDone(WatchOutcome),
    ClipReady { path: PathBuf, secs: Option<f64> },
}

pub struct DirectorApp {
    client: ReplayClient,
    settings: Settings,
    hotkeys: HotkeyBus,
    bg_tx: Sender<BgEvent>,
    bg_rx: Receiver<BgEvent>,
    connected: bool,
    last_poll: Instant,
    status: String,
    installs: Vec<GameInstall>,
    playback: Playback,
    render: Render,
    recording: Recording,
    was_recording: bool,
    last_capture: Option<PathBuf>,
    particles: Particles,
    sequence: Sequence,
    apply_sequence: bool,
    rec_codec: String,
    rec_fps: f64,
    rec_start: f64,
    rec_end: f64,
    particle_filter: String,
    tab: usize,
    undo: Vec<Sequence>,
    redo: Vec<Sequence>,
    rofls: Vec<PathBuf>,
    selected_rofl: Option<PathBuf>,
    skyboxes: Vec<(String, PathBuf)>,
    lcu_busy: bool,
    lcu_line: String,
    last_lcu_check: Instant,
    dragging_kf: bool,
    seq_name: String,
    selected_kf: Option<(TrackId, usize)>,
    perm_ax: bool,
    perm_input: bool,
    perm_docs: bool,
    from_bundle: bool,
    lcu_ok: bool,
    game_ok: bool,
    api_ok: bool,
    handshake_log: String,
    pending_rofl: Option<PathBuf>,
    rec_blocked: bool,
    last_clip_secs: Option<f64>,
    last_hotkey: String,
}

impl DirectorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);
        permissions::request_hotkey_permissions();
        let mut settings = Settings::load();
        if settings.record_dir.trim().is_empty() {
            settings.record_dir = detect::preferred_record_dir().display().to_string();
        }
        let _ = std::fs::create_dir_all(&settings.record_dir);
        let client = ReplayClient::new().expect("http client");
        let installs = detect::find_installs();
        let rofls = detect::list_rofls();
        let skyboxes = detect::list_skyboxes();
        let (bg_tx, bg_rx) = mpsc::channel();
        if let Some(arg) = std::env::args().nth(1) {
            if arg.to_ascii_lowercase().ends_with(".rofl") {
                spawn_watch(bg_tx.clone(), PathBuf::from(arg));
            }
        }
        let bindings = settings.bindings.clone();
        let seq_name = settings.last_sequence.clone();
        let tab = settings.last_tab;
        Self {
            client,
            tab,
            rec_codec: "webm".into(),
            rec_fps: 30.0,
            rec_start: 0.0,
            rec_end: 10.0,
            particle_filter: String::new(),
            apply_sequence: false,
            settings,
            hotkeys: HotkeyBus::start(bindings),
            bg_tx,
            bg_rx,
            connected: false,
            last_poll: Instant::now() - Duration::from_secs(1),
            status: "Waiting for a replay…".into(),
            installs,
            playback: Playback::default(),
            render: Render::default(),
            recording: Recording::default(),
            was_recording: false,
            last_capture: None,
            particles: Particles::default(),
            sequence: Sequence::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            rofls,
            selected_rofl: None,
            skyboxes,
            lcu_busy: false,
            lcu_line: String::new(),
            last_lcu_check: Instant::now() - Duration::from_secs(10),
            dragging_kf: false,
            seq_name,
            selected_kf: None,
            perm_ax: permissions::accessibility_trusted(),
            perm_input: permissions::input_monitoring_ok(),
            perm_docs: permissions::documents_ok(),
            from_bundle: permissions::running_from_bundle(),
            lcu_ok: false,
            game_ok: detect::game_pid().is_some(),
            api_ok: false,
            handshake_log: String::new(),
            pending_rofl: None,
            rec_blocked: false,
            last_clip_secs: None,
            last_hotkey: String::new(),
        }
    }

    fn refresh_lcu_line(&mut self) {
        if self.last_lcu_check.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.last_lcu_check = Instant::now();
        self.game_ok = detect::game_pid().is_some();
        match lcu::Lcu::connect() {
            Ok(c) => {
                self.lcu_ok = true;
                let path = c.replay_path().unwrap_or_default();
                let playing = c.playing_replay().unwrap_or(false);
                self.lcu_line = if path.is_empty() {
                    format!("LCU up · playing_replay={playing}")
                } else {
                    format!("LCU up · playing_replay={playing} · {path}")
                };
            }
            Err(_) => {
                self.lcu_ok = false;
                self.lcu_line = "LCU not found (open the League client to watch a replay).".into();
            }
        }
    }

    fn drain_bg(&mut self) {
        while let Ok(ev) = self.bg_rx.try_recv() {
            match ev {
                BgEvent::Status(s) => {
                    self.status = s;
                }
                BgEvent::WatchDone(out) => {
                    self.lcu_busy = false;
                    self.rofls = detect::list_rofls();
                    self.handshake_log = handshake::format_outcome(&out);
                    match out {
                        WatchOutcome::Ready { pid } => {
                            self.game_ok = true;
                            self.api_ok = true;
                            self.rec_blocked = false;
                            self.status = format!("Replay API up (pid {pid}).");
                        }
                        WatchOutcome::Crashed { .. } => {
                            self.game_ok = false;
                            self.api_ok = false;
                            self.status = "Game launched then crashed before Replay API. See log below.".into();
                        }
                        WatchOutcome::Timeout { .. } => {
                            self.status = "Timed out waiting for Replay API.".into();
                        }
                    }
                }
                BgEvent::ClipReady { path, secs } => {
                    self.last_capture = Some(path.clone());
                    self.last_clip_secs = secs;
                    self.settings.remember_clip(&path);
                    if let Some(s) = secs {
                        if s < 1.0 {
                            self.status = format!(
                                "Capture short ({s:.2}s) — encode likely truncated: {}",
                                path.display()
                            );
                        } else {
                            self.status = format!("Capture ready ({s:.2}s): {}", path.display());
                        }
                    } else {
                        self.status = format!("Capture ready: {}", path.display());
                    }
                }
            }
        }
    }

    fn poll(&mut self) {
        let interval = if self.recording.recording {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(400)
        };
        if self.last_poll.elapsed() < interval {
            return;
        }
        self.last_poll = Instant::now();
        match self.client.game() {
            Ok(_) => {
                self.connected = true;
                self.api_ok = true;
                self.rec_blocked = false;
                if let Ok(p) = self.client.playback() {
                    self.playback = p;
                    if self.rec_end <= 0.1 {
                        self.rec_end = self.playback.length;
                    }
                }
                if let Ok(r) = self.client.render() {
                    self.render = r;
                }
                if let Ok(r) = self.client.recording() {
                    self.recording = r;
                }
                if let Ok(p) = self.client.particles() {
                    self.particles = p;
                }
                if self.was_recording && !self.recording.recording {
                    if let Some(path) =
                        media::finalize_recording(&self.recording.path, &self.settings.record_dir)
                    {
                        self.status = format!("Capture ready: {}", path.display());
                        self.last_capture = Some(path);
                    } else {
                        self.status = "Recording finished (file not found — check the output folder)".into();
                    }
                } else if !self.recording.recording && !self.lcu_busy {
                    self.status = format!(
                        "Connected · {} · cam {}",
                        Playback::format_time(self.playback.time),
                        if self.render.camera_mode.is_empty() {
                            "?"
                        } else {
                            &self.render.camera_mode
                        }
                    );
                }
                self.was_recording = self.recording.recording;
            }
            Err(_) => {
                self.connected = false;
                self.api_ok = false;
                self.game_ok = detect::game_pid().is_some();
                if self.was_recording || self.rec_blocked {
                    self.rec_blocked = true;
                    if let Some(path) =
                        media::finalize_recording(&self.recording.path, &self.settings.record_dir)
                    {
                        self.last_capture = Some(path.clone());
                        self.settings.remember_clip(&path);
                        self.status = format!(
                            "API down after record — remuxed {}. Do not Watch until Recover.",
                            path.display()
                        );
                    } else if !self.lcu_busy {
                        self.status = "API down after record. Wait or Recover — do not Watch yet.".into();
                    }
                    self.was_recording = false;
                } else if self.lcu_busy {
                    self.status = if self.game_ok {
                        "Game starting… waiting for Replay API (warmup 404s are normal).".into()
                    } else {
                        "LCU watch sent — waiting for LeagueofLegends process…".into()
                    };
                } else if self.game_ok {
                    self.status = "Game process is up, Replay API not ready yet…".into();
                } else {
                    self.status = "Replay API idle — Watch a .rofl or start a replay.".into();
                }
            }
        }
    }

    fn post_playback(&self, body: serde_json::Value) {
        let _ = self.client.set_playback(&body);
    }

    fn post_render(&self, body: serde_json::Value) {
        let _ = self.client.set_render(&body);
    }

    fn push_sequence(&self) {
        if self.apply_sequence {
            let _ = self.client.set_sequence(&self.sequence);
        }
    }

    fn snapshot(&mut self) {
        self.undo.push(self.sequence.clone());
        if self.undo.len() > 80 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.sequence.clone());
            self.sequence = prev;
            self.push_sequence();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.sequence.clone());
            self.sequence = next;
            self.push_sequence();
        }
    }

    fn add_camera_keyframes(&mut self) {
        self.snapshot();
        let t = self.playback.time;
        upsert_kf(
            &mut self.sequence.camera_position,
            t,
            self.render.camera_position,
            "smoothStep",
        );
        upsert_kf(
            &mut self.sequence.camera_rotation,
            t,
            self.render.camera_rotation,
            "smoothStep",
        );
        upsert_kf(
            &mut self.sequence.field_of_view,
            t,
            self.render.field_of_view,
            "linear",
        );
        self.apply_sequence = true;
        self.push_sequence();
    }

    fn apply_action(&mut self, action: Action) {
        self.last_hotkey = format!("{} @ {}", action.label(), Playback::format_time(self.playback.time));
        match action {
            Action::PlayPause => {
                let paused = !self.playback.paused;
                self.post_playback(serde_json::json!({ "paused": paused }));
                self.playback.paused = paused;
                let label = if paused { "Paused" } else { "Playing" };
                self.status = format!("{} · {}", label, self.last_hotkey);
                crate::hotkeys::notify("League Director", label);
            }
            Action::Keyframe => {
                self.add_camera_keyframes();
                self.tab = 4;
                self.status = format!("Keyframe · {}", self.last_hotkey);
                crate::hotkeys::notify("League Director", "Keyframe");
            }
            Action::SeekBack | Action::TimeMinus5 => {
                let t = (self.playback.time - 5.0).max(0.0);
                self.post_playback(serde_json::json!({ "time": t }));
                self.playback.time = t;
            }
            Action::SeekFwd | Action::TimePlus5 => {
                let t = (self.playback.time + 5.0).min(self.playback.length);
                self.post_playback(serde_json::json!({ "time": t }));
                self.playback.time = t;
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::PlaySeq => {
                if let Some(start) = self.sequence.first_time() {
                    self.apply_sequence = true;
                    self.push_sequence();
                    self.post_playback(serde_json::json!({ "time": start, "paused": false }));
                }
            }
            Action::ToggleHud => {
                self.post_render(serde_json::json!({ "interfaceAll": !self.render.interface_all }));
            }
            Action::ToggleFow => {
                self.post_render(serde_json::json!({ "fogOfWar": !self.render.fog_of_war }));
            }
            Action::ToggleAttach => {
                self.post_render(serde_json::json!({ "cameraAttached": !self.render.camera_attached }));
            }
            Action::RecordToggle => {
                if self.recording.recording {
                    let _ = self.client.set_recording(&serde_json::json!({ "recording": false }));
                } else if self.can_record() {
                    let start = self.playback.time;
                    self.start_recording(start, (start + 8.0).min(self.playback.length.max(start + 0.5)));
                }
            }
            Action::Cinematic => {
                let _ = self.client.set_render(&presets::cinematic());
            }
            Action::Gameplay => {
                let _ = self.client.set_render(&presets::gameplay());
            }
            Action::NextKf => self.step_kf(1),
            Action::PrevKf => self.step_kf(-1),
            Action::DeleteKf => self.delete_selected_kf(),
        }
    }

    fn handle_hotkeys(&mut self, ctx: &egui::Context) {
        if !ctx.wants_keyboard_input() {
            for action in Action::ALL {
                let chord = bindings::chord_of(&self.settings.bindings, *action);
                if bindings::egui_pressed(ctx, chord) {
                    self.apply_action(*action);
                }
            }
        }
        while let Some(news) = self.hotkeys.try_recv() {
            match news {
                crate::hotkeys::HotkeyNews::Fired(action) => {
                    self.last_hotkey = format!(
                        "{} @ {}",
                        action.label(),
                        Playback::format_time(self.playback.time)
                    );
                    self.status = format!("Hotkey · {}", self.last_hotkey);
                }
                crate::hotkeys::HotkeyNews::TapFailed => {
                    self.status = "Hotkeys need permission. Click Grant — macOS will name League Director.".into();
                    permissions::request_hotkey_permissions();
                }
            }
        }
    }

    fn kf_index(&self) -> Vec<(TrackId, usize, f64)> {
        let mut v = Vec::new();
        for (i, k) in self.sequence.camera_position.iter().enumerate() {
            v.push((TrackId::Position, i, k.time));
        }
        for (i, k) in self.sequence.camera_rotation.iter().enumerate() {
            v.push((TrackId::Rotation, i, k.time));
        }
        for (i, k) in self.sequence.field_of_view.iter().enumerate() {
            v.push((TrackId::Fov, i, k.time));
        }
        for (i, k) in self.sequence.playback_speed.iter().enumerate() {
            v.push((TrackId::Speed, i, k.time));
        }
        for (i, k) in self.sequence.depth_fog_enabled.iter().enumerate() {
            v.push((TrackId::Fog, i, k.time));
        }
        for (i, k) in self.sequence.depth_of_field_enabled.iter().enumerate() {
            v.push((TrackId::Dof, i, k.time));
        }
        for (i, k) in self.sequence.skybox_rotation.iter().enumerate() {
            v.push((TrackId::Sky, i, k.time));
        }
        for (i, k) in self.sequence.near_clip.iter().enumerate() {
            v.push((TrackId::Near, i, k.time));
        }
        v.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    fn step_kf(&mut self, dir: i32) {
        let all = self.kf_index();
        if all.is_empty() {
            return;
        }
        let cur = self.selected_kf.and_then(|(t, i)| {
            all.iter().position(|(tt, ii, _)| *tt == t && *ii == i)
        });
        let idx = match cur {
            Some(i) => {
                let n = all.len() as i32;
                ((i as i32 + dir).rem_euclid(n)) as usize
            }
            None => {
                if dir >= 0 {
                    0
                } else {
                    all.len() - 1
                }
            }
        };
        let (track, index, time) = all[idx];
        self.selected_kf = Some((track, index));
        self.post_playback(serde_json::json!({ "time": time, "paused": true }));
    }

    fn delete_selected_kf(&mut self) {
        let Some((track, index)) = self.selected_kf else {
            return;
        };
        self.snapshot();
        match track {
            TrackId::Position => {
                if index < self.sequence.camera_position.len() {
                    self.sequence.camera_position.remove(index);
                }
            }
            TrackId::Rotation => {
                if index < self.sequence.camera_rotation.len() {
                    self.sequence.camera_rotation.remove(index);
                }
            }
            TrackId::Fov => {
                if index < self.sequence.field_of_view.len() {
                    self.sequence.field_of_view.remove(index);
                }
            }
            TrackId::Speed => {
                if index < self.sequence.playback_speed.len() {
                    self.sequence.playback_speed.remove(index);
                }
            }
            TrackId::Fog => {
                if index < self.sequence.depth_fog_enabled.len() {
                    self.sequence.depth_fog_enabled.remove(index);
                }
            }
            TrackId::Dof => {
                if index < self.sequence.depth_of_field_enabled.len() {
                    self.sequence.depth_of_field_enabled.remove(index);
                }
            }
            TrackId::Sky => {
                if index < self.sequence.skybox_rotation.len() {
                    self.sequence.skybox_rotation.remove(index);
                }
            }
            TrackId::Near => {
                if index < self.sequence.near_clip.len() {
                    self.sequence.near_clip.remove(index);
                }
            }
        }
        self.selected_kf = None;
        self.push_sequence();
    }

    fn can_record(&self) -> bool {
        self.connected && !self.recording.recording && !self.rec_blocked && !self.lcu_busy
    }

    fn open_rofl(&mut self, path: PathBuf) {
        if self.rec_blocked {
            self.status = "API still down after a recording. Recover first, then Watch.".into();
            return;
        }
        self.lcu_busy = true;
        self.pending_rofl = Some(path.clone());
        self.handshake_log.clear();
        self.status = format!("Watch sent — waiting for game + Replay API ({})…", path.display());
        spawn_watch(self.bg_tx.clone(), path);
    }

    fn recover_after_record(&mut self) {
        if let Some(path) =
            media::finalize_recording(&self.recording.path, &self.settings.record_dir)
        {
            self.last_capture = Some(path.clone());
            self.settings.remember_clip(&path);
            self.last_clip_secs = media::probe_duration(&path);
        }
        self.rec_blocked = false;
        self.was_recording = false;
        self.status = "Recovered remux. If the game is dead, Watch again after a few seconds.".into();
    }

    fn start_recording(&mut self, start: f64, end: f64) {
        if !self.can_record() {
            self.status = "Cannot record: need a live Replay API and no previous hung encode.".into();
            return;
        }
        let _ = std::fs::create_dir_all(&self.settings.record_dir);
        if self.apply_sequence {
            self.push_sequence();
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let ext = if self.rec_codec == "png" { "png" } else { "webm" };
        let file = format!("capture_{stamp}.{ext}");
        let path = PathBuf::from(&self.settings.record_dir).join(file);
        let body = serde_json::json!({
            "recording": true,
            "codec": self.rec_codec,
            "startTime": start,
            "endTime": end,
            "framesPerSecond": self.rec_fps,
            "enforceFrameRate": true,
            "lossless": false,
            "path": path,
        });
        self.rec_start = start;
        self.rec_end = end;
        self.was_recording = true;
        self.status = format!("Recording → {}", path.display());
        let client = self.client.clone();
        let tx = self.bg_tx.clone();
        let hint = path.display().to_string();
        let dest = self.settings.record_dir.clone();
        let span = (end - start).max(0.5);
        thread::spawn(move || {
            if let Err(e) = client.set_recording(&body) {
                let _ = tx.send(BgEvent::Status(format!("Recording API: {e}")));
            }
        });
        let tx2 = self.bg_tx.clone();
        media::spawn_watchdog(hint, dest, span, move |maybe, secs| {
            if let Some(p) = maybe {
                let _ = tx2.send(BgEvent::ClipReady { path: p, secs });
            }
        });
    }
}

impl eframe::App for DirectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_bg();
        self.refresh_lcu_line();
        self.poll();
        self.handle_hotkeys(ctx);
        ctx.request_repaint_after(Duration::from_millis(200));

        if self.settings.show_hud {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("director_hud"),
                egui::ViewportBuilder::default()
                    .with_title("Director")
                    .with_inner_size([460.0, 92.0])
                    .with_min_inner_size([320.0, 72.0])
                    .with_always_on_top()
                    .with_maximize_button(false)
                    .with_close_button(true),
                |ctx, class| {
                    let _ = class;
                    self.ui_hud(ctx);
                    self.handle_hotkeys(ctx);
                    if ctx.input(|i| i.viewport().close_requested()) {
                        self.settings.show_hud = false;
                        self.settings.save();
                    }
                },
            );
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("League Director");
                ui.separator();
                let color = if self.connected {
                    Color32::from_rgb(80, 200, 140)
                } else if self.lcu_busy {
                    Color32::from_rgb(230, 190, 80)
                } else {
                    Color32::from_rgb(220, 120, 80)
                };
                ui.label(RichText::new(&self.status).color(color));
                ui.label(RichText::new(crate::hotkeys::debug_snapshot()).small().weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.selectable_label(self.settings.show_hud, "HUD").clicked() {
                        self.settings.show_hud = !self.settings.show_hud;
                        self.settings.save();
                    }
                    if ui.button("Files & Folders").clicked() {
                        permissions::open_files_privacy();
                    }
                    if ui.button("Grant keys").clicked() {
                        permissions::request_hotkey_permissions();
                    }
                    if ui.button("Refresh").clicked() {
                        self.installs = detect::find_installs();
                        self.rofls = detect::list_rofls();
                        self.skyboxes = detect::list_skyboxes();
                        self.perm_ax = permissions::accessibility_trusted();
                        self.perm_input = permissions::input_monitoring_ok();
                        self.perm_docs = permissions::documents_ok();
                    }
                });
            });
            ui.horizontal(|ui| {
                for (i, label) in [
                    "Connect",
                    "Timeline",
                    "Render",
                    "Visibility",
                    "Sequencer",
                    "Recording",
                    "Particles",
                    "Keys",
                ]
                .iter()
                .enumerate()
                {
                    if ui.selectable_label(self.tab == i, *label).clicked() {
                        self.tab = i;
                        self.settings.last_tab = i;
                        self.settings.save();
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            0 => self.ui_connect(ui),
            1 => self.ui_timeline(ui),
            2 => self.ui_render(ui),
            3 => self.ui_visibility(ui),
            4 => self.ui_sequence(ui),
            5 => self.ui_recording(ui),
            6 => self.ui_particles(ui),
            _ => self.ui_keys(ui),
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

impl DirectorApp {
    fn ui_connect(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("macOS permissions").strong());
        perm_row(ui, "Accessibility (global hotkeys)", self.perm_ax, || {
            permissions::request_hotkey_permissions();
        });
        perm_row(ui, "Input Monitoring (read keys in League)", self.perm_input, || {
            permissions::request_hotkey_permissions();
        });
        if !self.perm_ax || !self.perm_input {
            ui.label(
                RichText::new("Grant names this app. After you allow, restart League Director.")
                    .small()
                    .weak(),
            );
            if ui.small_button("Open Settings only if the prompt did not appear").clicked() {
                permissions::open_privacy_settings();
            }
        }
        perm_row(ui, "Documents / Files and Folders", self.perm_docs, || {
            permissions::open_files_privacy();
        });
        perm_row(
            ui,
            if self.from_bundle {
                "Running as League Director.app"
            } else {
                "Running from cargo — TCC grants will not stick to the .app"
            },
            self.from_bundle,
            || {},
        );
        ui.horizontal(|ui| {
            if ui.button("Install to /Applications").clicked() {
                match permissions::install_to_applications() {
                    Ok(p) => {
                        self.status = format!("Installed {}", p.display());
                        permissions::open_folder(&p);
                    }
                    Err(e) => self.status = e,
                }
            }
            ui.label(
                RichText::new("Then open that copy and grant the three privacy toggles to it.")
                    .small()
                    .weak(),
            );
        });
        ui.separator();
        ui.label(RichText::new("Live status").strong());
        perm_row(ui, "LCU (League client lockfile)", self.lcu_ok, || {});
        perm_row(ui, "LeagueofLegends process", self.game_ok, || {});
        perm_row(ui, "Replay API https://127.0.0.1:2999", self.api_ok, || {});
        if self.rec_blocked {
            ui.colored_label(
                Color32::from_rgb(220, 120, 80),
                "Recording blocked: last encode killed the API. Recover before Watch.",
            );
            if ui.button("Recover remux").clicked() {
                self.recover_after_record();
            }
        }
        if !self.handshake_log.is_empty() {
            ui.collapsing("Last watch handshake", |ui| {
                ui.label(RichText::new(&self.handshake_log).small().monospace());
            });
            if !self.api_ok {
                if ui.button("Retry last .rofl").clicked() {
                    if let Some(p) = self.pending_rofl.clone().or_else(|| self.selected_rofl.clone()) {
                        self.open_rofl(p);
                    }
                }
            }
        }
        ui.separator();
        ui.label("1. Tick an install to write EnableReplayApi=1 into game.cfg.");
        ui.label("2. Open a .rofl here (copied into League Replays + LCU watch) — do not pass it as argv.");
        ui.add_space(8.0);
        if self.installs.is_empty() {
            ui.colored_label(Color32::YELLOW, "No League install found.");
        }
        let mut dirty: Option<(usize, bool)> = None;
        for (i, inst) in self.installs.iter().enumerate() {
            ui.horizontal(|ui| {
                let mut on = inst.enabled;
                if ui.checkbox(&mut on, inst.cfg.display().to_string()).changed() {
                    dirty = Some((i, on));
                }
            });
        }
        if let Some((i, on)) = dirty {
            if let Err(e) = detect::set_enabled(&self.installs[i].cfg, on) {
                self.status = format!("game.cfg write: {e}");
            }
            self.installs = detect::find_installs();
        }

        ui.add_space(12.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Replays");
            if ui.button("Scan").clicked() {
                self.rofls = detect::list_rofls();
            }
            if ui.button("Open .rofl…").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("rofl", &["rofl"]).pick_file() {
                    self.selected_rofl = Some(path.clone());
                    self.open_rofl(path);
                }
            }
            if ui.button("Documents privacy").clicked() {
                permissions::open_files_privacy();
            }
            if let Some(dir) = detect::preferred_replay_dir() {
                if ui.small_button("Open League Replays").clicked() {
                    permissions::open_folder(&dir);
                }
            }
        });
        ui.label(
            RichText::new(
                "On Mac, League mainly reads Contents/LoL/Replays. The app copies the file before watch.",
            )
            .small()
            .weak(),
        );
        if self.rofls.is_empty() {
            ui.colored_label(
                Color32::YELLOW,
                "No .rofl visible. Allow Files and Folders, or pick a file.",
            );
        }
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                for path in self.rofls.clone() {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("replay")
                        .to_string();
                    let parent = path
                        .parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    let selected = self.selected_rofl.as_ref() == Some(&path);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &name).clicked() {
                            self.selected_rofl = Some(path.clone());
                        }
                        ui.label(RichText::new(parent).small().weak());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Watch").clicked() {
                                self.selected_rofl = Some(path.clone());
                                self.open_rofl(path.clone());
                            }
                        });
                    });
                }
            });
        if self.lcu_busy {
            ui.colored_label(Color32::LIGHT_BLUE, "LCU busy (scan + metadata + watch)…");
        }
        if !self.lcu_line.is_empty() {
            ui.label(RichText::new(&self.lcu_line).small());
        }
        ui.add_space(8.0);
        ui.label("Replay API: https://127.0.0.1:2999  ·  not affiliated with Riot Games.");
    }

    fn ui_timeline(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.connected, |ui| {
            ui.horizontal(|ui| {
                let label = if self.playback.paused { "Play" } else { "Pause" };
                if ui.button(label).clicked() {
                    self.post_playback(serde_json::json!({ "paused": !self.playback.paused }));
                }
                ui.label(format!(
                    "{} / {}",
                    Playback::format_time(self.playback.time),
                    Playback::format_time(self.playback.length)
                ));
            });
            let mut t = self.playback.time;
            if ui
                .add(Slider::new(&mut t, 0.0..=self.playback.length.max(1.0)).text("Time"))
                .changed()
            {
                self.post_playback(serde_json::json!({ "time": t, "paused": true }));
            }
            let mut speed = if self.playback.speed <= 0.0 {
                1.0
            } else {
                self.playback.speed
            };
            if ui.add(Slider::new(&mut speed, 0.0..=8.0).text("Speed")).changed() {
                self.post_playback(serde_json::json!({ "speed": speed }));
            }
            ui.horizontal(|ui| {
                for d in [-120.0, -30.0, -10.0, -5.0, 5.0, 10.0, 30.0, 120.0] {
                    let lab = if d > 0.0 {
                        format!("+{d}s")
                    } else {
                        format!("{d}s")
                    };
                    if ui.button(lab).clicked() {
                        self.post_playback(serde_json::json!({
                            "time": (self.playback.time + d).clamp(0.0, self.playback.length)
                        }));
                    }
                }
            });
        });
    }

    fn ui_render(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.connected, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let client = self.client.clone();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Presets").strong());
                    if ui.button("Cinematic").clicked() {
                        let _ = client.set_render(&presets::cinematic());
                    }
                    if ui.button("Gameplay").clicked() {
                        let _ = client.set_render(&presets::gameplay());
                    }
                    if ui.button("Broadcast").clicked() {
                        let _ = client.set_render(&presets::broadcast());
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Camera mode");
                    ui.label(&self.render.camera_mode);
                    if ui.button("FPS").clicked() {
                        let _ = client.set_render(&serde_json::json!({ "cameraMode": "fps" }));
                    }
                    if ui.button("Top").clicked() {
                        let _ = client.set_render(&serde_json::json!({ "cameraMode": "top" }));
                    }
                });
                vec3_row(ui, "Position", &mut self.render.camera_position, |v| {
                    let _ = client.set_render(&serde_json::json!({ "cameraPosition": v }));
                });
                vec3_row(ui, "Rotation", &mut self.render.camera_rotation, |v| {
                    let _ = client.set_render(&serde_json::json!({ "cameraRotation": v }));
                });
                float_row(ui, "FOV", &mut self.render.field_of_view, 5.0..=120.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "fieldOfView": v }));
                });
                float_row(ui, "Near clip", &mut self.render.near_clip, 0.1..=5000.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "nearClip": v }));
                });
                float_row(ui, "Far clip", &mut self.render.far_clip, 100.0..=100000.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "farClip": v }));
                });
                float_row(
                    ui,
                    "Move speed",
                    &mut self.render.camera_move_speed,
                    0.0..=10000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "cameraMoveSpeed": v }));
                    },
                );
                float_row(
                    ui,
                    "Look speed",
                    &mut self.render.camera_look_speed,
                    0.0..=5.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "cameraLookSpeed": v }));
                    },
                );
                bool_row(ui, "Camera attached", &mut self.render.camera_attached, |v| {
                    let _ = client.set_render(&serde_json::json!({ "cameraAttached": v }));
                });
                bool_row(ui, "Lock X", &mut self.render.camera_lock_x, |v| {
                    let _ = client.set_render(&serde_json::json!({ "cameraLockX": v }));
                });
                bool_row(ui, "Lock Y", &mut self.render.camera_lock_y, |v| {
                    let _ = client.set_render(&serde_json::json!({ "cameraLockY": v }));
                });
                bool_row(ui, "Lock Z", &mut self.render.camera_lock_z, |v| {
                    let _ = client.set_render(&serde_json::json!({ "cameraLockZ": v }));
                });

                ui.separator();
                ui.label(RichText::new("Skybox").strong());
                ui.horizontal(|ui| {
                    let current = self
                        .render
                        .skybox_path
                        .rsplit('/')
                        .next()
                        .unwrap_or(&self.render.skybox_path)
                        .to_string();
                    egui::ComboBox::from_id_salt("skybox")
                        .selected_text(if current.is_empty() {
                            "(game default)".to_string()
                        } else {
                            current
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.render.skybox_path.is_empty(), "(game default)")
                                .clicked()
                            {
                                let _ = client.set_render(&serde_json::json!({ "skyboxPath": "" }));
                            }
                            for (label, path) in &self.skyboxes {
                                let p = path.display().to_string();
                                if ui
                                    .selectable_label(self.render.skybox_path == p, label)
                                    .clicked()
                                {
                                    let _ = client.set_render(&serde_json::json!({ "skyboxPath": p }));
                                }
                            }
                        });
                    if ui.small_button("Choose .dds…").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("dds", &["dds"]).pick_file() {
                            let p = path.display().to_string();
                            let _ = client.set_render(&serde_json::json!({ "skyboxPath": p }));
                            self.skyboxes = detect::list_skyboxes();
                        }
                    }
                    if ui.small_button("Skybox folder").clicked() {
                        let dir = crate::settings::config_dir().join("skyboxes");
                        let _ = std::fs::create_dir_all(&dir);
                        permissions::open_folder(&dir);
                    }
                });
                if self.skyboxes.is_empty() {
                    ui.label(
                        RichText::new(
                            "No .dds found. Copy skyboxes (from a Python resources/skyboxes install) into ~/Documents/LeagueDirector/skyboxes.",
                        )
                        .small()
                        .weak(),
                    );
                }
                float_row(
                    ui,
                    "Skybox rotation",
                    &mut self.render.skybox_rotation,
                    -180.0..=180.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "skyboxRotation": v }));
                    },
                );
                float_row(
                    ui,
                    "Skybox radius",
                    &mut self.render.skybox_radius,
                    0.0..=100000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "skyboxRadius": v }));
                    },
                );
                float_row(
                    ui,
                    "Skybox offset",
                    &mut self.render.skybox_offset,
                    -100000.0..=100000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "skyboxOffset": v }));
                    },
                );
                vec3_row(ui, "Soleil", &mut self.render.sun_direction, |v| {
                    let _ = client.set_render(&serde_json::json!({ "sunDirection": v }));
                });

                ui.separator();
                bool_row(ui, "Fog depth", &mut self.render.depth_fog_enabled, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthFogEnabled": v }));
                });
                float_row(ui, "Fog start", &mut self.render.depth_fog_start, 0.0..=100000.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthFogStart": v }));
                });
                float_row(ui, "Fog end", &mut self.render.depth_fog_end, 0.0..=100000.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthFogEnd": v }));
                });
                float_row(
                    ui,
                    "Fog intensity",
                    &mut self.render.depth_fog_intensity,
                    0.0..=1.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "depthFogIntensity": v }));
                    },
                );
                color_row(ui, "Fog color", &mut self.render.depth_fog_color, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthFogColor": v }));
                });
                bool_row(ui, "Fog height", &mut self.render.height_fog_enabled, |v| {
                    let _ = client.set_render(&serde_json::json!({ "heightFogEnabled": v }));
                });
                float_row(
                    ui,
                    "Height fog start",
                    &mut self.render.height_fog_start,
                    -10000.0..=10000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "heightFogStart": v }));
                    },
                );
                float_row(
                    ui,
                    "Height fog end",
                    &mut self.render.height_fog_end,
                    -10000.0..=10000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "heightFogEnd": v }));
                    },
                );
                float_row(
                    ui,
                    "Height fog intensity",
                    &mut self.render.height_fog_intensity,
                    0.0..=1.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "heightFogIntensity": v }));
                    },
                );

                ui.separator();
                bool_row(ui, "Depth of field", &mut self.render.depth_of_field_enabled, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthOfFieldEnabled": v }));
                });
                bool_row(ui, "DOF debug", &mut self.render.depth_of_field_debug, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthOfFieldDebug": v }));
                });
                float_row(
                    ui,
                    "DOF circle",
                    &mut self.render.depth_of_field_circle,
                    0.0..=300.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "depthOfFieldCircle": v }));
                    },
                );
                float_row(
                    ui,
                    "DOF width",
                    &mut self.render.depth_of_field_width,
                    0.0..=10000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "depthOfFieldWidth": v }));
                    },
                );
                float_row(
                    ui,
                    "DOF near",
                    &mut self.render.depth_of_field_near,
                    0.0..=100000.0,
                    |v| {
                        let _ = client.set_render(&serde_json::json!({ "depthOfFieldNear": v }));
                    },
                );
                float_row(ui, "DOF mid", &mut self.render.depth_of_field_mid, 0.0..=100000.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthOfFieldMid": v }));
                });
                float_row(ui, "DOF far", &mut self.render.depth_of_field_far, 0.0..=100000.0, |v| {
                    let _ = client.set_render(&serde_json::json!({ "depthOfFieldFar": v }));
                });

                ui.separator();
                if ui.button("Keyframe position + rotation + FOV (K)").clicked() {
                    self.add_camera_keyframes();
                    self.tab = 4;
                }
            });
        });
    }

    fn ui_visibility(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.connected, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (label, field, key) in [
                    ("Fog of war", self.render.fog_of_war, "fogOfWar"),
                    ("UI", self.render.interface_all, "interfaceAll"),
                    ("Replay UI", self.render.interface_replay, "interfaceReplay"),
                    ("Minimap", self.render.interface_minimap, "interfaceMinimap"),
                    ("Timeline", self.render.interface_timeline, "interfaceTimeline"),
                    ("Score", self.render.interface_score, "interfaceScore"),
                    ("Scoreboard", self.render.interface_scoreboard, "interfaceScoreboard"),
                    ("Chat", self.render.interface_chat, "interfaceChat"),
                    ("Announcements", self.render.interface_announce, "interfaceAnnounce"),
                    ("Kill callouts", self.render.interface_kill_callouts, "interfaceKillCallouts"),
                    ("Quests", self.render.interface_quests, "interfaceQuests"),
                    ("Frames", self.render.interface_frames, "interfaceFrames"),
                    ("Target", self.render.interface_target, "interfaceTarget"),
                    ("Champions", self.render.champions, "champions"),
                    ("Minions", self.render.minions, "minions"),
                    ("Characters", self.render.characters, "characters"),
                    ("Particles", self.render.particles, "particles"),
                    ("Banners", self.render.banners, "banners"),
                    ("Environment", self.render.environment, "environment"),
                    ("HP champions", self.render.health_bar_champions, "healthBarChampions"),
                    ("HP structures", self.render.health_bar_structures, "healthBarStructures"),
                    ("HP wards", self.render.health_bar_wards, "healthBarWards"),
                    ("HP pets", self.render.health_bar_pets, "healthBarPets"),
                    ("HP minions", self.render.health_bar_minions, "healthBarMinions"),
                ] {
                    let mut v = field;
                    if ui.checkbox(&mut v, label).changed() {
                        self.post_render(serde_json::json!({ key: v }));
                    }
                }
            });
        });
    }

    fn ui_sequence(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.connected, |ui| {
            ui.label(
                RichText::new(
                    "Shortcuts (Keys tab to remap) also work while League is frontmost.",
                )
                .small(),
            );
            ui.horizontal(|ui| {
                ui.label("Sequence");
                ui.text_edit_singleline(&mut self.seq_name);
                if ui.button("New").clicked() {
                    self.snapshot();
                    self.sequence = Sequence::default();
                    self.seq_name.clear();
                    let _ = self.client.clear_sequence();
                }
                if ui.button("Save").clicked() {
                    match seq_lib::save(&self.settings.sequence_dir, &self.seq_name, &self.sequence) {
                        Ok(p) => {
                            self.settings.last_sequence = self.seq_name.clone();
                            self.settings.save();
                            self.status = format!("Saved {}", p.display());
                        }
                        Err(e) => self.status = e,
                    }
                }
                if ui.button("Copy").clicked() {
                    if !self.seq_name.is_empty() {
                        let name = format!("{} copy", self.seq_name);
                        if seq_lib::save(&self.settings.sequence_dir, &name, &self.sequence).is_ok() {
                            self.seq_name = name;
                        }
                    }
                }
                let files = seq_lib::list(&self.settings.sequence_dir);
                let current = self.settings.last_sequence.clone();
                egui::ComboBox::from_id_salt("seq-lib")
                    .selected_text(if current.is_empty() {
                        "(library)".into()
                    } else {
                        current
                    })
                    .show_ui(ui, |ui| {
                        for p in files {
                            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("seq");
                            if ui.selectable_label(false, stem).clicked() {
                                if let Ok(seq) = seq_lib::load(&p) {
                                    self.snapshot();
                                    self.sequence = seq;
                                    self.seq_name = stem.to_string();
                                    self.settings.last_sequence = stem.to_string();
                                    self.apply_sequence = true;
                                    self.push_sequence();
                                    self.settings.save();
                                }
                            }
                        }
                    });
                if ui.small_button("Folder").clicked() {
                    let _ = std::fs::create_dir_all(&self.settings.sequence_dir);
                    permissions::open_folder(std::path::Path::new(&self.settings.sequence_dir));
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Prev KF").clicked() {
                    self.step_kf(-1);
                }
                if ui.button("Next KF").clicked() {
                    self.step_kf(1);
                }
                if ui.button("Delete KF").clicked() {
                    self.delete_selected_kf();
                }
            });
            if ui
                .checkbox(&mut self.apply_sequence, "Apply sequence to replay")
                .changed()
            {
                if self.apply_sequence {
                    self.push_sequence();
                } else {
                    let _ = self.client.clear_sequence();
                }
            }
            ui.horizontal(|ui| {
                if ui.button("Play sequence").clicked() {
                    if let Some(start) = self.sequence.first_time() {
                        self.apply_sequence = true;
                        self.push_sequence();
                        self.post_playback(serde_json::json!({ "time": start, "paused": false }));
                    }
                }
                if ui.button("KF cam (K)").clicked() {
                    self.add_camera_keyframes();
                }
                if ui.button("KF FOV").clicked() {
                    self.snapshot();
                    upsert_kf(
                        &mut self.sequence.field_of_view,
                        self.playback.time,
                        self.render.field_of_view,
                        "linear",
                    );
                    self.apply_sequence = true;
                    self.push_sequence();
                }
                if ui.button("KF speed").clicked() {
                    self.snapshot();
                    upsert_kf(
                        &mut self.sequence.playback_speed,
                        self.playback.time,
                        self.playback.speed,
                        "linear",
                    );
                    self.apply_sequence = true;
                    self.push_sequence();
                }
                if ui.button("KF fog").clicked() {
                    self.snapshot();
                    let t = self.playback.time;
                    upsert_kf(
                        &mut self.sequence.depth_fog_enabled,
                        t,
                        self.render.depth_fog_enabled,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.depth_fog_start,
                        t,
                        self.render.depth_fog_start,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.depth_fog_end,
                        t,
                        self.render.depth_fog_end,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.depth_fog_intensity,
                        t,
                        self.render.depth_fog_intensity,
                        "linear",
                    );
                    self.apply_sequence = true;
                    self.push_sequence();
                }
                if ui.button("KF DOF").clicked() {
                    self.snapshot();
                    let t = self.playback.time;
                    upsert_kf(
                        &mut self.sequence.depth_of_field_enabled,
                        t,
                        self.render.depth_of_field_enabled,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.depth_of_field_circle,
                        t,
                        self.render.depth_of_field_circle,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.depth_of_field_mid,
                        t,
                        self.render.depth_of_field_mid,
                        "linear",
                    );
                    self.apply_sequence = true;
                    self.push_sequence();
                }
                if ui.button("KF sky").clicked() {
                    self.snapshot();
                    let t = self.playback.time;
                    upsert_kf(
                        &mut self.sequence.skybox_rotation,
                        t,
                        self.render.skybox_rotation,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.skybox_radius,
                        t,
                        self.render.skybox_radius,
                        "linear",
                    );
                    upsert_kf(
                        &mut self.sequence.skybox_offset,
                        t,
                        self.render.skybox_offset,
                        "linear",
                    );
                    self.apply_sequence = true;
                    self.push_sequence();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Undo").clicked() {
                    self.undo();
                }
                if ui.button("Redo").clicked() {
                    self.redo();
                }
                if ui.button("Save JSON").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("json", &["json"])
                        .set_file_name("sequence.json")
                        .save_file()
                    {
                        if let Ok(json) = serde_json::to_string_pretty(&self.sequence) {
                            let _ = std::fs::write(path, json);
                        }
                    }
                }
                if ui.button("Load JSON").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("json", &["json"]).pick_file() {
                        if let Ok(text) = std::fs::read_to_string(path) {
                            if let Ok(seq) = serde_json::from_str::<Sequence>(&text) {
                                self.snapshot();
                                self.sequence = seq;
                                self.apply_sequence = true;
                                self.push_sequence();
                            }
                        }
                    }
                }
                if ui.button("Clear").clicked() {
                    self.snapshot();
                    self.sequence = Sequence::default();
                    let _ = self.client.clear_sequence();
                }
            });

            let ev = sequence_ui::draw_tracks(
                ui,
                &self.sequence,
                self.playback.time,
                self.playback.length.max(1.0),
            );
            if let Some(ev) = ev {
                match ev {
                    TrackEvent::Seek(t) => {
                        self.post_playback(serde_json::json!({ "time": t, "paused": true }));
                    }
                    TrackEvent::Move {
                        track,
                        index,
                        time,
                    } => {
                        if !self.dragging_kf {
                            self.snapshot();
                            self.dragging_kf = true;
                        }
                        sequence_ui::apply_move(&mut self.sequence, track, index, time);
                        self.apply_sequence = true;
                        self.push_sequence();
                    }
                }
            } else if self.dragging_kf {
                self.dragging_kf = false;
                self.sequence.sort_all();
                self.push_sequence();
            }

            ui.separator();
            ui.label(format!(
                "pos {} · rot {} · fov {} · speed {} · fog {} · dof {}",
                self.sequence.camera_position.len(),
                self.sequence.camera_rotation.len(),
                self.sequence.field_of_view.len(),
                self.sequence.playback_speed.len(),
                self.sequence.depth_fog_enabled.len(),
                self.sequence.depth_of_field_enabled.len()
            ));
            let mut dirty = false;
            dirty |= sequence_ui::keyframe_table_vec3(ui, "cameraPosition", &mut self.sequence.camera_position);
            dirty |= sequence_ui::keyframe_table_vec3(ui, "cameraRotation", &mut self.sequence.camera_rotation);
            dirty |= sequence_ui::keyframe_table_f64(ui, "fieldOfView", &mut self.sequence.field_of_view);
            dirty |= sequence_ui::keyframe_table_f64(ui, "playbackSpeed", &mut self.sequence.playback_speed);
            dirty |= sequence_ui::keyframe_table_bool(ui, "depthFogEnabled", &mut self.sequence.depth_fog_enabled);
            dirty |= sequence_ui::keyframe_table_f64(ui, "depthFogStart", &mut self.sequence.depth_fog_start);
            dirty |= sequence_ui::keyframe_table_bool(
                ui,
                "depthOfFieldEnabled",
                &mut self.sequence.depth_of_field_enabled,
            );
            dirty |= sequence_ui::keyframe_table_f64(ui, "skyboxRotation", &mut self.sequence.skybox_rotation);
            dirty |= sequence_ui::keyframe_table_color(ui, "depthFogColor", &mut self.sequence.depth_fog_color);
            if dirty {
                self.apply_sequence = true;
                self.push_sequence();
            }
        });
    }

    fn ui_recording(&mut self, ui: &mut Ui) {
        if !media::ffmpeg_available() {
            ui.colored_label(
                Color32::YELLOW,
                "ffmpeg not found — .webm.tmp remux will be a raw copy.",
            );
        }
        if self.rec_blocked {
            ui.colored_label(
                Color32::YELLOW,
                "Previous encode dropped the Replay API. Recover before recording again.",
            );
            if ui.button("Recover remux").clicked() {
                self.recover_after_record();
            }
        }
        if let Some(secs) = self.last_clip_secs {
            if secs < 1.0 {
                ui.colored_label(
                    Color32::YELLOW,
                    format!("Last clip is only {secs:.2}s — League likely truncated the encode."),
                );
            }
        }
        ui.add_enabled_ui(self.can_record(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Folder");
                ui.text_edit_singleline(&mut self.settings.record_dir);
                if ui.button("…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.settings.record_dir = dir.display().to_string();
                        self.settings.save();
                    }
                }
                if ui.button("League folder (recommended)").clicked() {
                    let dir = detect::preferred_record_dir();
                    self.settings.record_dir = dir.display().to_string();
                    self.settings.save();
                }
            });
            ui.label(
                RichText::new(
                    "League writes more reliably under Contents/LoL/Replays/… than Documents (TCC). The app remuxes afterward.",
                )
                .small()
                .weak(),
            );
            ui.horizontal(|ui| {
                ui.label("Codec");
                ui.selectable_value(&mut self.rec_codec, "webm".into(), "webm");
                ui.selectable_value(&mut self.rec_codec, "png".into(), "png");
            });
            ui.add(Slider::new(&mut self.rec_fps, 10.0..=60.0).text("FPS"));
            ui.add(Slider::new(&mut self.rec_start, 0.0..=self.playback.length.max(1.0)).text("Start"));
            ui.add(Slider::new(&mut self.rec_end, 0.0..=self.playback.length.max(1.0)).text("End"));
            if ui.button("Record").clicked() {
                self.start_recording(self.rec_start, self.rec_end);
            }
            if ui.button("Record sequence (auto range)").clicked() {
                if let (Some(a), Some(b)) = (self.sequence.first_time(), self.sequence.last_time()) {
                    self.apply_sequence = true;
                    self.push_sequence();
                    self.start_recording(a, b.max(a + 0.5));
                }
            }
        });
        if self.recording.recording {
            ui.colored_label(
                Color32::LIGHT_BLUE,
                "Recording… the game API may stall until the clip ends.",
            );
            let span = (self.recording.end_time - self.recording.start_time).max(0.01);
            let p = ((self.recording.current_time - self.recording.start_time) / span) as f32;
            ui.add(egui::ProgressBar::new(p.clamp(0.0, 1.0)));
            ui.label(&self.recording.path);
            if ui.button("Annuler").clicked() {
                let _ = self.client.set_recording(&serde_json::json!({ "recording": false }));
            }
        }
        if let Some(path) = &self.last_capture {
            ui.horizontal(|ui| {
                ui.label(format!("Last capture: {}", path.display()));
                if ui.button("Reveal").clicked() {
                    permissions::reveal_in_finder(path);
                }
            });
        }
        ui.separator();
        ui.label(RichText::new("Clips").strong());
        let rec_alt = detect::preferred_record_dir();
        let dirs = [
            self.settings.record_dir.as_str(),
            rec_alt.to_str().unwrap_or(""),
        ];
        let clips = media::list_clips(&dirs);
        if clips.is_empty() && self.settings.clips.is_empty() {
            ui.label(RichText::new("No clips yet.").small().weak());
        }
        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
            for p in clips.into_iter().chain(
                self.settings
                    .clips
                    .iter()
                    .map(PathBuf::from)
                    .filter(|p| p.is_file()),
            ) {
                ui.horizontal(|ui| {
                    ui.label(
                        p.file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("clip")
                            .to_string(),
                    );
                    if ui.small_button("Reveal").clicked() {
                        permissions::reveal_in_finder(&p);
                    }
                    if ui.small_button("Open").clicked() {
                        permissions::open_folder(&p);
                    }
                });
            }
        });
    }

    fn ui_keys(&mut self, ui: &mut Ui) {
        ui.label("Bindings apply in this window and while League of Legends is frontmost.");
        ui.label(
            RichText::new("Grant Accessibility + Input Monitoring to the .app, not to cargo.")
                .small()
                .weak(),
        );
        if ui.button("Reset to defaults").clicked() {
            self.settings.bindings = bindings::default_map();
            self.hotkeys.update_bindings(self.settings.bindings.clone());
            self.settings.save();
        }
        let mut dirty = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for action in Action::ALL {
                let id = action.id().to_string();
                let mut value = self
                    .settings
                    .bindings
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| action.default_chord().to_string());
                ui.horizontal(|ui| {
                    ui.label(action.label());
                    if ui.text_edit_singleline(&mut value).changed() {
                        self.settings.bindings.insert(id, value);
                        dirty = true;
                    }
                });
            }
        });
        if dirty {
            self.hotkeys.update_bindings(self.settings.bindings.clone());
            self.settings.save();
        }
    }

    fn ui_hud(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let play = if self.playback.paused { "Play" } else { "Pause" };
                if ui.button(play).clicked() {
                    self.apply_action(Action::PlayPause);
                }
                if ui.button("−5s").clicked() {
                    self.apply_action(Action::SeekBack);
                }
                if ui.button("+5s").clicked() {
                    self.apply_action(Action::SeekFwd);
                }
                if ui.button("K").clicked() {
                    self.apply_action(Action::Keyframe);
                }
                if ui.button("Seq").clicked() {
                    self.apply_action(Action::PlaySeq);
                }
                let rec = if self.recording.recording { "Stop rec" } else { "Rec" };
                if ui.button(rec).clicked() {
                    self.apply_action(Action::RecordToggle);
                }
                if ui.button("Cinema").clicked() {
                    self.apply_action(Action::Cinematic);
                }
            });
            ui.label(RichText::new(crate::hotkeys::debug_snapshot()).small());
            if !self.last_hotkey.is_empty() {
                ui.label(RichText::new(&self.last_hotkey).small().color(Color32::from_rgb(80, 200, 140)));
            }
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} / {}   {}",
                    Playback::format_time(self.playback.time),
                    Playback::format_time(self.playback.length),
                    if self.recording.recording {
                        "REC"
                    } else if self.connected {
                        "live"
                    } else {
                        "idle"
                    }
                ));
            });
        });
    }

    fn ui_particles(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.connected, |ui| {
            ui.text_edit_singleline(&mut self.particle_filter);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let filter = self.particle_filter.to_lowercase();
                let names: Vec<String> = self
                    .particles
                    .keys()
                    .filter(|n| filter.is_empty() || n.to_lowercase().contains(&filter))
                    .cloned()
                    .collect();
                for name in names {
                    let mut on = *self.particles.get(&name).unwrap_or(&true);
                    if ui.checkbox(&mut on, &name).changed() {
                        let _ = self.client.set_particle(&name, on);
                    }
                }
            });
        });
    }
}

fn spawn_watch(tx: Sender<BgEvent>, path: PathBuf) {
    thread::spawn(move || {
        match lcu::Lcu::connect().and_then(|c| c.watch_rofl(&path)) {
            Ok(_) => {
                let _ = tx.send(BgEvent::Status(
                    "LCU accepted watch — waiting for game process + Replay API…".into(),
                ));
                let out = handshake::await_replay(Duration::from_secs(50));
                let _ = tx.send(BgEvent::WatchDone(out));
            }
            Err(e) => {
                let _ = tx.send(BgEvent::WatchDone(WatchOutcome::Timeout {
                    log: format!("LCU: {e}"),
                }));
            }
        }
    });
}

fn perm_row(ui: &mut Ui, label: &str, ok: bool, on_fix: impl FnOnce()) {
    ui.horizontal(|ui| {
        let c = if ok {
            Color32::from_rgb(80, 200, 140)
        } else {
            Color32::from_rgb(220, 120, 80)
        };
        ui.colored_label(c, if ok { "OK" } else { "NO" });
        ui.label(label);
        if !ok && ui.small_button("Fix").clicked() {
            on_fix();
        }
    });
}

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(36, 36, 40);
    visuals.window_fill = Color32::from_rgb(42, 42, 48);
    visuals.override_text_color = Some(Color32::from_rgb(210, 210, 214));
    style.visuals = visuals;
    ctx.set_style(style);
}

fn bool_row(ui: &mut Ui, label: &str, value: &mut bool, on_change: impl FnOnce(bool)) {
    if ui.checkbox(value, label).changed() {
        on_change(*value);
    }
}

fn float_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    on_change: impl FnOnce(f64),
) {
    if ui.add(Slider::new(value, range).text(label)).changed() {
        on_change(*value);
    }
}

fn vec3_row(ui: &mut Ui, label: &str, v: &mut Vec3, on_change: impl FnOnce(Vec3)) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut changed = false;
        changed |= ui.add(egui::DragValue::new(&mut v.x).speed(5.0).prefix("x ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut v.y).speed(5.0).prefix("y ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut v.z).speed(5.0).prefix("z ")).changed();
        if changed {
            on_change(*v);
        }
    });
}

fn color_row(ui: &mut Ui, label: &str, c: &mut Color, on_change: impl FnOnce(Color)) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut changed = false;
        changed |= ui
            .add(egui::DragValue::new(&mut c.r).speed(0.01).range(0.0..=1.0).prefix("r "))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut c.g).speed(0.01).range(0.0..=1.0).prefix("g "))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut c.b).speed(0.01).range(0.0..=1.0).prefix("b "))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut c.a).speed(0.01).range(0.0..=1.0).prefix("a "))
            .changed();
        if changed {
            on_change(*c);
        }
    });
}
