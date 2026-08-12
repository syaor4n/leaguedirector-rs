use crate::api::{
    upsert_kf, Color, Playback, Particles, Recording, Render, ReplayClient, Sequence, Vec3,
};
use crate::detect::{self, GameInstall};
use crate::hotkeys::{Hotkey, HotkeyBus};
use crate::lcu;
use crate::media;
use crate::permissions;
use crate::sequence_ui::{self, TrackEvent};
use crate::settings::Settings;
use eframe::egui::{self, Color32, RichText, Slider, Ui};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

enum BgEvent {
    Status(String),
    RoflOpened(PathBuf),
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
}

impl DirectorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);
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
        Self {
            client,
            tab: settings.last_tab,
            rec_codec: "webm".into(),
            rec_fps: 30.0,
            rec_start: 0.0,
            rec_end: 10.0,
            particle_filter: String::new(),
            apply_sequence: false,
            settings,
            hotkeys: HotkeyBus::start(),
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
        }
    }

    fn refresh_lcu_line(&mut self) {
        if self.last_lcu_check.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.last_lcu_check = Instant::now();
        self.lcu_line = match lcu::Lcu::connect() {
            Ok(c) => match c.replay_path() {
                Some(p) => format!("LCU connected · {p}"),
                None => "LCU connected.".into(),
            },
            Err(_) => "LCU not found (open the League client to watch a replay).".into(),
        };
    }

    fn drain_bg(&mut self) {
        while let Ok(ev) = self.bg_rx.try_recv() {
            match ev {
                BgEvent::Status(s) => {
                    self.lcu_busy = false;
                    self.status = s;
                }
                BgEvent::RoflOpened(path) => {
                    self.lcu_busy = false;
                    self.status = format!("Replay requested: {}", path.display());
                    self.rofls = detect::list_rofls();
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
                if self.was_recording {
                    if let Some(path) =
                        media::finalize_recording(&self.recording.path, &self.settings.record_dir)
                    {
                        self.last_capture = Some(path.clone());
                        self.status = format!("API dropped, remuxed: {}", path.display());
                    }
                    self.was_recording = false;
                } else if !self.lcu_busy {
                    self.status = "Replay API idle — open a .rofl or start a replay.".into();
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

    fn apply_hotkey(&mut self, key: Hotkey) {
        if !self.connected {
            return;
        }
        match key {
            Hotkey::PlayPause => {
                self.post_playback(serde_json::json!({ "paused": !self.playback.paused }));
            }
            Hotkey::Keyframe => {
                self.add_camera_keyframes();
                self.tab = 4;
            }
            Hotkey::SeekBack => {
                self.post_playback(serde_json::json!({
                    "time": (self.playback.time - 5.0).max(0.0)
                }));
            }
            Hotkey::SeekFwd => {
                self.post_playback(serde_json::json!({
                    "time": (self.playback.time + 5.0).min(self.playback.length)
                }));
            }
            Hotkey::Undo => self.undo(),
            Hotkey::Redo => self.redo(),
            Hotkey::PlaySeq => {
                if let Some(start) = self.sequence.first_time() {
                    self.apply_sequence = true;
                    self.push_sequence();
                    self.post_playback(serde_json::json!({ "time": start, "paused": false }));
                }
            }
        }
    }

    fn handle_hotkeys(&mut self, ctx: &egui::Context) {
        if !ctx.wants_keyboard_input() {
            let space = ctx.input(|i| i.key_pressed(egui::Key::Space));
            let key_k = ctx.input(|i| i.key_pressed(egui::Key::K));
            let left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
            let right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
            let undo =
                ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z) && !i.modifiers.shift);
            let redo = ctx.input(|i| {
                (i.modifiers.command && i.key_pressed(egui::Key::Z) && i.modifiers.shift)
                    || (i.modifiers.command && i.key_pressed(egui::Key::Y))
            });
            let play_seq = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            if space {
                self.apply_hotkey(Hotkey::PlayPause);
            }
            if key_k {
                self.apply_hotkey(Hotkey::Keyframe);
            }
            if left {
                self.apply_hotkey(Hotkey::SeekBack);
            }
            if right {
                self.apply_hotkey(Hotkey::SeekFwd);
            }
            if undo {
                self.apply_hotkey(Hotkey::Undo);
            }
            if redo {
                self.apply_hotkey(Hotkey::Redo);
            }
            if play_seq {
                self.apply_hotkey(Hotkey::PlaySeq);
            }
        }
        while let Some(k) = self.hotkeys.try_recv() {
            self.apply_hotkey(k);
        }
    }

    fn open_rofl(&mut self, path: PathBuf) {
        self.lcu_busy = true;
        self.status = format!("Opening {}…", path.display());
        spawn_watch(self.bg_tx.clone(), path);
    }

    fn start_recording(&mut self, start: f64, end: f64) {
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
        thread::spawn(move || {
            if let Err(e) = client.set_recording(&body) {
                let _ = tx.send(BgEvent::Status(format!("Recording API: {e}")));
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Files & Folders").clicked() {
                        permissions::open_files_privacy();
                    }
                    if ui.button("Accessibility").clicked() {
                        permissions::open_privacy_settings();
                    }
                    if ui.button("Refresh").clicked() {
                        self.installs = detect::find_installs();
                        self.rofls = detect::list_rofls();
                        self.skyboxes = detect::list_skyboxes();
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
            _ => self.ui_particles(ui),
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

impl DirectorApp {
    fn ui_connect(&mut self, ui: &mut Ui) {
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
                    "Shortcuts: Space play/pause · ←/→ ±5s · K keyframe · ⌘Z undo · Enter play sequence. Also work in League (Accessibility + Input Monitoring).",
                )
                .small(),
            );
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
        ui.add_enabled_ui(self.connected && !self.recording.recording, |ui| {
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
            Ok(dest) => {
                let _ = tx.send(BgEvent::RoflOpened(dest));
            }
            Err(e) => {
                let _ = tx.send(BgEvent::Status(format!("LCU: {e}")));
            }
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
