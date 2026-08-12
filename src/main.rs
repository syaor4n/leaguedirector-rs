mod api;
mod app;
mod bindings;
mod detect;
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

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("League Director"),
        ..Default::default()
    };
    eframe::run_native(
        "League Director",
        options,
        Box::new(|cc| Ok(Box::new(DirectorApp::new(cc)))),
    )
}
