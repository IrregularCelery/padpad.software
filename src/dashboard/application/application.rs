use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
};

use eframe::egui::{self, Button, Context, Pos2, ProgressBar, Response, Ui, Vec2};

use padpad_software::{
    config::{update_config_and_server, Component, ComponentKind, Config, Layout},
    constants::{HOME_IMAGE_SIZE, SERIAL_MESSAGE_INNER_SEP, SERVER_DATA_UPDATE_INTERVAL},
    log_error, log_print,
    tcp::{client_to_server_message, ServerData},
    utility::extract_hex_bytes_and_serialize,
};

use crate::application::utility::request_send_serial;

use super::{get_current_style, widgets::Modal};

static SERVER_DATA: OnceLock<Arc<Mutex<ServerData>>> = OnceLock::new();
static ERROR_MESSAGE: OnceLock<Arc<Mutex<String>>> = OnceLock::new(); // Global vairable to keep the
                                                                      // last error message

pub struct AppWrapper {
    app: Rc<RefCell<Application>>,
}

impl AppWrapper {
    pub fn new(app: Application) -> Self {
        Self {
            app: Rc::new(RefCell::new(app)),
        }
    }

    fn handle_modal(&mut self, ctx: &Context) {
        let mut close_modal = false;
        let mut callbacks: Vec<Box<dyn FnMut()>> = Vec::new();

        {
            let mut app = self.app.borrow_mut();

            match &mut app.modal {
                Modal::Message { title, message } => {
                    let modal = egui::Modal::new(egui::Id::new("Modal::Message")).show(ctx, |ui| {
                        ui.set_width(300.0);

                        ui.heading(title);
                        ui.label(message.clone());

                        ui.add_space(32.0);

                        egui::Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Ok").clicked() {
                                    close_modal = true;
                                }
                            },
                        );
                    });

                    if modal.should_close() {
                        close_modal = true;
                    }
                }
                Modal::YesNo {
                    title,
                    question,
                    on_yes,
                    on_no,
                } => {
                    let modal = egui::Modal::new(egui::Id::new("Modal::YesNo")).show(ctx, |ui| {
                        ui.set_width(350.0);

                        ui.heading(title);
                        ui.label(question.clone());

                        ui.add_space(32.0);

                        egui::Sides::new().show(
                            ui,
                            |_ui| {},
                            |ui| {
                                if ui.button("Yes").clicked() {
                                    close_modal = true;

                                    if let Some(callback) = on_yes.take() {
                                        callbacks.push(callback);
                                    }
                                }

                                if ui.button("No").clicked() {
                                    close_modal = true;

                                    if let Some(callback) = on_no.take() {
                                        callbacks.push(callback);
                                    }
                                }
                            },
                        );
                    });

                    if modal.should_close() {
                        close_modal = true;

                        if let Some(callback) = on_no.take() {
                            callbacks.push(callback);
                        }
                    }
                }
                Modal::Custom { content } => {
                    egui::Modal::new(egui::Id::new("Modal::Custom")).show(ctx, |ui| {
                        (content)(ui);
                    });
                }
                Modal::None => (),
            }
        }

        if close_modal {
            let mut app = self.app.borrow_mut();

            app.close_modal();
        }

        for mut callback in callbacks {
            callback();
        }
    }
}

pub struct Application {
    close_app: (
        bool, /* show_close_modal */
        bool, /* can_close_app */
    ),
    error_modal: (
        bool,   /* show_error_modal */
        String, /* error_modal_text */
    ),
    modal: Modal,
    config: Option<Config>,
    server_data: ServerData,
    components: HashMap<String /* component_global_id */, String /* value */>,

    // TEMP VARIABLES
    new_layout_name: String,
    xbm_string: String,
}

impl eframe::App for AppWrapper {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        use egui::*;
        let style = get_current_style();
        ctx.set_style(style);

        {
            let mut app = self.app.borrow_mut();

            // Access the latest server data
            if let Some(server_data) = SERVER_DATA.get() {
                let new_server_data = server_data.lock().unwrap().clone();

                app.server_data = new_server_data;
            }

            // Access the last error message
            if let Some(last_error_message) = ERROR_MESSAGE.get() {
                let error_message = last_error_message.lock().unwrap().clone();

                if !error_message.is_empty() {
                    app.error_modal = (true, error_message);
                } else {
                    app.error_modal = (false, String::new());
                }
            }

            // Handle server orders
            if !app.server_data.order.is_empty() {
                let mut handled = false;

                match app.server_data.order.as_str() {
                    "reload_config" => {
                        handled = true;

                        if let Some(config) = &mut app.config {
                            config.load();
                        }
                    }
                    _ => (),
                }

                if handled {
                    if let Some(server_data) = SERVER_DATA.get() {
                        let mut new_server_data = server_data.lock().unwrap();

                        app.server_data.order = String::new();

                        *new_server_data = app.server_data.clone();
                    }

                    client_to_server_message("handled").ok();
                }
            }

            // Update component values
            if !app.server_data.last_updated_component.0.is_empty() {
                let component_global_id = app.server_data.last_updated_component.0.clone();
                let value = app.server_data.last_updated_component.1.clone();

                *app.components
                    .entry(component_global_id)
                    .or_insert(String::new()) = value;

                if let Some(server_data) = SERVER_DATA.get() {
                    let mut new_server_data = server_data.lock().unwrap();

                    app.server_data.last_updated_component = (String::new(), String::new());

                    *new_server_data = app.server_data.clone();
                }
            }
        }

        self.handle_modal(ctx);

        {
            let mut app = self.app.borrow_mut();

            app.handle_error_modal(ctx);

            app.handle_close_modal(ctx);

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
                app.server_data.is_client_connected
            ));
            ui.label(format!(
                "Device status: {}",
                if app.server_data.is_device_paired {
                    "Paired"
                } else {
                    "Not paired"
                }
            ));

            ui.label(format!("Server current order: {}", app.server_data.order));

            let mut port_name = String::new();
            let mut current_profile = String::new();

            if let Some(config) = &app.config {
                port_name = config.settings.port_name.clone();
                current_profile = config.settings.current_profile.to_string();
            }

            ui.text_edit_singleline(&mut port_name).enabled();

            ui.label(format!("Current profile: {}", current_profile));

            // Raw components layout
            ui.label(format!(
                    "Raw layout:\n- Buttons\n{}\n- Potentiometers\n{}",
                    app.server_data.raw_layout.0, app.server_data.raw_layout.1
            ));

            if ui.button("Auto-detect components").clicked() {
                let app_clone = self.app.clone();

                app.show_yes_no_modal(
                    "Override layout".to_string(),
                    "This operation will override the current layout!\nAre you sure you want to proceed?".to_string(),
                    move || {
                        let mut app = app_clone.borrow_mut();

                        app.detect_components();
                    },
                    || {},
                );
            }

            if ui.button("Send serial message").clicked() {
                request_send_serial("t50").ok();
            }

            let button = ui.button("hi");

            // Upload X BitMap
            egui::Window::new("Upload X BitMap")
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.text_edit_multiline(&mut app.xbm_string);

                    if ui.button("Save to memory").clicked() {
                            app.show_yes_no_modal(
                                "Override memory".to_string(),
                                "This operation will override the current memory!\nAre you sure you want to continue?".to_string(),
                                move || {
                                    // `m` = `Memory`, `1` = true
                                    request_send_serial("m1").ok();
                                },
                                || {},
                            );
                    }

                    if ui.button("Upload and Test").clicked() {
                        if app.server_data.is_device_paired {
                            let xbm_string = app.xbm_string.clone();

                            match extract_hex_bytes_and_serialize(&xbm_string, HOME_IMAGE_SIZE)
                            {
                                Ok(bytes) => {
                                    // `ui` = `Upload *HOME* Image`
                                    let message = format!("ui{}", &bytes);

                                    request_send_serial(message.as_str()).ok();

                                    app.show_message_modal("Ok".to_string(), "New X BitMap image was uploaded to the device.".to_string());
                                }
                                Err(error) => log_print!("ERROR: {}", error),
                            }
                        } else {
                            app.show_message_modal(
                                "Unavailable".to_string(),
                                "Device must be paired to be able to upload to it!".to_string(),
                            );
                        }
                    }

                    if ui.button("Remove X BitMap").clicked() {
                        app.show_yes_no_modal(
                            "Reset \"Home Image\"".to_string(),
                            "You're about to remove and reset current \"Home Image\" on your device!\nAre you sure you want to continue?".to_string(),
                            || {
                                // `ui` = `Upload *HOME* Image`, and since there's no value
                                // the device removes current image and set its default
                                request_send_serial("ui").ok();
                            },
                            || {}
                        );
                    }
                });

            if button.hovered() {
                ui.label("YES");
            }

            if button.clicked() {}

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
                    app.draw_layout(ctx, ui);
                });
        });
        }

        // Redraw continuously at 60 FPS
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

impl Application {
    fn handle_close_modal(&mut self, ctx: &Context) {
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

    fn handle_error_modal(&mut self, ctx: &Context) {
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

    fn show_message_modal(&mut self, title: String, message: String) {
        self.modal = Modal::Message { title, message }
    }

    fn show_yes_no_modal(
        &mut self,
        title: String,
        question: String,
        on_yes: impl FnMut() + 'static,
        on_no: impl FnMut() + 'static,
    ) {
        self.modal = Modal::YesNo {
            title,
            question,
            on_yes: Some(Box::new(on_yes)),
            on_no: Some(Box::new(on_no)),
        }
    }

    fn show_custom_modal(&mut self, content: impl FnMut(&mut Ui) + 'static) {
        self.modal = Modal::Custom {
            content: Box::new(content),
        };
    }

    fn close_modal(&mut self) {
        self.modal = Modal::None;
    }

    fn new_layout(&mut self, name: String) {
        if let Some(config) = &mut self.config {
            // Currently, Only one layout is supported
            if config.layout.is_some() {
                return;
            }

            let layout = Layout {
                name,
                components: Default::default(),
            };

            update_config_and_server(config, |c| {
                c.layout = Some(layout);
            });
        }
    }

    fn draw_empty_layout(&mut self, ui: &mut Ui) {
        ui.label("Layout is empty!");

        ui.text_edit_singleline(&mut self.new_layout_name);

        if ui.button("Add new layout").clicked() {
            self.new_layout(self.new_layout_name.clone());
        }
    }

    fn draw_layout(&mut self, _ctx: &Context, ui: &mut Ui) {
        match &self.config {
            Some(config) => {
                let layout = if let Some(layout) = &config.layout {
                    layout
                } else {
                    self.draw_empty_layout(ui);

                    return;
                };

                for component in &layout.components {
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
                    let value = match self.components.get(&component.0.to_string()) {
                        Some(v) => String::from(v),
                        None => String::new(),
                    };
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

    // TODO: Add corrent spacing for auto-generated components
    fn detect_components(&mut self) {
        let config = match &self.config {
            Some(config) => config,
            None => return,
        };

        let layout_name = if let Some(layout) = &config.layout {
            layout.name.clone()
        } else {
            log_error!("Access violation: Tried to access layout without creating it first!");

            return;
        };

        let mut layout = Layout {
            name: layout_name,
            components: Default::default(),
        };

        let mut index = 0;

        // Buttons
        let test = self.get_buttons();

        for (button_id, _button_normal, _button_mod) in test {
            index += 1;

            let button_name = format!("{} {}", ComponentKind::Button, button_id);

            let layout_button = Component::new_button(
                button_id,
                button_name,
                (index as f32 * 30.0, index as f32 * 30.0),
            );

            layout.components.insert(layout_button.0, layout_button.1);
        }

        // Potentiometers
        for (potentiometer_id, _potentiometer_value) in self.get_potentiometers() {
            index += 1;

            let potentiometer_name = format!("{} {}", ComponentKind::Button, potentiometer_id);

            let layout_potentiometer = Component::new_potentiometer(
                potentiometer_id,
                potentiometer_name,
                (index as f32 * 30.0, index as f32 * 30.0),
            );

            layout
                .components
                .insert(layout_potentiometer.0, layout_potentiometer.1);
        }

        if let Some(config) = &mut self.config {
            update_config_and_server(config, |c| {
                c.layout = Some(layout);
            });
        }
    }

    // Format: 1|97|98|2|99|100|3|101|102|4|103|104|5|105|106|...
    // separated by groups of three like "1|97|98"
    // 1,2,3... = button id in keymap (Started from 1)
    // 97|98 => 97 = letter 'a' normal, b = letter 'b' modkey
    // letters are in ascii number. e.g. 97 = a
    fn get_buttons(
        &self,
    ) -> impl Iterator<
        Item = (
            u8, /* id */
            u8, /* normal_key */
            u8, /* mod_key */
        ),
    > {
        let buttons_string = self.server_data.raw_layout.0.clone();

        let mut buttons: Vec<(u8, u8, u8)> = vec![];

        if buttons_string.is_empty() {
            return buttons.into_iter();
        }

        let parts: Vec<u8> = buttons_string
            .split(SERIAL_MESSAGE_INNER_SEP)
            .map(|s| s.parse::<u8>().unwrap())
            .collect();

        // Get values in groups of 3
        for part in parts.chunks(3) {
            let id = part[0];
            let normal_key = part[1];
            let mod_key = part[2];

            if id == 0 {
                continue;
            }

            buttons.push((id, normal_key, mod_key));
        }

        buttons.into_iter()
    }

    // Format: 1|25|2|45|3|12|4|99|5|75|...
    // separated by groups of two like "1|25"
    // 1,2,3... = potentiometer id (Started from 1)
    // 25 => value of the potentiometer
    fn get_potentiometers(&self) -> impl Iterator<Item = (u8 /* id */, u8 /* value */)> {
        let potentiometers_string = &self.server_data.raw_layout.1;

        let mut potentiometers: Vec<(u8, u8)> = vec![];

        if potentiometers_string.is_empty() {
            return potentiometers.into_iter();
        }

        let numbers: Vec<u8> = potentiometers_string
            .split(SERIAL_MESSAGE_INNER_SEP)
            .map(|s| s.parse::<u8>().unwrap())
            .collect();

        // Get values in groups of 2
        for chunk in numbers.chunks(2) {
            let id = chunk[0];
            let value = chunk[1];

            if id == 0 {
                continue;
            }

            potentiometers.push((id, value));
        }

        potentiometers.into_iter()
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
            modal: Modal::None,
            config: match Config::default().read() {
                Ok(config) => Some(config),
                Err(err) => {
                    log_error!("Error reading config file: {}", err);

                    None
                }
            },
            server_data: ServerData::default(),
            components: HashMap::default(),

            // TEMP VARIABLES
            new_layout_name: String::new(),
            xbm_string: String::new(),
        }
    }
}
