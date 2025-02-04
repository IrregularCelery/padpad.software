use enigo::{
    Direction::{Press, Release},
    Enigo, Keyboard, Settings,
};
use open;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::{
    config::{ComponentKind, Interaction, CONFIG},
    log_error,
    service::serial::Serial,
    tcp,
    utility::EnigoKey,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InteractionKind {
    None(), /* Can be used for interactions that are handled by the device, or no interactions */
    Command(String /* command */, String /* shell */),
    Application(String /* full_path */),
    Website(String /* url */),
    Shortcut(Vec<EnigoKey> /* keys */, String /* text */),
    File(String /* full_path */),
}

impl std::fmt::Display for InteractionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let type_str = format!("{:?}", self);

        write!(f, "{}", type_str.split('(').next().unwrap_or(&type_str))
    }
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
fn simulate_shortcut(keys: &Vec<EnigoKey> /* Vec<enigo::Key> */, text: &str) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    if text.is_empty() {
        for key in keys.iter() {
            if let Err(e) = enigo.key(key.0.clone(), Press) {
                log_error!(
                    "Shortcut simulation failed while pressing key {:?}: {:?}",
                    key,
                    e
                );
            }
        }

        for key in keys.iter().rev() {
            if let Err(e) = enigo.key(key.0.clone(), Release) {
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

fn do_interaction(kind: &InteractionKind, value: impl ToString) {
    let parse_value = |text: &str| text.replace("{value}", &value.to_string());

    match kind {
        InteractionKind::None() => (),
        InteractionKind::Command(command, unix_shell) => {
            run_command(&parse_value(command), &unix_shell)
        }
        InteractionKind::Application(app_full_path) => {
            open_application(&parse_value(app_full_path))
        }
        InteractionKind::Website(website_url) => open_website(&parse_value(website_url)),
        InteractionKind::Shortcut(keys, text) => simulate_shortcut(keys, &parse_value(text)),
        InteractionKind::File(file_full_path) => open_file(&parse_value(file_full_path)),
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

    let interactions =
        get_component_interactions(component_global_id).unwrap_or(Interaction::default());

    let interaction = if !modkey {
        &interactions.normal
    } else {
        &interactions.modkey
    };

    do_interaction(interaction, value);
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

    do_interaction(interaction, value);
}
