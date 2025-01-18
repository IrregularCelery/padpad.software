use dirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::prelude::*,
    path::Path,
    sync::{Mutex, OnceLock},
};
use toml;

use crate::{
    constants::{
        APP_NAME, CONFIG_FILE_NAME, DASHBOARD_DEVICE_INTERNAL_PROFILE, DEFAULT_BAUD_RATE,
        DEFAULT_DEVICE_NAME,
    },
    log_error, log_info,
    service::interaction::InteractionKind,
    tcp::{client_to_server_message, get_server_data},
};

pub static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    file_path: String,

    pub settings: Settings,
    pub profiles: Vec<Profile>,
    pub layout: Option<Layout>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    // General
    pub current_profile: usize,

    // Device
    pub device_name: String,

    // Serial
    pub port_name: String,
    pub baud_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub interactions: HashMap<
        String, /* component_global_id: component's key in `components` inside `Layout` */
        Interaction,
    >,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub components: HashMap<String /* key format: kind:id e.g. Button:1 */, Component>,
    pub size: (f32 /* width */, f32 /* height */),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interaction {
    pub normal: InteractionKind,
    pub modkey: InteractionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub label: String,
    pub position: (f32 /* x */, f32 /* y */),
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ComponentKind {
    None,
    Button,
    LED,
    Potentiometer,
    Joystick,
    RotaryEncoder,
    Display,
}

impl std::fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let type_str = format!("{:?}", self);

        write!(f, "{}", type_str.split('(').next().unwrap_or(&type_str))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: {
                let file_name = format!(
                    "{}/{}",
                    APP_NAME.to_lowercase(),
                    CONFIG_FILE_NAME.to_lowercase()
                );
                let file_location = dirs::config_local_dir().unwrap();

                format!(
                    "{}/{}",
                    if cfg!(debug_assertions) {
                        let app_path = std::env::current_exe().unwrap();
                        let app_folder = std::path::Path::new(&app_path).parent().unwrap();

                        app_folder.to_str().unwrap_or("./config/").to_string()
                    } else {
                        file_location.to_str().unwrap_or(".").to_string()
                    },
                    file_name
                )
            },
            settings: Settings {
                current_profile: 0,
                device_name: DEFAULT_DEVICE_NAME.to_string(),
                port_name: String::new(),
                baud_rate: DEFAULT_BAUD_RATE,
            },
            profiles: vec![
                // Device's internal profile
                Profile {
                    name: DASHBOARD_DEVICE_INTERNAL_PROFILE.to_string(),
                    interactions: Default::default(),
                },
            ],
            layout: None,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            name: "New Layout".to_string(),
            components: Default::default(),
            // TODO: Make this dynamic by getting the current screen size and subtract the
            // rest of the ui, to calculate the maximum layout size
            size: (1030.0, 580.0),
        }
    }
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            normal: InteractionKind::None(),
            modkey: InteractionKind::None(),
        }
    }
}

impl Config {
    pub fn load(&mut self) {
        *self = match self.read() {
            Ok(config) => config,
            Err(err) => {
                log_error!("Error reading config file: {}", err);

                return;
            }
        };
    }

    pub fn save<F>(&mut self, callback: F, write_to_file: bool)
    where
        F: FnOnce(&mut Self),
    {
        callback(self);

        if !write_to_file {
            return;
        }

        if let Err(err) = self.write() {
            log_error!("Error writing config: {}", err);
        }
    }

    pub fn read(&self) -> Result<Config, Box<dyn Error>> {
        let mut file_path = &self.file_path;

        // Look for a config file inside application's folder
        let app_path = std::env::current_exe().unwrap();
        let app_folder = std::path::Path::new(&app_path).parent().unwrap();
        let config_file = format!(
            "{}/{}",
            app_folder.to_str().unwrap_or("."),
            CONFIG_FILE_NAME.to_lowercase()
        );

        if Path::new(&config_file).exists() {
            log_info!("A config file was found in the application's folder and will be used...");

            file_path = &config_file;
        }

        // Couldn't find any config file in the application's folder
        let path = Path::new(file_path);
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => {
                // If the file doesn't exist, create a new one
                self.write()?;

                log_info!("New config file was created at `{}`", file_path);

                File::open(&path)?
            }
        };

        let mut toml_str = String::new();

        file.read_to_string(&mut toml_str)?;

        log_info!("Config file found at `{}`", file_path);

        let mut config: Config = toml::from_str(&toml_str).unwrap_or_default();

        // After reading, all the serde-ignored variables are empty
        config.file_path = Config::default().file_path;

        Ok(config)
    }

    pub fn write(&self) -> Result<File, Box<dyn Error>> {
        let path = Path::new(&self.file_path);
        let parent_folder = path.parent().unwrap();

        std::fs::create_dir_all(parent_folder)?;

        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        let toml_str = toml::to_string_pretty(&self)?;

        file.write_all(toml_str.as_bytes())?;

        Ok(file)
    }

    pub fn does_profile_exist(&self, profile_name: &String) -> bool {
        for profile in &self.profiles {
            if profile.name == *profile_name {
                return true;
            }
        }

        false
    }
}

impl Profile {
    pub fn add_interaction(_interaction: InteractionKind, _profile_id: u8) {}
}

impl Component {
    pub fn new_button(
        id: u8,
        label: String,
        position: (f32 /* x */, f32 /* y */),
    ) -> (String /* component_global_id */, Self) {
        let key = format!("{}:{}", ComponentKind::Button, id);

        (key, Self { label, position })
    }

    pub fn new_potentiometer(
        id: u8,
        label: String,
        position: (f32 /* x */, f32 /* y */),
    ) -> (String /* component_global_id */, Self) {
        let key = format!("{}:{}", ComponentKind::Potentiometer, id);

        (key, Self { label, position })
    }
}

pub fn init() -> bool {
    match Config::default().read() {
        Ok(config) => {
            CONFIG.get_or_init(|| Mutex::new(config));
        }
        Err(err) => {
            log_error!("Error reading config file: {}", err);

            return false;
        }
    };

    true
}

// Function for applying changes to config and send a message to `TCP clients` to reload it
pub fn update_config_and_client<F>(config: &mut Config, callback: F)
where
    F: FnOnce(&mut Config),
{
    config.save(callback, true);

    if let Ok(mut data) = get_server_data().lock() {
        let mut server_data = data.clone();

        server_data.order = "reload_config".to_string();

        *data = server_data;
    }
}

// Function for applying changes to config and send a message to `TCP server` to reload it
pub fn update_config_and_server<F>(config: &mut Config, callback: F)
where
    F: FnOnce(&mut Config),
{
    config.save(callback, true);

    client_to_server_message("reload_config").ok();
}
