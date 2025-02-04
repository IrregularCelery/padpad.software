use padpad_software::{constants::SERIAL_MESSAGE_SEP, tcp::client_to_server_message};

pub fn request_send_serial(message: &str) -> Result<String, String> {
    let request = format!("send_serial{}{}", SERIAL_MESSAGE_SEP, message);

    client_to_server_message(&request)
}

pub fn request_refresh_device() {
    request_send_serial("refresh_device").ok();
}

pub fn request_device_upload(data: String, save_to_flash: bool) -> Result<String, String> {
    // `u` => Upload, `M` => Save to Memory
    let request = format!("u{}{}", if save_to_flash { "M" } else { "-" }, data);

    request_send_serial(&request)
}

pub fn request_restart_service() -> Result<String, String> {
    client_to_server_message("restart")
}
