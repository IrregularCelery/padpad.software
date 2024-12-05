#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

mod config;
mod service;

fn main() {
    // Read configuration or create it if it doesn't exist
    let config = match service::config_manager::Config::default().read() {
        Ok(config) => Arc::new(Mutex::new(config)),
        Err(err) => {
            eprintln!("Error reading config file: {}", err);

            return;
        }
    };

    // Application service tray icon
    let tray_thread = std::thread::spawn(|| {
        service::tray::handle_tray_thread();
    });

    // IPC handling between dashboard and service app
    let tcp_server_thread = std::thread::spawn(|| {
        service::tcp::handle_tcp_server();
    });

    let config_clone = config.clone();

    let test_thread = std::thread::spawn(move || {
        let mut config = config_clone.lock().unwrap();

        println!("Read all data from teh config file");

        println!("Config settings: {:?}", config.settings);

        std::thread::sleep(std::time::Duration::from_millis(1000));

        println!("Changing `port_nam` to test");

        config.settings.port_name = "test".to_string();

        println!("Config settings: {:?}", config.settings);

        println!("Waiting for you to change the config file manually...");

        std::thread::sleep(std::time::Duration::from_millis(5000));

        *config = match config.read() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Error reading config file: {}", err);

                return;
            }
        };

        println!("Config settings: {:?}", config.settings);

        //// Write modified config back to file
        //if let Err(err) = config.write() {
        //    eprintln!("Error writing config: {}", err);
        //}

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
