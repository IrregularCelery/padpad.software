#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod service;

mod log;

fn main() {
    log_info!("Application started at {:?}", std::env::current_exe());

    if service::tcp::is_another_instance_running() {
        log_error!("Another instance of the app is already running!");
        eprintln!("Another instance of the app is already running!");

        return;
    }

    // Read configuration or create it if it doesn't exist
    service::config_manager::init();

    // Initialize an empty Serial object
    service::serial::init();

    // Application service tray icon
    let tray_thread = std::thread::spawn(|| {
        log_info!("Tray thread is started...");

        service::tray::handle_tray_thread();
    });

    // IPC handling between dashboard and service app
    let tcp_server_thread = std::thread::spawn(|| {
        service::tcp::handle_tcp_server();
    });

    let serial_thread = std::thread::spawn(|| {
        let mut serial = service::serial::SERIAL
            .get()
            .expect("Could not retrieve SERIAL data! Maybe it wasn't initialized.")
            .lock()
            .unwrap();

        serial.handle_serial_port();
    });

    tray_thread
        .join()
        .expect_err("there was a problem while spawning the `tray` thread!");
    tcp_server_thread
        .join()
        .expect_err("there was a problem while spawning the `tcp_server` thread!");
    serial_thread
        .join()
        .expect_err("there was a problem while spawning the `tcp_server` thread!");
}
