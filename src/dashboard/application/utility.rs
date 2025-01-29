use padpad_software::{constants::SERIAL_MESSAGE_SEP, tcp::client_to_server_message};

pub fn request_send_serial(message: &str) -> Result<String, String> {
    let request = format!("send_serial{}{}", SERIAL_MESSAGE_SEP, message);

    client_to_server_message(&request)
}

pub fn request_restart_service() -> Result<String, String> {
    client_to_server_message("restart")
}
