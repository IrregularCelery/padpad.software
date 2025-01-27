use enigo::{
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use open;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::{
    config::{ComponentKind, Interaction, CONFIG},
    log_error,
    service::serial::Serial,
    tcp,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InteractionKind {
    None(), /* Can be used for interactions that are handled by the device, or no interactions */
    Command(String /* command */, String /* shell */),
    Application(String /* full_path */),
    Website(String /* url */),
    Shortcut(Vec<Key> /* keys */, String /* text */),
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

// If `text` parameter is NOT empty, `keys` will be ignored
fn simulate_shortcut(keys: &Vec<Key> /* Vec<enigo::Key> */, text: &str) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    if text.is_empty() {
        for key in keys.iter() {
            if let Err(e) = enigo.key(key.clone(), Press) {
                log_error!(
                    "Shortcut simulation failed while pressing key {:?}: {:?}",
                    key,
                    e
                );
            }
        }

        for key in keys.iter().rev() {
            if let Err(e) = enigo.key(key.clone(), Release) {
                log_error!(
                    "Shortcut simulation failed while releasing key {:?}: {:?}",
                    key,
                    e
                );
            }
        }

        return;
    }

    if let Err(e) = enigo.text(text) {
        log_error!("Shortcut simulation failed for text `{}`: {:?}", text, e);
    }
}

fn open_file(file_full_path: &str) {
    let file_path = file_full_path.trim();

    open::that_detached(&file_path).expect("Failed to open file");

    println!("File opened: {}", file_path);
}

fn do_interaction(kind: &InteractionKind) {
    match kind {
        InteractionKind::None() => (),
        InteractionKind::Command(command, unix_shell) => run_command(&command, &unix_shell),
        InteractionKind::Application(app_full_path) => open_application(&app_full_path),
        InteractionKind::Website(website_url) => open_website(&website_url),
        InteractionKind::Shortcut(keys, text) => simulate_shortcut(keys, &text),
        InteractionKind::File(file_full_path) => open_file(&file_full_path),
    }
}

fn update_server_data_component(component_global_id: String, value: String) {
    if let Ok(mut data) = tcp::get_server_data().lock() {
        let mut server_data = data.clone();

        server_data.last_updated_component = (component_global_id, value);

        *data = server_data;
    }
}

fn get_component_interactions(component_global_id: String) -> Option<Interaction> {
    let config = CONFIG
        .get()
        .expect("Could not retrieve CONFIG data!")
        .lock()
        .unwrap();

    let current_profile = &config.profiles[config.settings.current_profile];

    let interactions = current_profile
        .interactions
        .get(&component_global_id)
        .cloned();

    if interactions.is_none() {
        log_error!(
            "Couldn't find any interaction for the Component `{}` in the current profile `{}`",
            component_global_id,
            config.settings.current_profile
        );

        return None;
    }

    interactions
}

pub fn do_button(id: u8, value: i8, modkey: bool, _serial: &mut Serial) {
    let component_global_id = format!("{}:{}", ComponentKind::Button, id);

    update_server_data_component(component_global_id.clone(), value.to_string());

    // Only on button press for now
    if value != 1 {
        return;
    }

    // TEST: Upload key letters for buttons
    //if id == 4 {
    //    serial.write("u1:0|111;2:98|112;3:99|113;4:100|114;5:101|115;6:102|116;7:103|117;8:104|118;9:105|119;10:106|120;11:107|121;12:108|122;13:109|32;14:48|120;15:255|0;".to_string()); // 120 = 'w' | 121 = 'z'
    //
    //    return;
    //}

    let interactions =
        get_component_interactions(component_global_id).unwrap_or(Interaction::default());

    let interaction = if !modkey {
        &interactions.normal
    } else {
        &interactions.modkey
    };

    do_interaction(interaction);
}

pub fn do_potentiometer(
    id: u8,
    value: u8, /* the value is mapped between 0-99 in the device */
) {
    let component_global_id = format!("{}:{}", ComponentKind::Potentiometer, id);

    update_server_data_component(component_global_id.clone(), value.to_string());

    let interactions = get_component_interactions(component_global_id).unwrap_or(Interaction {
        normal: InteractionKind::None(),
        modkey: InteractionKind::None(),
    });

    let interaction = &interactions.normal;

    do_interaction(interaction);
}
