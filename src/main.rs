mod api;
mod app;
mod bindings;
mod chrome;
mod detect;
mod edits;
mod handshake;
mod hotkeys;
mod lcu;
mod media;
mod permissions;
mod presets;
mod seq_lib;
mod sequence_ui;
mod settings;

use app::DirectorApp;
use eframe::egui;

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(bytes)
        .expect("icon.png")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("League Director")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "League Director",
        options,
        Box::new(|cc| Ok(Box::new(DirectorApp::new(cc)))),
    )
}
