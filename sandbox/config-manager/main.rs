use dirs;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    database: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseConfig {
    server: String,
    ports: Vec<u32>,
    connection_max: u32,
    enabled: bool,
}

fn read_config(file_location: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let default_config = Config {
        database: DatabaseConfig {
            server: String::from("Server"),
            ports: vec![80, 433],
            connection_max: 32,
            enabled: true,
        },
    };

    let path = Path::new(file_location);
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            // If the file doesn't exist, create a new file
            write_config(file_location, &default_config)?;

            File::open(&path)?
        }
    };

    let mut toml_str = String::new();

    file.read_to_string(&mut toml_str)?;

    let config: Config = toml::from_str(&toml_str)?;

    Ok(config)
}

fn write_config(file_location: &str, config: &Config) -> Result<File, Box<dyn std::error::Error>> {
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
    let config_file_name = "config-manager/config.toml";
    let config_file_location = dirs::config_local_dir().unwrap();

    let config_file_path = format!(
        "{}/{}",
        if cfg!(debug_assertions) {
            "./target/config"
        } else {
            config_file_location.to_str().unwrap_or(".")
        },
        config_file_name
    );

    // Read configuration from file
    let mut config = match read_config(&config_file_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error reading config: {}", err);

            return;
        }
    };

    println!("Config Server: {}", config.database.server);

    // Modify config if needed
    config.database.enabled = false;

    // Write modified config back to file
    if let Err(err) = write_config(&config_file_path, &config) {
        eprintln!("Error writing config: {}", err);
    }
}
