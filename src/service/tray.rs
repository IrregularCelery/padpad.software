use tray_item::{IconSource, TrayItem};

use crate::config::{update_config_and_client, CONFIG};

pub fn handle_tray_thread() {
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

        let mut config = CONFIG
            .get()
            .expect("Could not retrieve CONFIG data!")
            .lock()
            .unwrap();

        config.load();

        // Reload client as well
        update_config_and_client(&mut config, |_| {});
    })
    .unwrap();

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
        std::thread::park();
    }
}
