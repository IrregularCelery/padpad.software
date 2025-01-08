// Function will look for the hex bytes inside a string that at least has braces "{", "}"
// Return serialized string of all hex bytes 2 by 2
// e.g. `input` = { 0x00, 0xFE, 0x15, ..., 0xFF } => "00FE15..FF".to_string()
pub fn extract_hex_bytes_and_serialize(input: &str, size: usize) -> Result<String, String> {
    let mut output = String::new();

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
                        output.push_str(&format!("{:02X?}", byte));
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
    let output_size = output.len() / 2; // Each byte's serialized string is 2 characters

    if output_size != size {
        return Err(format!(
            "Invalid number of bytes: expected {}, found {}",
            size, output_size
        ));
    }

    Ok(output)
}
