use crate::api::{Color, Render, Vec3};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Visual grade only — camera pose stays on the Cut desk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLook {
    pub name: String,
    #[serde(default)]
    pub field_of_view: f64,
    #[serde(default)]
    pub camera_mode: String,
    #[serde(default)]
    pub camera_move_speed: f64,
    #[serde(default)]
    pub camera_look_speed: f64,
    #[serde(default)]
    pub near_clip: f64,
    #[serde(default)]
    pub far_clip: f64,
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

impl SavedLook {
    pub fn from_render(name: &str, r: &Render) -> Self {
        Self {
            name: name.to_string(),
            field_of_view: r.field_of_view,
            camera_mode: r.camera_mode.clone(),
            camera_move_speed: r.camera_move_speed,
            camera_look_speed: r.camera_look_speed,
            near_clip: r.near_clip,
            far_clip: r.far_clip,
            skybox_path: r.skybox_path.clone(),
            skybox_rotation: r.skybox_rotation,
            skybox_radius: r.skybox_radius,
            skybox_offset: r.skybox_offset,
            sun_direction: r.sun_direction,
            depth_fog_enabled: r.depth_fog_enabled,
            depth_fog_start: r.depth_fog_start,
            depth_fog_end: r.depth_fog_end,
            depth_fog_intensity: r.depth_fog_intensity,
            depth_fog_color: r.depth_fog_color,
            height_fog_enabled: r.height_fog_enabled,
            height_fog_start: r.height_fog_start,
            height_fog_end: r.height_fog_end,
            height_fog_intensity: r.height_fog_intensity,
            height_fog_color: r.height_fog_color,
            depth_of_field_enabled: r.depth_of_field_enabled,
            depth_of_field_circle: r.depth_of_field_circle,
            depth_of_field_width: r.depth_of_field_width,
            depth_of_field_near: r.depth_of_field_near,
            depth_of_field_mid: r.depth_of_field_mid,
            depth_of_field_far: r.depth_of_field_far,
        }
    }

    pub fn to_patch(&self) -> serde_json::Value {
        json!({
            "fieldOfView": self.field_of_view,
            "cameraMode": self.camera_mode,
            "cameraMoveSpeed": self.camera_move_speed,
            "cameraLookSpeed": self.camera_look_speed,
            "nearClip": self.near_clip,
            "farClip": self.far_clip,
            "skyboxPath": self.skybox_path,
            "skyboxRotation": self.skybox_rotation,
            "skyboxRadius": self.skybox_radius,
            "skyboxOffset": self.skybox_offset,
            "sunDirection": self.sun_direction,
            "depthFogEnabled": self.depth_fog_enabled,
            "depthFogStart": self.depth_fog_start,
            "depthFogEnd": self.depth_fog_end,
            "depthFogIntensity": self.depth_fog_intensity,
            "depthFogColor": self.depth_fog_color,
            "heightFogEnabled": self.height_fog_enabled,
            "heightFogStart": self.height_fog_start,
            "heightFogEnd": self.height_fog_end,
            "heightFogIntensity": self.height_fog_intensity,
            "heightFogColor": self.height_fog_color,
            "depthOfFieldEnabled": self.depth_of_field_enabled,
            "depthOfFieldCircle": self.depth_of_field_circle,
            "depthOfFieldWidth": self.depth_of_field_width,
            "depthOfFieldNear": self.depth_of_field_near,
            "depthOfFieldMid": self.depth_of_field_mid,
            "depthOfFieldFar": self.depth_of_field_far,
        })
    }
}

pub fn dir() -> PathBuf {
    crate::settings::config_dir().join("looks")
}

pub fn list() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(rd) = fs::read_dir(dir()) else {
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

pub fn save(look: &SavedLook) -> Result<PathBuf, String> {
    let folder = dir();
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let stem = sanitize(&look.name);
    if stem.is_empty() {
        return Err("name required".into());
    }
    let path = folder.join(format!("{stem}.json"));
    let json = serde_json::to_string_pretty(look).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn load(path: &Path) -> Result<SavedLook, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn delete(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
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
