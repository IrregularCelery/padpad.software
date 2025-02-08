use serde::{Deserialize, Serialize};

// Function will look for the hex bytes inside a string that at least has braces "{", "}"
// Return Vec<u8> of all hex bytes
// e.g. `input` = { 0x00, 0xFE, 0x15, ..., 0xFF } => [0, 254, 21, ..., 255]
pub fn extract_hex_bytes(input: &str, size: usize) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();

    let mut inside_braces = false;
    let mut current_byte = String::new(); // Temporary variable to hold the current hex value

    for character in input.chars() {
        match character {
            '{' => {
                inside_braces = true;

                continue;
            }
            '}' => {
                if !inside_braces {
                    return Err("Unexpected closing brace '}' found!".to_string());
                }

                break;
            }
            _ => (),
        }

        if inside_braces {
            if character.is_whitespace() || character == ',' {
                // Ignore whitespace and commas
                continue;
            }

            current_byte.push(character);

            if current_byte.len() == 4 && current_byte.starts_with("0x") {
                // Check if we have a valid hex byte
                match u8::from_str_radix(&current_byte[2..], 16) {
                    Ok(byte) => {
                        output.push(byte);
                        current_byte.clear();
                    }
                    Err(_) => return Err(format!("Invalid hex byte: {}", current_byte)),
                }
            }

            if current_byte.len() > 4 {
                return Err(format!("Invalid format in: {}", current_byte));
            }
        }
    }

    // Validate total byte count
    if output.len() != size {
        return Err(format!(
            "Invalid number of bytes: expected {}, found {}",
            size,
            output.len()
        ));
    }

    Ok(output)
}

pub fn hex_bytes_vec_to_string(input: &[u8]) -> String {
    input.iter().map(|b| format!("{:02X}", b)).collect()
}

pub fn hex_bytes_string_to_vec(input: &str) -> Result<Vec<u8>, String> {
    if input.len() % 2 != 0 {
        return Err("Invalid hex string length".to_string());
    }

    (0..input.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&input[i..i + 2], 16)
                .map_err(|_| format!("Invalid hex byte: {}", &input[i..i + 2]))
        })
        .collect()
}

pub fn restart() {
    let args: Vec<String> = std::env::args().collect();
    std::process::Command::new(&args[0])
        .args(&args[1..])
        .spawn()
        .expect("Failed to restart");
    std::process::exit(0);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnigoKey(pub enigo::Key);

impl std::fmt::Display for EnigoKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                enigo::Key::Unicode(letter) => {
                    format!("Key '{}'", letter.to_uppercase())
                }
                _ => format!("{:?}", self.0),
            }
        )
    }
}

pub fn get_app_name_without_extension() -> Option<String> {
    let exe_path = std::env::current_exe().ok()?;

    let exe_file_name = exe_path.file_name()?;
    let exe_name = exe_file_name.to_str()?;

    let app_name = std::path::Path::new(exe_name)
        .file_stem()?
        .to_str()?
        .to_string();

    Some(app_name)
}
