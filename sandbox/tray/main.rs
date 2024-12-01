use tray_item::{IconSource, TrayItem};

fn main() {
    #[cfg(target_os = "linux")]
    gtk::init().unwrap();

    let mut tray = TrayItem::new("Tray Example", IconSource::Resource("app-icon")).unwrap();

    tray.add_label("Tray Label").unwrap();

    tray.add_menu_item("Hello", || {
        println!("Hello!");
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

    loop {}
}
