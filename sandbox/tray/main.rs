#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use tray_item::{IconSource, TrayItem};

fn main() {
    // ---------- Tray ---------- //
    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        gtk::init().unwrap();

        let mut tray = TrayItem::new("Tray Example", IconSource::Resource("app-icon")).unwrap();

        tray.add_label("Tray Label").unwrap();

        tray.add_menu_item("Dashboard", move || {
            println!("Opening dashboard...");
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

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
