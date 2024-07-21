#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use padpad_software as lib;

use eframe::egui;
use lib::application::Application;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([640.0, 360.0])
            .with_clamp_size_to_monitor_size(false)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_window_type(egui::X11WindowType::Dialog),
        ..Default::default()
    };

    eframe::run_native(
        "PadPad",
        options,
        Box::new(|_cc| Ok(Box::<Application>::default())),
    )
}
