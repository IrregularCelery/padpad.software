#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use service::config_manager::CONFIG;

mod config;
mod service;

fn main() {
    if service::tcp::is_another_instance_running() {
        eprintln!("Another instance of the app is already running!");

        return;
    }

    // Read configuration or create it if it doesn't exist
    service::config_manager::init();

    // Application service tray icon
    let tray_thread = std::thread::spawn(|| {
        service::tray::handle_tray_thread();
    });

    // IPC handling between dashboard and service app
    let tcp_server_thread = std::thread::spawn(|| {
        service::tcp::handle_tcp_server();
    });

    let test_thread = std::thread::spawn(move || {
        if let Some(c) = CONFIG.get() {
            let mut config = c.lock().unwrap();

            println!("Read all data from the config file");

            println!("Config settings: {:?}", config.settings);

            std::thread::sleep(std::time::Duration::from_millis(1000));

            println!("Changing `port_name` to test");

            config.update(|c| c.settings.port_name = "test".to_string(), false);

            println!("Config settings: {:?}", config.settings);

            println!("Waiting for you to change the config file manually...");

            std::thread::sleep(std::time::Duration::from_millis(5000));

            config.reload();

            println!("Config settings: {:?}", config.settings);
        }

        std::thread::sleep(std::time::Duration::from_millis(1000));
    });

    tray_thread
        .join()
        .expect_err("there was a problem while spawning the `tray` thread!");
    tcp_server_thread
        .join()
        .expect_err("there was a problem while spawning the `tcp_server` thread!");
    test_thread.join().unwrap();
}
