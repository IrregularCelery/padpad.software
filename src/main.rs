#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use padpad_software::{config, log_error, log_info, service, tcp};

fn main() {
    log_info!("Application started at {:?}", std::env::current_exe());

    if tcp::is_another_instance_running() {
        log_error!("Another instance of the app is already running!");

        return;
    }

    // Read configuration or create it if it doesn't exist
    config::init();

    // Initialize an empty Serial object
    service::serial::init();

    // Application service tray icon
    let tray_thread = std::thread::Builder::new()
        .name("Tray".to_string())
        .spawn(|| {
            log_info!("Tray thread is started...");

            service::tray::handle_tray_thread();
        })
        .expect("Failed to spawn `Tray` thread!");

    // IPC handling between dashboard and service app
    let tcp_server_thread = std::thread::Builder::new()
        .name("TCP Server".to_string())
        .spawn(|| {
            log_info!("TCP Server thread is started...");

            tcp::handle_tcp_server();
        })
        .expect("Failed to spawn `TCP Server` thread!");

    let serial_thread = std::thread::Builder::new()
        .name("Serial".to_string())
        .spawn(|| {
            log_info!("Serial thread is started...");

            let mut serial = service::serial::SERIAL
                .get()
                .expect("Could not retrieve SERIAL data! Maybe it wasn't initialized.")
                .lock()
                .unwrap();

            serial.handle_serial_port();
        })
        .expect("Failed to spawn `Serial` thread!");

    // TEST: Testing thread for mutating server_data
    std::thread::Builder::new()
        .name("Test".to_string())
        .spawn(|| {
            let mut count: u32 = 0;
            loop {
                count = count + 1;

                if let Ok(mut data) = tcp::get_server_data().lock() {
                    let mut server_data = data.clone();

                    if count % 5 == 0 {
                        server_data.is_device_paired = true;
                    } else {
                        server_data.is_device_paired = false;
                    }

                    server_data.order = format!("Count: {}", count);

                    *data = server_data;
                }

                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        })
        .expect("Failed to spawn `Serial` thread!");

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
