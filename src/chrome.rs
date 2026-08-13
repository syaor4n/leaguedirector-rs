use crate::api::Playback;
use eframe::egui::{
    self, Color32, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
pub const PANEL: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x20);
pub const RAIL: Color32 = Color32::from_rgb(0x18, 0x18, 0x1B);
pub const LIVE: Color32 = Color32::from_rgb(0x3D, 0xCF, 0x8E);
pub const REC: Color32 = Color32::from_rgb(0xE2, 0x4B, 0x4A);
pub const AMBER: Color32 = Color32::from_rgb(0xE8, 0xA2, 0x3A);
pub const TEXT: Color32 = Color32::from_rgb(0xE8, 0xE8, 0xEC);
pub const MUTE: Color32 = Color32::from_rgb(0x7A, 0x7A, 0x82);
pub const LINE: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x30);

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(title.to_ascii_uppercase())
            .size(10.0)
            .color(MUTE)
            .extra_letter_spacing(1.4),
    );
    ui.add_space(4.0);
}

pub fn lamp(ui: &mut Ui, on: bool, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            3.5,
            if on { LIVE } else { Color32::from_rgb(0x4A, 0x4A, 0x50) },
        );
        ui.label(RichText::new(label).size(12.0).color(if on { TEXT } else { MUTE }));
    });
}

pub fn group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    ui.add_space(8.0);
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .inner_margin(Margin::symmetric(12, 10))
        .corner_radius(3.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            section(ui, title);
            add(ui);
        });
}

pub fn preset_btn(ui: &mut Ui, label: &str, primary: bool) -> bool {
    let fill = if primary {
        Color32::from_rgb(0x2A, 0x24, 0x18)
    } else {
        Color32::from_rgb(0x24, 0x24, 0x28)
    };
    let ink = if primary { AMBER } else { TEXT };
    ui.add(
        egui::Button::new(RichText::new(label).size(13.0).color(ink))
            .fill(fill)
            .min_size(Vec2::new(108.0, 34.0)),
    )
    .clicked()
}

/// Large FOV control. Returns (changed, drag_started).
pub fn fov_strip(ui: &mut Ui, fov: &mut f64) -> (bool, bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("FOV")
                .size(11.0)
                .color(MUTE)
                .extra_letter_spacing(1.2),
        );
        ui.label(
            RichText::new(format!("{fov:.0}°"))
                .font(FontId::monospace(22.0))
                .color(TEXT),
        );
        ui.add_space(8.0);
        if ui
            .add(egui::Button::new(RichText::new("Tight").size(11.0).color(MUTE)).fill(RAIL))
            .clicked()
        {
            *fov = 28.0;
            return (true, true);
        }
        if ui
            .add(egui::Button::new(RichText::new("Wide").size(11.0).color(MUTE)).fill(RAIL))
            .clicked()
        {
            *fov = 70.0;
            return (true, true);
        }
        (false, false)
    })
    .inner
}

pub fn desk_switcher(ui: &mut Ui, current: usize) -> Option<usize> {
    let desks = ["CONNECT", "LOOK", "CUT", "CAPTURE"];
    let mut picked = None;
    let height = 28.0;
    let gap = 2.0;
    let width = ui.available_width();
    let cell = ((width - gap * 3.0) / 4.0).max(72.0);
    let (bar, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(bar);
    painter.rect_filled(bar, 2.0, RAIL);
    for (i, name) in desks.iter().enumerate() {
        let x = bar.left() + i as f32 * (cell + gap);
        let r = Rect::from_min_size(Pos2::new(x, bar.top()), Vec2::new(cell, height));
        let id = ui.id().with(("desk", i));
        let resp = ui.interact(r, id, Sense::click());
        let on = current == i || resp.hovered();
        if on {
            painter.rect_filled(
                r.shrink(1.0),
                2.0,
                if current == i {
                    Color32::from_rgb(0x2A, 0x24, 0x18)
                } else {
                    Color32::from_rgb(0x22, 0x22, 0x26)
                },
            );
        }
        if current == i {
            painter.line_segment(
                [Pos2::new(r.left() + 8.0, r.bottom() - 2.0), Pos2::new(r.right() - 8.0, r.bottom() - 2.0)],
                Stroke::new(1.5_f32, AMBER),
            );
        }
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            *name,
            FontId::proportional(11.0),
            if current == i { AMBER } else { MUTE },
        );
        if resp.clicked() {
            picked = Some(i);
        }
    }
    picked
}

pub struct DeckIn<'a> {
    pub playback: &'a Playback,
    pub recording: bool,
    pub rec_blocked: bool,
    pub clip_in: Option<f64>,
    pub clip_out: Option<f64>,
    pub compact: bool,
}

pub struct TransportOut {
    pub play: bool,
    pub back: bool,
    pub fwd: bool,
    pub key: bool,
    pub seq: bool,
    pub rec: bool,
    pub mark_in: bool,
    pub mark_out: bool,
    pub clear_marks: bool,
    pub seek: Option<f64>,
    pub undo: bool,
    pub reset: bool,
}

impl Default for TransportOut {
    fn default() -> Self {
        Self {
            play: false,
            back: false,
            fwd: false,
            key: false,
            seq: false,
            rec: false,
            mark_in: false,
            mark_out: false,
            clear_marks: false,
            seek: None,
            undo: false,
            reset: false,
        }
    }
}

pub fn transport_bar(ui: &mut Ui, deck: DeckIn<'_>) -> TransportOut {
    let mut out = TransportOut::default();
    let h = if deck.compact { 62.0 } else { 58.0 };
    let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let painter = ui.painter_at(bar);
    painter.rect_filled(bar, 0.0, Color32::from_rgb(0x12, 0x12, 0x14));
    painter.line_segment(
        [Pos2::new(bar.left(), bar.top()), Pos2::new(bar.right(), bar.top())],
        Stroke::new(1.0_f32, LINE),
    );

    let mut x = bar.left() + 10.0;
    let mid_y = bar.center().y - 6.0;
    let btn_w = if deck.compact { 40.0 } else { 44.0 };
    let btn = |ui: &Ui, x: &mut f32, label: &str, fill: Color32, ink: Color32, w: f32| {
        let r = Rect::from_center_size(Pos2::new(*x + w * 0.5, mid_y), Vec2::new(w, 28.0));
        *x += w + 6.0;
        let resp = ui.interact(r, ui.id().with(("tr", label)), Sense::click());
        let bg = if resp.hovered() {
            Color32::from_rgb(0x32, 0x32, 0x38)
        } else {
            fill
        };
        ui.painter().rect_filled(r, 2.0, bg);
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            ink,
        );
        resp.clicked()
    };

    let play_lab = if deck.playback.paused { "PLAY" } else { "PAUSE" };
    if btn(ui, &mut x, play_lab, Color32::from_rgb(0x24, 0x24, 0x28), TEXT, btn_w + 8.0) {
        out.play = true;
    }
    if btn(ui, &mut x, "−5", Color32::from_rgb(0x24, 0x24, 0x28), TEXT, btn_w) {
        out.back = true;
    }
    if btn(ui, &mut x, "+5", Color32::from_rgb(0x24, 0x24, 0x28), TEXT, btn_w) {
        out.fwd = true;
    }
    if btn(ui, &mut x, "K", Color32::from_rgb(0x2A, 0x24, 0x18), AMBER, btn_w) {
        out.key = true;
    }
    if btn(ui, &mut x, "SEQ", Color32::from_rgb(0x24, 0x24, 0x28), TEXT, btn_w) {
        out.seq = true;
    }
    if !deck.compact {
        let in_fill = if deck.clip_in.is_some() {
            Color32::from_rgb(0x18, 0x28, 0x20)
        } else {
            Color32::from_rgb(0x24, 0x24, 0x28)
        };
        if btn(ui, &mut x, "IN", in_fill, if deck.clip_in.is_some() { LIVE } else { TEXT }, btn_w) {
            out.mark_in = true;
        }
        let out_fill = if deck.clip_out.is_some() {
            Color32::from_rgb(0x28, 0x18, 0x18)
        } else {
            Color32::from_rgb(0x24, 0x24, 0x28)
        };
        if btn(
            ui,
            &mut x,
            "OUT",
            out_fill,
            if deck.clip_out.is_some() { REC } else { TEXT },
            btn_w,
        ) {
            out.mark_out = true;
        }
        if (deck.clip_in.is_some() || deck.clip_out.is_some())
            && btn(ui, &mut x, "CLR", Color32::from_rgb(0x24, 0x24, 0x28), MUTE, 36.0)
        {
            out.clear_marks = true;
        }
    }
    let rec_fill = if deck.recording {
        REC
    } else if deck.rec_blocked {
        Color32::from_rgb(0x3A, 0x28, 0x18)
    } else {
        Color32::from_rgb(0x28, 0x18, 0x18)
    };
    let rec_ink = if deck.recording {
        TEXT
    } else if deck.rec_blocked {
        AMBER
    } else {
        REC
    };
    let rec_lab = if deck.recording {
        "STOP"
    } else if deck.rec_blocked {
        "WAIT"
    } else {
        "REC"
    };
    if btn(ui, &mut x, rec_lab, rec_fill, rec_ink, btn_w + 4.0) {
        out.rec = true;
    }
    if deck.compact {
        if btn(ui, &mut x, "UNDO", Color32::from_rgb(0x24, 0x24, 0x28), TEXT, 48.0) {
            out.undo = true;
        }
        if btn(ui, &mut x, "RST", Color32::from_rgb(0x24, 0x24, 0x28), MUTE, 40.0) {
            out.reset = true;
        }
    }

    let tc = format!(
        "{}   /   {}",
        Playback::format_time(deck.playback.time),
        Playback::format_time(deck.playback.length)
    );
    painter.text(
        Pos2::new(bar.right() - 16.0, mid_y - 2.0),
        egui::Align2::RIGHT_CENTER,
        tc,
        FontId::monospace(if deck.compact { 16.0 } else { 18.0 }),
        TEXT,
    );
    let mark = match (deck.clip_in, deck.clip_out) {
        (Some(a), Some(b)) => format!("IN {}  OUT {}", Playback::format_time(a), Playback::format_time(b)),
        (Some(a), None) => format!("IN {}  ·  Rec +8s", Playback::format_time(a)),
        (None, Some(b)) => format!("OUT {}  ·  Rec −8s", Playback::format_time(b)),
        _ => "Rec playhead +8s".into(),
    };
    painter.text(
        Pos2::new(bar.right() - 16.0, mid_y + 14.0),
        egui::Align2::RIGHT_CENTER,
        mark,
        FontId::proportional(10.0),
        MUTE,
    );

    let scrub = Rect::from_min_max(
        Pos2::new(bar.left() + 12.0, bar.bottom() - 10.0),
        Pos2::new(bar.right() - 12.0, bar.bottom() - 5.0),
    );
    painter.rect_filled(scrub, 1.0, Color32::from_rgb(0x2A, 0x2A, 0x30));
    let len = deck.playback.length.max(0.1);
    let x_of = |t: f64| scrub.left() + (t / len).clamp(0.0, 1.0) as f32 * scrub.width();
    if let (Some(a), Some(b)) = (deck.clip_in, deck.clip_out) {
        let (lo, hi) = if b >= a { (a, b) } else { (b, a) };
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(x_of(lo), scrub.top()), Pos2::new(x_of(hi), scrub.bottom())),
            1.0,
            Color32::from_rgba_unmultiplied(0xE2, 0x4B, 0x4A, 70),
        );
    }
    if let Some(a) = deck.clip_in {
        let x = x_of(a);
        painter.line_segment(
            [Pos2::new(x, scrub.top() - 2.0), Pos2::new(x, scrub.bottom() + 2.0)],
            Stroke::new(1.5_f32, LIVE),
        );
    }
    if let Some(b) = deck.clip_out {
        let x = x_of(b);
        painter.line_segment(
            [Pos2::new(x, scrub.top() - 2.0), Pos2::new(x, scrub.bottom() + 2.0)],
            Stroke::new(1.5_f32, REC),
        );
    }
    let t = (deck.playback.time / len).clamp(0.0, 1.0) as f32;
    let head = scrub.left() + t * scrub.width();
    painter.rect_filled(
        Rect::from_min_max(scrub.min, Pos2::new(head, scrub.bottom())),
        1.0,
        AMBER,
    );
    let scrub_id = ui.interact(scrub.expand(4.0), ui.id().with("scrub"), Sense::click_and_drag());
    if (scrub_id.clicked() || scrub_id.dragged()) && deck.playback.length > 0.1 {
        if let Some(pos) = scrub_id.interact_pointer_pos() {
            let nt = ((pos.x - scrub.left()) / scrub.width()).clamp(0.0, 1.0) as f64 * deck.playback.length;
            out.seek = Some(nt);
        }
    }
    out
}

pub fn replay_row(ui: &mut Ui, name: &str, parent: &str, selected: bool) -> (bool, bool) {
    let h = 36.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::click());
    let painter = ui.painter_at(rect);
    if selected || resp.hovered() {
        painter.rect_filled(
            rect,
            2.0,
            if selected {
                Color32::from_rgb(0x2A, 0x24, 0x18)
            } else {
                Color32::from_rgb(0x22, 0x22, 0x26)
            },
        );
    }
    if selected {
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
            0.0,
            AMBER,
        );
    }
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.center().y - 6.0),
        egui::Align2::LEFT_CENTER,
        name,
        FontId::proportional(13.0),
        TEXT,
    );
    painter.text(
        Pos2::new(rect.left() + 12.0, rect.center().y + 8.0),
        egui::Align2::LEFT_CENTER,
        parent,
        FontId::proportional(10.0),
        MUTE,
    );
    let watch = Rect::from_center_size(
        Pos2::new(rect.right() - 36.0, rect.center().y),
        Vec2::new(56.0, 22.0),
    );
    let w = ui.interact(watch, ui.id().with(("w", name)), Sense::click());
    painter.rect_stroke(
        watch,
        2.0,
        Stroke::new(1.0_f32, if w.hovered() { AMBER } else { LINE }),
        StrokeKind::Middle,
    );
    painter.text(
        watch.center(),
        egui::Align2::CENTER_CENTER,
        "WATCH",
        FontId::proportional(10.0),
        if w.hovered() { AMBER } else { MUTE },
    );
    (resp.clicked() && !w.clicked(), w.clicked())
}
