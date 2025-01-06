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
    constants::{APP_NAME, CONFIG_FILE_NAME, DEFAULT_BAUD_RATE, DEFAULT_DEVICE_NAME},
    log_error, log_info,
    service::interaction::InteractionKind,
};

pub static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    file_path: String,

    pub settings: Settings,
    pub profiles: Vec<Profile>,
    pub layout: Layout,
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

#[derive(Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interaction {
    pub normal: InteractionKind,
    pub modkey: InteractionKind,
}

#[derive(Debug, Serialize, Deserialize)]
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
                current_profile: 2,
                device_name: DEFAULT_DEVICE_NAME.to_string(),
                port_name: String::new(),
                baud_rate: DEFAULT_BAUD_RATE,
            },
            // TODO: Remove these testing vectors!
            profiles: vec![
                Profile::new_profile("Profile 1".to_string(), false),
                Profile::new_profile("Profile 2".to_string(), true),
                Profile::new_profile("Profile 3".to_string(), false),
                Profile::new_profile("Profile 4".to_string(), false),
                Profile::new_profile("Profile 5".to_string(), false),
                Profile::new_profile("Profile 6".to_string(), false),
            ],
            layout: Layout {
                name: "Layout 1".to_string(),
                components: {
                    let mut components: HashMap<String, Component> = Default::default();

                    let button1 =
                        Component::new_button(1, "First Button".to_string(), (50.0, 50.0));
                    components.insert(button1.0, button1.1);

                    let potentiometer2 = Component::new_potentiometer(
                        2,
                        "Potentiometer 2".to_string(),
                        (150.0, 100.0),
                    );

                    components.insert(potentiometer2.0, potentiometer2.1);
                    let button2 =
                        Component::new_button(2, "Second Button".to_string(), (50.0, 100.0));
                    components.insert(button2.0, button2.1);

                    let button3 =
                        Component::new_button(3, "Thid Button".to_string(), (50.0, 150.0));
                    components.insert(button3.0, button3.1);

                    let button4 =
                        Component::new_button(4, "Fourth Button".to_string(), (50.0, 200.0));
                    components.insert(button4.0, button4.1);

                    let potentiometer1 = Component::new_potentiometer(
                        1,
                        "Potentiometer 1".to_string(),
                        (150.0, 200.0),
                    );
                    components.insert(potentiometer1.0, potentiometer1.1);

                    components
                },
            },
        }
    }
}

impl Profile {
    // TEST: Only for testing purposes! This method will get heavily changed!
    fn new_profile(profile_name: String, no_interactions: bool) -> Self {
        Self {
            name: profile_name,
            interactions: {
                let mut interactions: HashMap<String, Interaction> = Default::default();

                if !no_interactions {
                    // NOTE: The way this should happen (after the dashboard app is ready),
                    // is to auto-generate NONE interactions for all components every time
                    // you create a profile. so, user can set their interactions.
                    interactions.insert(
                        "Button:1".to_string(),
                        Interaction {
                            normal: InteractionKind::File(
                                "/home/mohsen/media/Music/ava".to_string(),
                            ),
                            modkey: InteractionKind::None(),
                        },
                    );

                    interactions.insert(
                        "Button:2".to_string(),
                        Interaction {
                            normal: InteractionKind::None(),
                            modkey: InteractionKind::File(
                                "/home/mohsen/media/Wallpapers/wallpaper.jpg".to_string(),
                            ),
                        },
                    );

                    interactions.insert(
                        "Potentiometer:2".to_string(),
                        Interaction {
                            normal: InteractionKind::None(),
                            modkey: InteractionKind::None(),
                        },
                    );

                    interactions.insert(
                        "Button:3".to_string(),
                        Interaction {
                            normal: InteractionKind::Command(
                                "notify-send \"hello!\"".to_string(),
                                "sh".to_string(),
                            ),
                            modkey: InteractionKind::None(),
                        },
                    );

                    interactions.insert(
                        "Button:4".to_string(),
                        Interaction {
                            normal: InteractionKind::Shortcut(
                                vec![enigo::Key::Control, enigo::Key::Unicode('p')],
                                String::new(),
                            ),
                            modkey: InteractionKind::Shortcut(
                                vec![enigo::Key::Alt, enigo::Key::Unicode('p')],
                                String::new(),
                            ),
                        },
                    );

                    interactions.insert(
                        "Potentiometer:1".to_string(),
                        Interaction {
                            normal: InteractionKind::None(),
                            modkey: InteractionKind::None(),
                        },
                    );
                }

                interactions
            },
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
            .open(&path)?;

        let toml_str = toml::to_string_pretty(&self)?;

        file.write_all(toml_str.as_bytes())?;

        Ok(file)
    }
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
