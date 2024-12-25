use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use eframe::egui::{self, Button, Context, Pos2, ProgressBar, Response, Ui, Vec2};

use padpad_software::{
    config::{ComponentKind, Config},
    constants::SERVER_DATA_UPDATE_INTERVAL,
    log_error, log_print,
    tcp::{client_to_server_message, ServerData},
};

use super::get_current_style;

static SERVER_DATA: OnceLock<Arc<Mutex<ServerData>>> = OnceLock::new();
static ERROR_MESSAGE: OnceLock<Arc<Mutex<String>>> = OnceLock::new(); // Global vairable to keep the
                                                                      // last error message

pub struct Application {
    close_app: (
        bool, /* show_close_modal */
        bool, /* can_close_app */
    ),
    error_modal: (
        bool,   /* show_error_modal */
        String, /* error_modal_text */
    ),
    config: Option<Config>,
    server_data: ServerData,
    components: HashMap<String /* component_global_id */, String /* value */>,
}

impl eframe::App for Application {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        use egui::*;
        let style = get_current_style();
        ctx.set_style(style);

        // Access the latest server data
        if let Some(server_data) = SERVER_DATA.get() {
            let new_server_data = server_data.lock().unwrap().clone();

            self.server_data = new_server_data;
        }

        // Access the last error message
        if let Some(last_error_message) = ERROR_MESSAGE.get() {
            let error_message = last_error_message.lock().unwrap().clone();

            if !error_message.is_empty() {
                self.error_modal = (true, error_message);
            } else {
                self.error_modal = (false, String::new());
            }
        }

        // Handle server orders
        if !self.server_data.order.is_empty() {
            let mut handled = false;

            match self.server_data.order.as_str() {
                "reload_config" => {
                    handled = true;

                    if let Some(config) = &mut self.config {
                        config.load();
                    }
                }
                _ => (),
            }

            if handled {
                if let Some(server_data) = SERVER_DATA.get() {
                    let mut new_server_data = server_data.lock().unwrap();

                    self.server_data.order = String::new();

                    *new_server_data = self.server_data.clone();
                }

                client_to_server_message("handled").ok();
            }
        }

        // Fill self.components with default values
        if self.components.is_empty() {
            if let Some(config) = &self.config {
                for component in &config.layout.components {
                    // Add all components with default values
                    if !self.components.contains_key(&component.0.to_string()) {
                        self.components
                            .insert(component.0.to_string(), "0".to_string());
                    }
                }
            }
        }

        // Update component values
        if !self.server_data.last_updated_component.0.is_empty() {
            let component_global_id = self.server_data.last_updated_component.0.clone();
            let value = self.server_data.last_updated_component.1.clone();

            *self
                .components
                .entry(component_global_id)
                .or_insert(String::new()) = value;

            if let Some(server_data) = SERVER_DATA.get() {
                let mut new_server_data = server_data.lock().unwrap();

                self.server_data.last_updated_component = (String::new(), String::new());

                *new_server_data = self.server_data.clone();
            }
        }

        self.close_modal(ctx);

        self.error_modal(ctx);

        // Custom main window
        CentralPanel::default().show(ctx, |ui| {
            let app_rect = ui.max_rect();
            let title_bar_height = 32.0;
            let title_bar_rect = {
                let mut rect = app_rect;
                rect.max.y = rect.min.y + title_bar_height;
                rect
            };

            // Adding support for dragging from the top bar of the app
            let title_bar_response = ui.interact(
                title_bar_rect,
                Id::new("title_bar"),
                Sense::click_and_drag(),
            );

            if title_bar_response.drag_started_by(PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }

            ui.allocate_new_ui(
                UiBuilder::new()
                    .max_rect(title_bar_rect)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    ui.add_space(8.0);

                    // Close and Minimize Button
                    let button_size = 16.0;

                    let close_button = ui
                        .add(Button::new(RichText::new("×").size(button_size)))
                        .on_hover_text("Close the window");

                    let minimized_button = ui
                        .add(Button::new(RichText::new("–").size(button_size)))
                        .on_hover_text("Minimize the window");

                    if close_button.clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    if minimized_button.clicked() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                },
            );

            // Custom window content
            ui.heading("Hello World!");
            ui.label("PadPad is under construction!");
            ui.label(format!(
                "Server status: {}",
                self.server_data.is_client_connected
            ));
            ui.label(format!(
                "Device status: {}",
                if self.server_data.is_device_paired {
                    "Paired"
                } else {
                    "Not paired"
                }
            ));

            ui.label(format!("Server current order: {}", self.server_data.order));

            let mut port_name = if let Some(config) = &self.config {
                config.settings.port_name.clone()
            } else {
                "".to_string()
            };

            ui.text_edit_singleline(&mut port_name).enabled();
            ui.label(format!("Server current order: {}", self.server_data.order));

            let button = ui.button("hi");

            if button.hovered() {
                ui.label("YES");
            }

            if button.clicked() {
                if let Ok(response) =
                    client_to_server_message("A message from TCP client to server")
                {
                    log_print!("{}", response);
                }
            }

            // Layout window
            egui::Window::new("Layout")
                //.movable(false)
                .resizable(false)
                .collapsible(false)
                .title_bar(false)
                .hscroll(true)
                .vscroll(true)
                .fixed_size(egui::Vec2::new(1030.0, 580.0))
                .default_pos(egui::Pos2::new(150.0, 150.0))
                .frame(egui::Frame {
                    fill: egui::Color32::RED,
                    rounding: 4.0.into(),
                    ..egui::Frame::default()
                })
                .show(ctx, |ui| {
                    self.draw_layout(ctx, ui);
                });
        });

        // Redraw continuously at 60 FPS
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

impl Application {
    fn close_modal(&mut self, ctx: &Context) {
        // Confirm exit functionality
        if ctx.input(|i| i.viewport().close_requested()) {
            // can_close_app
            if !self.close_app.1 {
                // Cancel closing the app if it's not allowed
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);

                // show_close_modal
                self.close_app.0 = true;
            }
        }

        // Confirm exit modal
        if self.close_app.0 {
            let modal = egui::Modal::new(egui::Id::new("Close Modal")).show(ctx, |ui| {
                ui.set_width(200.0);
                ui.heading("Are you sure you want to close the application?");

                ui.add_space(32.0);

                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Close").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            self.close_app.0 = false;
                            self.close_app.1 = true;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        if ui.button("Cancel").clicked() {
                            self.close_app.0 = false;
                        }
                    },
                );
            });

            if modal.should_close() {
                self.close_app.0 = false;
            }
        }
    }

    fn error_modal(&mut self, ctx: &Context) {
        if self.error_modal.0 {
            let modal = egui::Modal::new(egui::Id::new("Error Modal")).show(ctx, |ui| {
                ui.set_width(250.0);

                ui.heading(self.error_modal.1.clone());
            });

            if modal.should_close() {
                self.error_modal.0 = false;
            }
        }
    }

    fn draw_layout(&mut self, _ctx: &Context, ui: &mut Ui) {
        match &self.config {
            Some(config) => {
                for component in &config.layout.components {
                    let kind_id: Vec<&str> = component.0.split(':').collect();

                    let kind = match kind_id.first() {
                        Some(&"Button") => ComponentKind::Button,
                        Some(&"LED") => ComponentKind::LED,
                        Some(&"Potentiometer") => ComponentKind::Potentiometer,
                        Some(&"Joystick") => ComponentKind::Joystick,
                        Some(&"RotaryEncoder") => ComponentKind::RotaryEncoder,
                        Some(&"Display") => ComponentKind::Display,
                        _ => ComponentKind::None,
                    };
                    let id = kind_id.get(1).unwrap_or(&"0").parse::<u8>().unwrap_or(0);
                    let value = self.components.get(&component.0.to_string()).unwrap();
                    let label = &component.1.label;
                    let position: Pos2 = component.1.position.into();
                    let size = Vec2::new(100.0, 30.0);

                    match kind {
                        ComponentKind::None => (),
                        ComponentKind::Button => {
                            let button = self.draw_button(
                                ui,
                                label,
                                position,
                                size,
                                value.parse::<i8>().unwrap_or(0),
                            );

                            if button.clicked() {
                                log_print!("{}: {}", label, id);
                            };
                        }
                        ComponentKind::LED => (),
                        ComponentKind::Potentiometer => {
                            self.draw_potentiometer(
                                ui,
                                label,
                                position,
                                size,
                                value.parse::<u8>().unwrap_or(0),
                            );
                        }
                        ComponentKind::Joystick => (),
                        ComponentKind::RotaryEncoder => (),
                        ComponentKind::Display => (),
                    }
                }
            }
            None => {
                // Need to wait before the config is ready
                ui.label("Loading...");
            }
        }
    }

    fn draw_button(
        &self,
        ui: &mut Ui,
        label: &String,
        relative_position: Pos2, /* relative to window position */
        size: Vec2,
        value: i8,
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let rect = egui::Rect::from_min_size(position, size.into());

        let button_color = if value > 0 {
            egui::Color32::from_rgb(100, 200, 100)
        } else {
            ui.style().visuals.widgets.inactive.bg_fill
        };

        let response = ui.put(rect, Button::new(label).fill(button_color));

        response
    }

    fn draw_potentiometer(
        &self,
        ui: &mut Ui,
        _label: &String,
        relative_position: Pos2, /* relative to window position */
        size: Vec2,
        value: u8,
    ) {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let rect = egui::Rect::from_min_size(position, size.into());

        // value is mapped between 0-99, therefore we can device it by 100 to get 0-1 value
        let value = (value as f32) / 100.0;

        ui.put(rect, ProgressBar::new(value).show_percentage());
    }
}

impl Default for Application {
    fn default() -> Self {
        let mut first_time = true;

        let server_response = SERVER_DATA
            .get_or_init(|| Arc::new(Mutex::new(ServerData::default())))
            .clone();

        let error_message = ERROR_MESSAGE
            .get_or_init(|| Arc::new(Mutex::new(String::new())))
            .clone();

        // IPC handling between dashboard and service app
        std::thread::Builder::new()
            .name("TCP client".to_string())
            .spawn(move || {
                let update_response = |server_data: &Option<ServerData>| {
                    let mut response = server_response
                        .lock()
                        .expect("Failed to lock `SERVER_DATA`");

                    if let Some(ref data) = server_data {
                        let mut new_server_data = data.clone();

                        new_server_data.is_client_connected = true;

                        *response = new_server_data;
                    } else {
                        // Reset server_data if the connection was lost
                        *response = ServerData::default();
                    }
                };

                let mut server_data: Option<ServerData>;
                let mut data_message = "force_data".to_string();

                loop {
                    let mut update_interval = SERVER_DATA_UPDATE_INTERVAL;

                    match client_to_server_message(&data_message) {
                        Ok(r) => {
                            // "0" means the data hasn't been changed since last ping
                            if r != "0".to_string() {
                                server_data = Some(ServerData::parse(r));
                                update_response(&server_data);

                                // Reset error message
                                if let Ok(mut error) = error_message.lock() {
                                    *error = String::new();
                                }
                            }
                        }
                        Err(e) => {
                            update_interval = 1000;

                            server_data = None;
                            update_response(&server_data);

                            // Set error message for `error_modal` in `Application`
                            if let Ok(mut error) = error_message.lock() {
                                *error = e.clone();
                            }

                            log_error!("{}", e.replace('\n', " "));
                        }
                    }

                    if first_time {
                        first_time = false;

                        data_message = "data".to_string();
                    }

                    std::thread::sleep(std::time::Duration::from_millis(update_interval));
                }
            })
            .expect("Failed to spawn `TCP client` thread!");

        Self {
            close_app: (false, false),
            error_modal: (false, String::new()),
            config: match Config::default().read() {
                Ok(config) => Some(config),
                Err(err) => {
                    log_error!("Error reading config file: {}", err);

                    None
                }
            },
            server_data: ServerData::default(),
            components: HashMap::default(),
        }
    }
}
