use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use open;
use std::process::Command;

use super::serial::Serial;

fn run_command(command: &str, unix_shell: &str) {
    let cmd = command.trim();

    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", cmd])
            .spawn()
            .expect("Failed to run command");
    } else {
        let shell = unix_shell.trim();

        Command::new(shell)
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Failed to run command");
    }

    println!("Command executed: {}", cmd);
}

fn open_application(app_full_path: &str) {
    let app_path = app_full_path.trim();

    Command::new(app_path)
        .spawn()
        .expect("Failed to open application");

    println!("Application opened: {}", app_path);
}

fn open_website(website_url: &str) {
    let url = website_url.trim();

    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    };

    open::that_detached(&full_url).expect("Failed to open website");

    println!("Website opened: {}", full_url);
}

fn simulate_shortcut(shortcut: &str) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    match shortcut {
        "test" => {
            enigo
                .text("Subscribe to IrregularCelery on YouTube please :D")
                .unwrap();
        }
        "dmenu" => {
            enigo.key(Key::Alt, Press).unwrap();
            enigo.key(Key::Unicode('p'), Click).unwrap();
            enigo.key(Key::Alt, Release).unwrap();
        }
        _ => println!("Shortcut not recognized or not implemented."),
    }
}

fn open_file(file_full_path: &str) {
    let file_path = file_full_path.trim();

    open::that_detached(&file_path).expect("Failed to open file");

    println!("File opened: {}", file_path);
}

pub fn do_button(id: u8, value: i32, modkey: bool, serial: &mut Serial) {
    // TEST
    match id {
        1 => {
            if !modkey {
                serial.write(format!("l{}", value));
            }
        }
        _ => {}
    }
}
