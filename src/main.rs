#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tray_item::{IconSource, TrayItem};

fn main() {
    // ---------- Tray ---------- //
    let tray_thread = std::thread::spawn(|| {
        #[cfg(target_os = "linux")]
        gtk::init().unwrap();

        let mut tray = TrayItem::new("PadPad", IconSource::Resource("app-icon")).unwrap();

        tray.add_menu_item("Dashboard", || {
            println!("Opening dashboard...");
        })
        .unwrap();

        tray.inner_mut().add_separator().unwrap();

        tray.add_menu_item("Reload", || {
            println!("Reloading config file...");
        })
        .unwrap();

        tray.inner_mut().add_separator().unwrap();

        //tray.add_label("Status").unwrap();
        //tray.add_label("- Not Contected").unwrap();
        //tray.add_label("- Baud rate: 0000000").unwrap();
        //
        //tray.add_label("- twetwtiuetowietwu").unwrap();

        tray.inner_mut().add_separator().unwrap();

        tray.add_menu_item("Debug", || {
            println!("Debug");
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

    tray_thread
        .join()
        .expect_err("there was a problem while spawning the `read` thread!");
}
