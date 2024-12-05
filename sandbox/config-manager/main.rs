use dirs;
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::path::Path;
use std::{error::Error, fs::File};
use toml;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(skip)]
    file_path: String,

    database: DatabaseConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: {
                let config_file_name = "config-manager/config.toml";
                let config_file_location = dirs::config_local_dir().unwrap();

                format!(
                    "{}/{}",
                    if cfg!(debug_assertions) {
                        "./target/config"
                    } else {
                        config_file_location.to_str().unwrap_or(".")
                    },
                    config_file_name
                )
            },
            database: DatabaseConfig {
                server: String::from("Server"),
                ports: vec![80, 433],
                connection_max: 32,
                enabled: true,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseConfig {
    server: String,
    ports: Vec<u32>,
    connection_max: u32,
    enabled: bool,
}

fn read_config() -> Result<Config, Box<dyn Error>> {
    let default_config = Config {
        file_path: {
            let config_file_name = "config-manager/config.toml";
            let config_file_location = dirs::config_local_dir().unwrap();

            format!(
                "{}/{}",
                if cfg!(debug_assertions) {
                    "./target/config"
                } else {
                    config_file_location.to_str().unwrap_or(".")
                },
                config_file_name
            )
        },
        database: DatabaseConfig {
            server: String::from("Server"),
            ports: vec![80, 433],
            connection_max: 32,
            enabled: true,
        },
    };

    let file_path = &default_config.file_path;

    let path = Path::new(file_path);
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            // If the file doesn't exist, create a new file
            write_config(&default_config)?;

            File::open(&path)?
        }
    };

    let mut toml_str = String::new();

    file.read_to_string(&mut toml_str)?;

    let config: Config = toml::from_str(&toml_str).unwrap_or_default();

    Ok(config)
}

fn write_config(config: &Config) -> Result<File, Box<dyn Error>> {
    let default_config = Config {
        file_path: {
            let config_file_name = "config-manager/config.toml";
            let config_file_location = dirs::config_local_dir().unwrap();

            format!(
                "{}/{}",
                if cfg!(debug_assertions) {
                    "./target/config"
                } else {
                    config_file_location.to_str().unwrap_or(".")
                },
                config_file_name
            )
        },
        database: DatabaseConfig {
            server: String::from("Server"),
            ports: vec![80, 433],
            connection_max: 32,
            enabled: true,
        },
    };

    let file_location = &default_config.file_path;
    let path = Path::new(file_location);
    let parent_folder = path.parent().unwrap();
    std::fs::create_dir_all(parent_folder)?;

    let mut file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)?;

    let toml_str = toml::to_string_pretty(config)?;

    file.write_all(toml_str.as_bytes())?;

    Ok(file)
}

fn main() {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        // Read configuration from file
        let mut config = match read_config() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Error reading config: {}", err);

                return;
            }
        };

        println!("Config database: {:?}", config.database);

        // Modify config if needed
        config.database.enabled = false;

        // Write modified config back to file
        if let Err(err) = write_config(&config) {
            eprintln!("Error writing config: {}", err);
        }
    }
}
