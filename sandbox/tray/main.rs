#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use tray_icon::menu::Menu;

#[cfg(not(target_os = "linux"))]
use std::{cell::RefCell, rc::Rc};

use tray_icon::TrayIconBuilder;

fn main() {
    let path = concat!("./res/images/icon.png");
    let icon = load_icon(std::path::Path::new(path));

    // Since egui uses winit under the hood and doesn't use gtk on Linux, and we need gtk for
    // the tray icon to show up, we need to spawn a thread
    // where we initialize gtk and create the tray_icon
    #[cfg(target_os = "linux")]
    std::thread::spawn(|| {
        gtk::init().unwrap();
        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(Menu::new()))
            .with_icon(icon)
            .build()
            .unwrap();

        gtk::main();
    });

    #[cfg(not(target_os = "linux"))]
    let mut _tray_icon = Rc::new(RefCell::new(None));
    #[cfg(not(target_os = "linux"))]
    let tray_c = _tray_icon.clone();

    #[cfg(not(target_os = "linux"))]
    {
        tray_c.borrow_mut().replace(
            TrayIconBuilder::new()
                .with_menu(Box::new(Menu::new()))
                .with_icon(icon)
                .build()
                .unwrap(),
        );
    }

    loop {}
}

fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
