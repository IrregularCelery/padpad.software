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

pub fn blend_colors(
    color1: eframe::egui::Color32,
    color2: eframe::egui::Color32,
    t: f32,
) -> eframe::egui::Color32 {
    let t = t.clamp(0.0, 1.0); // Ensure `t` is between 0 and 1

    let r = ((1.0 - t) * color1.r() as f32 + t * color2.r() as f32) as u8;
    let g = ((1.0 - t) * color1.g() as f32 + t * color2.g() as f32) as u8;
    let b = ((1.0 - t) * color1.b() as f32 + t * color2.b() as f32) as u8;
    let a = ((1.0 - t) * color1.a() as f32 + t * color2.a() as f32) as u8;

    eframe::egui::Color32::from_rgba_premultiplied(r, g, b, a)
}
