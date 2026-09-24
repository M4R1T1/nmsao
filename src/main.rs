#![windows_subsystem = "windows"]

mod app;
mod assets;
mod config;
mod logging;
mod models;
mod soundbank;
mod tree;
mod ui;
mod utils;
mod wwise;

use app::MyApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let icon_bytes = include_bytes!("../assets/classmini.a.png");
    let icon = image::load_from_memory(icon_bytes)
        .unwrap()
        .to_rgba8();
    let icon_data = egui::IconData {
        rgba: icon.to_vec(),
        width: icon.width(),
        height: icon.height(),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1300.0, 850.0])
            .with_icon(std::sync::Arc::new(icon_data)),
        ..Default::default()
    };
    let app = MyApp::default();
    eframe::run_native(
        "NMSAO - No Man's Sky Audio Organizer",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            Ok(Box::new(app))
        }),
    )
}