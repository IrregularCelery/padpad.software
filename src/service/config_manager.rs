use dirs;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::File,
    io::prelude::*,
    path::Path,
    sync::{Mutex, OnceLock},
};
use toml;

use crate::{
    config::{APP_NAME, CONFIG_FILE_NAME, DEFAULT_BAUD_RATE, DEFAULT_DEVICE_NAME},
    log_error, log_info,
};

pub static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    file_path: String,

    pub settings: Settings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    // General
    pub device_name: String,
    pub port_name: String,

    // Serial
    pub baud_rate: u32,
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
                        "./target/config"
                    } else {
                        file_location.to_str().unwrap_or(".")
                    },
                    file_name
                )
            },
            settings: Settings {
                device_name: DEFAULT_DEVICE_NAME.to_string(),
                port_name: "".to_string(),
                baud_rate: DEFAULT_BAUD_RATE,
            },
        }
    }
}

impl Config {
    pub fn reload(&mut self) {
        *self = match self.read() {
            Ok(config) => config,
            Err(err) => {
                log_error!("Error reading config file: {}", err);

                return;
            }
        };
    }

    pub fn update<F>(&mut self, callback: F, write_to_file: bool)
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

    fn read(&self) -> Result<Config, Box<dyn Error>> {
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

        let mut config: Config = toml::from_str(&toml_str).unwrap_or_default();

        // After reading, all the serde-ignored variables are empty
        config.file_path = Config::default().file_path;

        Ok(config)
    }

    fn write(&self) -> Result<File, Box<dyn Error>> {
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
