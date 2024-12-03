#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod service;

fn main() {
    // Application service tray icon
    let tray_thread = std::thread::spawn(|| {
        service::tray::handle_tray_thread();
    });

    // IPC handling between dashboard and service app
    let tcp_server_thread = std::thread::spawn(|| {
        service::tcp::handle_tcp_server();
    });

    tray_thread
        .join()
        .expect_err("there was a problem while spawning the `tray` thread!");
    tcp_server_thread
        .join()
        .expect_err("there was a problem while spawning the `tcp_server` thread!");
}
