use crate::api::{Color, Keyframe, Sequence, Vec3, BLENDS};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Ui};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrackId {
    Position,
    Rotation,
    Fov,
    Speed,
    Fog,
    Dof,
}

pub enum TrackEvent {
    Seek(f64),
    Move {
        track: TrackId,
        index: usize,
        time: f64,
    },
}

pub fn apply_move(sequence: &mut Sequence, track: TrackId, index: usize, time: f64) {
    match track {
        TrackId::Position => set_time(&mut sequence.camera_position, index, time),
        TrackId::Rotation => set_time(&mut sequence.camera_rotation, index, time),
        TrackId::Fov => set_time(&mut sequence.field_of_view, index, time),
        TrackId::Speed => set_time(&mut sequence.playback_speed, index, time),
        TrackId::Fog => set_time(&mut sequence.depth_fog_enabled, index, time),
        TrackId::Dof => set_time(&mut sequence.depth_of_field_enabled, index, time),
    }
}

fn set_time<T>(frames: &mut [Keyframe<T>], index: usize, time: f64) {
    if let Some(kf) = frames.get_mut(index) {
        kf.time = time.max(0.0);
    }
}

pub fn draw_tracks(ui: &mut Ui, sequence: &Sequence, playhead: f64, length: f64) -> Option<TrackEvent> {
    let length = length.max(1.0);
    let tracks: [(&str, TrackId, usize, Color32); 6] = [
        (
            "pos",
            TrackId::Position,
            sequence.camera_position.len(),
            Color32::from_rgb(120, 180, 255),
        ),
        (
            "rot",
            TrackId::Rotation,
            sequence.camera_rotation.len(),
            Color32::from_rgb(255, 170, 90),
        ),
        (
            "fov",
            TrackId::Fov,
            sequence.field_of_view.len(),
            Color32::from_rgb(160, 220, 140),
        ),
        (
            "spd",
            TrackId::Speed,
            sequence.playback_speed.len(),
            Color32::from_rgb(220, 140, 220),
        ),
        (
            "fog",
            TrackId::Fog,
            sequence.depth_fog_enabled.len(),
            Color32::from_rgb(140, 200, 210),
        ),
        (
            "dof",
            TrackId::Dof,
            sequence.depth_of_field_enabled.len(),
            Color32::from_rgb(230, 200, 120),
        ),
    ];
    let row_h = 22.0;
    let height = 12.0 + tracks.len() as f32 * row_h;
    let (rect, bg) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(28, 28, 32));

    let mut event = None;
    for (i, (name, id, _n, color)) in tracks.iter().enumerate() {
        let y0 = rect.top() + 6.0 + i as f32 * row_h;
        let y1 = y0 + row_h - 4.0;
        let row = Rect::from_min_max(
            egui::pos2(rect.left() + 40.0, y0),
            egui::pos2(rect.right() - 8.0, y1),
        );
        painter.rect_filled(row, 2.0, Color32::from_rgb(40, 40, 46));
        painter.text(
            egui::pos2(rect.left() + 6.0, (y0 + y1) * 0.5),
            egui::Align2::LEFT_CENTER,
            *name,
            egui::FontId::proportional(11.0),
            Color32::GRAY,
        );

        let times: Vec<f64> = match id {
            TrackId::Position => sequence.camera_position.iter().map(|k| k.time).collect(),
            TrackId::Rotation => sequence.camera_rotation.iter().map(|k| k.time).collect(),
            TrackId::Fov => sequence.field_of_view.iter().map(|k| k.time).collect(),
            TrackId::Speed => sequence.playback_speed.iter().map(|k| k.time).collect(),
            TrackId::Fog => sequence.depth_fog_enabled.iter().map(|k| k.time).collect(),
            TrackId::Dof => sequence.depth_of_field_enabled.iter().map(|k| k.time).collect(),
        };

        for (ki, t) in times.iter().enumerate() {
            let x = row.left() + (*t / length).clamp(0.0, 1.0) as f32 * row.width();
            let center = Pos2::new(x, row.center().y);
            painter.circle_filled(center, 5.0, *color);
            let hit = Rect::from_center_size(center, egui::vec2(14.0, 14.0));
            let resp = ui.interact(hit, ui.id().with(("kf", *name, ki)), Sense::click_and_drag());
            if resp.dragged() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let nt = ((pos.x - row.left()) / row.width()).clamp(0.0, 1.0) as f64 * length;
                    event = Some(TrackEvent::Move {
                        track: *id,
                        index: ki,
                        time: nt,
                    });
                }
            } else if resp.clicked() {
                event = Some(TrackEvent::Seek(*t));
            }
            if resp.hovered() {
                painter.circle_stroke(center, 6.5, egui::Stroke::new(1.0_f32, Color32::WHITE));
            }
        }

        let row_resp = ui.interact(row, ui.id().with(("row", *name)), Sense::click());
        if event.is_none() && row_resp.clicked() {
            if let Some(pos) = row_resp.interact_pointer_pos() {
                let nt = ((pos.x - row.left()) / row.width()).clamp(0.0, 1.0) as f64 * length;
                event = Some(TrackEvent::Seek(nt));
            }
        }
    }

    let x = rect.left() + 40.0 + (playhead / length).clamp(0.0, 1.0) as f32 * (rect.width() - 48.0);
    painter.line_segment(
        [egui::pos2(x, rect.top() + 4.0), egui::pos2(x, rect.bottom() - 4.0)],
        egui::Stroke::new(1.5_f32, Color32::from_rgb(230, 80, 80)),
    );

    if event.is_none() && bg.clicked() {
        if let Some(pos) = bg.interact_pointer_pos() {
            let left = rect.left() + 40.0;
            let width = (rect.width() - 48.0).max(1.0);
            let nt = ((pos.x - left) / width).clamp(0.0, 1.0) as f64 * length;
            event = Some(TrackEvent::Seek(nt));
        }
    }
    event
}

pub fn keyframe_table_vec3(ui: &mut Ui, title: &str, frames: &mut Vec<Keyframe<Vec3>>) -> bool {
    let mut dirty = false;
    ui.collapsing(title, |ui| {
        let mut remove = None;
        for (i, kf) in frames.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("#{i}"));
                dirty |= ui.add(egui::DragValue::new(&mut kf.time).speed(0.05).prefix("t ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.x).speed(5.0).prefix("x ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.y).speed(5.0).prefix("y ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.z).speed(5.0).prefix("z ")).changed();
                dirty |= blend_combo(ui, title, i, &mut kf.blend);
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                    dirty = true;
                }
            });
        }
        if let Some(i) = remove {
            frames.remove(i);
        }
    });
    dirty
}

pub fn keyframe_table_f64(ui: &mut Ui, title: &str, frames: &mut Vec<Keyframe<f64>>) -> bool {
    let mut dirty = false;
    ui.collapsing(title, |ui| {
        let mut remove = None;
        for (i, kf) in frames.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("#{i}"));
                dirty |= ui.add(egui::DragValue::new(&mut kf.time).speed(0.05).prefix("t ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value).speed(0.1).prefix("v ")).changed();
                dirty |= blend_combo(ui, title, i, &mut kf.blend);
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                    dirty = true;
                }
            });
        }
        if let Some(i) = remove {
            frames.remove(i);
        }
    });
    dirty
}

pub fn keyframe_table_bool(ui: &mut Ui, title: &str, frames: &mut Vec<Keyframe<bool>>) -> bool {
    let mut dirty = false;
    ui.collapsing(title, |ui| {
        let mut remove = None;
        for (i, kf) in frames.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("#{i}"));
                dirty |= ui.add(egui::DragValue::new(&mut kf.time).speed(0.05).prefix("t ")).changed();
                if ui.checkbox(&mut kf.value, "on").changed() {
                    dirty = true;
                }
                dirty |= blend_combo(ui, title, i, &mut kf.blend);
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                    dirty = true;
                }
            });
        }
        if let Some(i) = remove {
            frames.remove(i);
        }
    });
    dirty
}

pub fn keyframe_table_color(ui: &mut Ui, title: &str, frames: &mut Vec<Keyframe<Color>>) -> bool {
    let mut dirty = false;
    ui.collapsing(title, |ui| {
        let mut remove = None;
        for (i, kf) in frames.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("#{i}"));
                dirty |= ui.add(egui::DragValue::new(&mut kf.time).speed(0.05).prefix("t ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.r).speed(0.01).range(0.0..=1.0).prefix("r ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.g).speed(0.01).range(0.0..=1.0).prefix("g ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.b).speed(0.01).range(0.0..=1.0).prefix("b ")).changed();
                dirty |= ui.add(egui::DragValue::new(&mut kf.value.a).speed(0.01).range(0.0..=1.0).prefix("a ")).changed();
                dirty |= blend_combo(ui, title, i, &mut kf.blend);
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                    dirty = true;
                }
            });
        }
        if let Some(i) = remove {
            frames.remove(i);
        }
    });
    dirty
}

fn blend_combo(ui: &mut Ui, title: &str, i: usize, blend: &mut String) -> bool {
    let mut dirty = false;
    egui::ComboBox::from_id_salt(format!("{title}-blend-{i}"))
        .selected_text(blend.as_str())
        .show_ui(ui, |ui| {
            for b in BLENDS {
                if ui.selectable_value(blend, (*b).to_string(), *b).changed() {
                    dirty = true;
                }
            }
        });
    dirty
}
