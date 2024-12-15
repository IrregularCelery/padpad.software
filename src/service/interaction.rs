use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use open;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::{
    config::{ComponentKind, CONFIG},
    log_error,
    service::serial::Serial,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum InteractionKind {
    None, /* Can be used for the interactions that are handled by the device, or no interactions */
    Command(String /* command */, String /* shell */),
    Application(String /* full_path */),
    Website(String /* url */),
    Shortcut(String /* shortcut */),
    File(String /* full_path */),
}

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

fn simulate_shortcut(keys: &str) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    match keys {
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

fn do_interaction(kind: &InteractionKind) {
    match kind {
        InteractionKind::None => (),
        InteractionKind::Command(command, unix_shell) => run_command(&command, &unix_shell),
        InteractionKind::Application(app_full_path) => open_application(&app_full_path),
        InteractionKind::Website(website_url) => open_website(&website_url),
        InteractionKind::Shortcut(shortcut) => simulate_shortcut(&shortcut),
        InteractionKind::File(file_full_path) => open_file(&file_full_path),
    }
}

pub fn do_button(id: u8, value: i32, modkey: bool, _serial: &mut Serial) {
    // Only on button press for now
    if value != 1 {
        return;
    }

    let config = CONFIG
        .get()
        .expect("Could not retrieve CONFIG data!")
        .lock()
        .unwrap();

    let current_profile = &config.profiles[config.settings.current_profile];

    let component_global_id = format!("Button:{}", id);

    let interactions = current_profile.interactions.get(&component_global_id);

    if interactions.is_none() {
        log_error!(
            "Couldn't find any interaction for the Button `{}` in the current profile `{}`",
            id,
            config.settings.current_profile
        );

        return;
    }

    let interactions = interactions.unwrap();

    let interaction = if !modkey {
        &interactions.normal
    } else {
        &interactions.modkey
    };

    do_interaction(interaction);
}
