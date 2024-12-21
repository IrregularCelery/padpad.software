use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui::{self, Button, Context, Pos2, Response, Ui, Vec2};

use padpad_software::{
    config::{ComponentKind, Config},
    log_error, log_trace,
    tcp::{client_to_server_message, ServerData},
};

use super::get_current_style;

static SERVER_RESPONSE_DATA: OnceLock<Arc<Mutex<ServerData>>> = OnceLock::new();

pub struct Application {
    close_app: (
        bool, /* can_close_app */
        bool, /* show_on_close_modal */
    ),
    config: Option<Config>,
    server_data: ServerData,
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
        if let Some(server_response_data) = SERVER_RESPONSE_DATA.get() {
            let server_data = server_response_data.lock().unwrap().clone();

            self.server_data = server_data
        }

        println!("Please");

        // Confirm exit functionality
        if ctx.input(|i| i.viewport().close_requested()) {
            // can_close_app
            if !self.close_app.0 {
                // Cancel closing the app if it's not allowed
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);

                // show_on_close_modal
                self.close_app.1 = true;
            }
        }

        // Confirm exit modal
        if self.close_app.1 {
            let modal = Modal::new(Id::new("Close Modal")).show(ctx, |ui| {
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
                            self.close_app.1 = false;
                            self.close_app.0 = true;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        if ui.button("Cancel").clicked() {
                            self.close_app.1 = false;
                        }
                    },
                );
            });

            if modal.should_close() {
                self.close_app.1 = false;
            }
        }

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
            ui.label(format!("Server status: {}", self.server_data.is_connected));
            ui.label(format!("Server current order: {}", self.server_data.order));

            let button = ui.button("hi");

            if button.hovered() {
                ui.label("YES");
            }

            if button.clicked() {
                println!("Button was clicked");

                if let Ok(response) =
                    client_to_server_message("A message from TCP client to server")
                {
                    log_trace!("{}", response);
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
                    let label = &component.1.label;
                    let position: Pos2 = component.1.position.into();
                    let size = Vec2::new(100.0, 30.0);

                    match kind {
                        ComponentKind::None => (),
                        ComponentKind::Button => {
                            let button = self.draw_button(ui, label, position, size);

                            if button.clicked() {
                                println!("{}: {}", label, id);
                            };
                        }
                        ComponentKind::LED => (),
                        ComponentKind::Potentiometer => (),
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
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let rect = egui::Rect::from_min_size(position, size.into());

        ui.put(rect, Button::new(label))
    }
}

impl Default for Application {
    fn default() -> Self {
        SERVER_RESPONSE_DATA
            .set(Arc::new(Mutex::new(ServerData::default())))
            .ok();

        let server_response = SERVER_RESPONSE_DATA
            .get()
            .expect("Failed to get `SERVER_RESPONSE_DATA`")
            .clone();

        // IPC handling between dashboard and service app
        std::thread::Builder::new()
            .name("TCP client".to_string())
            .spawn(move || {
                let mut server_data: Option<ServerData>;

                loop {
                    let response_update = |server_data: &Option<ServerData>| {
                        let mut response = server_response
                            .lock()
                            .expect("Failed to lock `SERVER_RESPONSE_DATA`");

                        if let Some(ref r) = server_data {
                            let mut new_response = r.clone();

                            new_response.set_connected(true);

                            *response = new_response;
                        } else {
                            // Reset server_data if the connection was lost
                            *response = ServerData::default();
                        }
                    };

                    match client_to_server_message("client::get_data") {
                        Ok(r) => {
                            server_data = Some(ServerData::parse(r));
                            response_update(&server_data);
                        }
                        Err(e) => {
                            server_data = None;
                            response_update(&server_data);
                            log_error!("{}", e);
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            })
            .expect("Failed to spawn `TCP client` thread!");

        Self {
            close_app: (false, false),
            config: match Config::default().read() {
                Ok(config) => Some(config),
                Err(err) => {
                    log_error!("Error reading config file: {}", err);

                    None
                }
            },
            server_data: ServerData::default(),
        }
    }
}
