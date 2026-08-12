use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    #[serde(default)]
    pub process_id: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub seeking: bool,
    #[serde(default)]
    pub time: f64,
    #[serde(default)]
    pub speed: f64,
    #[serde(default)]
    pub length: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    #[serde(default)]
    pub recording: bool,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub codec: String,
    #[serde(default)]
    pub start_time: f64,
    #[serde(default)]
    pub end_time: f64,
    #[serde(default)]
    pub current_time: f64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub frames_per_second: f64,
    #[serde(default)]
    pub enforce_frame_rate: bool,
    #[serde(default)]
    pub replay_speed: f64,
    #[serde(default)]
    pub lossless: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Render {
    #[serde(default)]
    pub camera_mode: String,
    #[serde(default)]
    pub camera_position: Vec3,
    #[serde(default)]
    pub camera_rotation: Vec3,
    #[serde(default)]
    pub camera_attached: bool,
    #[serde(default)]
    pub camera_lock_x: bool,
    #[serde(default)]
    pub camera_lock_y: bool,
    #[serde(default)]
    pub camera_lock_z: bool,
    #[serde(default)]
    pub camera_move_speed: f64,
    #[serde(default)]
    pub camera_look_speed: f64,
    #[serde(default)]
    pub field_of_view: f64,
    #[serde(default)]
    pub near_clip: f64,
    #[serde(default)]
    pub far_clip: f64,
    #[serde(default)]
    pub fog_of_war: bool,
    #[serde(default)]
    pub outline_select: bool,
    #[serde(default)]
    pub outline_hover: bool,
    #[serde(default)]
    pub floating_text: bool,
    #[serde(default)]
    pub nav_grid_offset: f64,
    #[serde(default)]
    pub simulate_all_particles_while_off_screen: bool,
    #[serde(default)]
    pub interface_all: bool,
    #[serde(default)]
    pub interface_replay: bool,
    #[serde(default)]
    pub interface_score: bool,
    #[serde(default)]
    pub interface_scoreboard: bool,
    #[serde(default)]
    pub interface_frames: bool,
    #[serde(default)]
    pub interface_minimap: bool,
    #[serde(default)]
    pub interface_timeline: bool,
    #[serde(default)]
    pub interface_chat: bool,
    #[serde(default)]
    pub interface_target: bool,
    #[serde(default)]
    pub interface_quests: bool,
    #[serde(default)]
    pub interface_announce: bool,
    #[serde(default)]
    pub interface_kill_callouts: bool,
    #[serde(default)]
    pub interface_neutral_timers: bool,
    #[serde(default)]
    pub health_bar_champions: bool,
    #[serde(default)]
    pub health_bar_structures: bool,
    #[serde(default)]
    pub health_bar_wards: bool,
    #[serde(default)]
    pub health_bar_pets: bool,
    #[serde(default)]
    pub health_bar_minions: bool,
    #[serde(default)]
    pub environment: bool,
    #[serde(default)]
    pub characters: bool,
    #[serde(default)]
    pub champions: bool,
    #[serde(default)]
    pub minions: bool,
    #[serde(default)]
    pub particles: bool,
    #[serde(default)]
    pub banners: bool,
    #[serde(default)]
    pub skybox_path: String,
    #[serde(default)]
    pub skybox_rotation: f64,
    #[serde(default)]
    pub skybox_radius: f64,
    #[serde(default)]
    pub skybox_offset: f64,
    #[serde(default)]
    pub sun_direction: Vec3,
    #[serde(default)]
    pub depth_fog_enabled: bool,
    #[serde(default)]
    pub depth_fog_start: f64,
    #[serde(default)]
    pub depth_fog_end: f64,
    #[serde(default)]
    pub depth_fog_intensity: f64,
    #[serde(default)]
    pub depth_fog_color: Color,
    #[serde(default)]
    pub height_fog_enabled: bool,
    #[serde(default)]
    pub height_fog_start: f64,
    #[serde(default)]
    pub height_fog_end: f64,
    #[serde(default)]
    pub height_fog_intensity: f64,
    #[serde(default)]
    pub height_fog_color: Color,
    #[serde(default)]
    pub depth_of_field_enabled: bool,
    #[serde(default)]
    pub depth_of_field_debug: bool,
    #[serde(default)]
    pub depth_of_field_circle: f64,
    #[serde(default)]
    pub depth_of_field_width: f64,
    #[serde(default)]
    pub depth_of_field_near: f64,
    #[serde(default)]
    pub depth_of_field_mid: f64,
    #[serde(default)]
    pub depth_of_field_far: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub time: f64,
    pub value: T,
    #[serde(default = "default_blend")]
    pub blend: String,
}

fn default_blend() -> String {
    "linear".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    #[serde(default)]
    pub playback_speed: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub camera_position: Vec<Keyframe<Vec3>>,
    #[serde(default)]
    pub camera_rotation: Vec<Keyframe<Vec3>>,
    #[serde(default)]
    pub field_of_view: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub near_clip: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub far_clip: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub nav_grid_offset: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub skybox_rotation: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub skybox_radius: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub skybox_offset: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub sun_direction: Vec<Keyframe<Vec3>>,
    #[serde(default)]
    pub depth_fog_enabled: Vec<Keyframe<bool>>,
    #[serde(default)]
    pub depth_fog_start: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_fog_end: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_fog_intensity: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_fog_color: Vec<Keyframe<Color>>,
    #[serde(default)]
    pub height_fog_enabled: Vec<Keyframe<bool>>,
    #[serde(default)]
    pub height_fog_start: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub height_fog_end: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub height_fog_intensity: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub height_fog_color: Vec<Keyframe<Color>>,
    #[serde(default)]
    pub depth_of_field_enabled: Vec<Keyframe<bool>>,
    #[serde(default)]
    pub depth_of_field_circle: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_of_field_width: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_of_field_near: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_of_field_mid: Vec<Keyframe<f64>>,
    #[serde(default)]
    pub depth_of_field_far: Vec<Keyframe<f64>>,
}

impl Sequence {
    pub fn sort_all(&mut self) {
        sort_kfs(&mut self.playback_speed);
        sort_kfs(&mut self.camera_position);
        sort_kfs(&mut self.camera_rotation);
        sort_kfs(&mut self.field_of_view);
        sort_kfs(&mut self.near_clip);
        sort_kfs(&mut self.far_clip);
        sort_kfs(&mut self.nav_grid_offset);
        sort_kfs(&mut self.skybox_rotation);
        sort_kfs(&mut self.skybox_radius);
        sort_kfs(&mut self.skybox_offset);
        sort_kfs(&mut self.sun_direction);
        sort_kfs(&mut self.depth_fog_enabled);
        sort_kfs(&mut self.depth_fog_start);
        sort_kfs(&mut self.depth_fog_end);
        sort_kfs(&mut self.depth_fog_intensity);
        sort_kfs(&mut self.depth_fog_color);
        sort_kfs(&mut self.height_fog_enabled);
        sort_kfs(&mut self.height_fog_start);
        sort_kfs(&mut self.height_fog_end);
        sort_kfs(&mut self.height_fog_intensity);
        sort_kfs(&mut self.height_fog_color);
        sort_kfs(&mut self.depth_of_field_enabled);
        sort_kfs(&mut self.depth_of_field_circle);
        sort_kfs(&mut self.depth_of_field_width);
        sort_kfs(&mut self.depth_of_field_near);
        sort_kfs(&mut self.depth_of_field_mid);
        sort_kfs(&mut self.depth_of_field_far);
    }

    pub fn first_time(&self) -> Option<f64> {
        self.camera_position
            .first()
            .map(|k| k.time)
            .or_else(|| self.camera_rotation.first().map(|k| k.time))
            .or_else(|| self.field_of_view.first().map(|k| k.time))
            .or_else(|| self.playback_speed.first().map(|k| k.time))
    }

    pub fn last_time(&self) -> Option<f64> {
        self.camera_position
            .last()
            .map(|k| k.time)
            .into_iter()
            .chain(self.camera_rotation.last().map(|k| k.time))
            .chain(self.field_of_view.last().map(|k| k.time))
            .chain(self.playback_speed.last().map(|k| k.time))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

fn sort_kfs<T>(frames: &mut [Keyframe<T>]) {
    frames.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
}

pub fn upsert_kf<T: Clone>(frames: &mut Vec<Keyframe<T>>, time: f64, value: T, blend: &str) {
    if let Some(existing) = frames.iter_mut().find(|k| (k.time - time).abs() < 0.02) {
        existing.value = value;
        existing.blend = blend.to_string();
    } else {
        frames.push(Keyframe {
            time,
            value,
            blend: blend.to_string(),
        });
    }
    sort_kfs(frames);
}

pub type Particles = BTreeMap<String, bool>;

pub const BLENDS: &[&str] = &[
    "linear",
    "snap",
    "smoothStep",
    "smootherStep",
    "quadraticEaseInOut",
    "cubicEaseInOut",
    "sineEaseInOut",
];

impl Playback {
    pub fn format_time(t: f64) -> String {
        let t = t.max(0.0);
        let minutes = (t / 60.0).floor() as u32;
        let seconds = t % 60.0;
        format!("{minutes:02}:{seconds:05.2}")
    }
}
