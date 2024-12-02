#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use eframe::egui;
use tray_item::{IconSource, TrayItem};

fn main() {
    env_logger::init();

    let is_done = Arc::new(AtomicBool::new(false));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    // ---------- Tray ---------- //
    let is_done_clone = Arc::clone(&is_done);

    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        gtk::init().unwrap();

        let mut tray = TrayItem::new("Tray Example", IconSource::Resource("app-icon")).unwrap();

        tray.add_label("Tray Label").unwrap();

        tray.add_menu_item("Dashboard", move || {
            println!("Opening dashboard...");

            is_done_clone.store(true, Ordering::Relaxed); // Set the boolean to true

            //if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
            //    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            //}
        })
        .unwrap();

        tray.inner_mut().add_separator().unwrap();

        tray.add_menu_item("Test", || {
            println!("Test!");
        })
        .unwrap();

        tray.inner_mut().add_separator().unwrap();

        tray.add_menu_item("Quit", || {
            println!("Quit!");

            std::process::exit(0);
        })
        .unwrap();

        #[cfg(target_os = "linux")]
        gtk::main();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
    // ---------- End Tray ---------- //

    let _ = eframe::run_native(
        "Sandbox: Tray",
        options.clone(),
        Box::new(|_cc| Ok(Box::<TrayApp>::default())),
    );

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));

        if is_done.load(Ordering::Relaxed) {
            let _ = eframe::run_native(
                "Sandbox: Tray",
                options.clone(),
                Box::new(|_cc| Ok(Box::<TrayApp>::default())),
            );

            is_done.store(false, Ordering::Relaxed);
        }
    }
}

//lazy_static::lazy_static! {
//    static ref EGUI_CTX: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));
//}

struct TrayApp {}

impl Default for TrayApp {
    fn default() -> Self {
        Self {}
    }
}

impl eframe::App for TrayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //let mut global_ctx = EGUI_CTX.lock().unwrap();
        //
        //if global_ctx.is_none() {
        //    *global_ctx = Some(ctx.clone());
        //}

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");

            if ui.button("close").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                // First this was gonna be the way to handle opening and closing the dashboard app
                // but WINDOWS!!! is dumb and works NOT in a similar way to other OSs (to be fair I
                // only tested linux, LOL) but anyways... and instead of closing/hiding the window,
                // it crashes the window until the next one is showing. This could also be because
                // of winit or egui. so for now, I thought of something else which might be (is)
                // even better.
                // I'm going to create two separate apps, one as a service which handles the
                // serial communication and tray management, and one for the dashboard gui.
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        });
    }
}
