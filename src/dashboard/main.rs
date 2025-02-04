#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

mod application;

use application::Application;
use padpad_software::constants::{APP_MIN_HEIGHT, APP_MIN_WIDTH};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let app = Application::default();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([APP_MIN_WIDTH as f32, APP_MIN_HEIGHT as f32])
            .with_min_inner_size([APP_MIN_WIDTH as f32, APP_MIN_HEIGHT as f32])
            .with_clamp_size_to_monitor_size(false)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_window_type(egui::X11WindowType::Dialog),
        ..Default::default()
    };

    eframe::run_native(
        format!(
            "{}PadPad",
            if cfg!(debug_assertions) {
                "[DEBUG] "
            } else {
                ""
            }
        )
        .as_str(),
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
