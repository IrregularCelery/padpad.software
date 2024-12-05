use dirs;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, io::prelude::*, path::Path};
use toml;

use crate::config::{APP_NAME, DEFAULT_BAUD_RATE, DEFAULT_DEVICE_NAME};

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
                let file_name = format!("{}/config.toml", APP_NAME.to_lowercase());
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
    pub fn read(&self) -> Result<Config, Box<dyn Error>> {
        let path = Path::new(&self.file_path);
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => {
                // If the file doesn't exist, create a new one
                self.write()?;

                File::open(&path)?
            }
        };

        let mut toml_str = String::new();

        file.read_to_string(&mut toml_str)?;

        let mut config: Config = toml::from_str(&toml_str).unwrap_or_default();

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
