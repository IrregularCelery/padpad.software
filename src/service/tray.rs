use tray_item::{IconSource, TrayItem};

use crate::{
    config::{update_config_and_client, CONFIG},
    log_error, log_info,
    utility::{get_app_directory, restart},
};

pub fn handle_tray_thread() {
    #[cfg(target_os = "linux")]
    gtk::init().unwrap();

    let mut tray = TrayItem::new("PadPad", IconSource::Resource("app-icon")).unwrap();

    tray.add_menu_item("Dashboard", || {
        log_info!("Opening dashboard...");

        if let Ok(app_dir) = get_app_directory() {
            let dashboard_path = std::path::Path::new(&app_dir).join("dashboard");

            let result = std::process::Command::new(&dashboard_path).spawn();
            if let Err(e) = result {
                log_error!("{}", e);
            }
        }
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    tray.add_menu_item("Reload", || {
        log_info!("Reloading config file...");

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

    tray.add_menu_item("Restart", || {
        log_info!("Restarting the app...");

        restart()
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    tray.add_menu_item("Quit", || {
        log_info!("Closing the app...");

        std::process::exit(0);
    })
    .unwrap();

    #[cfg(target_os = "linux")]
    gtk::main();

    loop {
        std::thread::park();
    }
}
