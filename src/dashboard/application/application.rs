use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use eframe::egui::{self, Context, DragValue, Pos2, Rect, Response, Ui, Vec2};

use super::{get_current_style, utility::request_send_serial, widgets::*};
use padpad_software::{
    config::{
        update_config_and_server, Component, ComponentKind, Config, Interaction, Layout, Profile,
    },
    constants::{
        DASHBOARD_DISAPLY_PIXEL_SIZE, DASHBOARD_PROFILE_MAX_CHARACTERS, HOME_IMAGE_BYTES_SIZE,
        HOME_IMAGE_DEFAULT_BYTES, HOME_IMAGE_HEIGHT, HOME_IMAGE_WIDTH, SERIAL_MESSAGE_INNER_SEP,
        SERIAL_MESSAGE_SEP, SERVER_DATA_UPDATE_INTERVAL,
    },
    log_error,
    tcp::{client_to_server_message, ServerData},
    utility::{extract_hex_bytes, hex_bytes_string_to_vec, hex_bytes_vec_to_string},
};

static SERVER_DATA: OnceLock<Arc<Mutex<ServerData>>> = OnceLock::new();
static ERROR_MESSAGE: OnceLock<Arc<Mutex<String>>> = OnceLock::new(); // Global vairable to keep the
                                                                      // last unavoidable error message

pub struct Application {
    close_app: (
        bool, /* show_close_popup */
        bool, /* can_close_app */
    ),
    unavoidable_error: (
        bool,   /* show_unavoidable_error */
        String, /* unavoidable_error_text */
    ),
    modal: Arc<Mutex<ModalManager>>,
    config: Option<Config>,
    server_data: ServerData,
    components: HashMap<String /* component_global_id */, String /* value */>,
    editing_layout: bool,
    dragged_component_offset: (f32, f32),
    layout_grid: (bool /* enabled/disabled */, f32 /* size */),
    layout_backup_components: HashMap<String, Component>, // For storing current layout components while editing

    // Visuals
    global_shadow: f32,

    // TEMP VARIABLES
    new_layout_name: String,
    new_layout_size: (f32, f32),
    new_profile_name: String,
    last_profile_name: String, // Used for updating a profile
    profile_exists: bool,
    xbm_string: String,
    paired_status_panel: (f32 /* position_x */, f32 /* opacity */),

    // TODO: Remove this
    test_potentiometer_style: u8,
    test_potentiometer_value: f32,
    test_joystick_value: (f32, f32),

    // Constants
    component_button_size: (f32 /* width */, f32 /* height */),
    component_led_size: (f32 /* width */, f32 /* height */),
    component_potentiometer_size: (f32 /* width */, f32 /* height */),
    component_joystick_size: (f32 /* width */, f32 /* height */),
    component_rotary_encoder_size: (f32 /* width */, f32 /* height */),
    component_display_size: (f32 /* width */, f32 /* height */),
}

impl eframe::App for Application {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        use egui::*;

        ctx.set_style(get_current_style());

        // Server data
        self.handle_server_data();

        // Modal manager
        self.handle_modal(ctx);

        // Unavoidable errors
        self.handle_unavoidable_error(ctx);

        // Application confirm close popup
        self.handle_close_popup(ctx);

        // Layout
        self.draw_layout(ctx);

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
                    .layout(Layout::right_to_left(Align::Center)),
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
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }

                    if minimized_button.clicked() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                    }

                    let editing_layout_response =
                        ui.checkbox(&mut self.editing_layout, "Enable editing layout");

                    if editing_layout_response.clicked() {
                        if self.editing_layout {
                            // Started editing layout

                            if let Some(config) = &self.config {
                                if let Some(layout) = &config.layout {
                                    self.layout_backup_components = layout.components.clone();
                                }
                            }
                        } else {
                            // Stopped editing layout

                            if !self.layout_backup_components.is_empty() {
                                // TODO: Check if the current layout was actually edited
                                self.show_yes_no_modal(
                                    "layout-edited",
                                    "Edited Layout".to_string(),
                                    "You have changed your current layout, Do you want to save it?"
                                        .to_string(),
                                    |app| {
                                        app.close_modal();

                                        app.save_current_layout();

                                        app.show_message_modal(
                                            "layout-saved-successfully",
                                            "Success".to_string(),
                                            "Your current layout was saved successfully!"
                                                .to_string(),
                                        )
                                    },
                                    |app| {
                                        app.close_modal();
                                    },
                                    false,
                                );

                                self.layout_backup_components = Default::default();
                            }
                        }
                    }
                },
            );

            // Custom main window content

            if let Some(config) = &self.config {
                if config.layout.is_none() {
                    self.draw_new_layout_button(ui);
                }
            }

            self.draw_status_indicator(ui);
        });

        if cfg!(debug_assertions) {
            self.draw_debug_panel(ctx);
        }

        // Redraw continuously at 60 FPS
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

impl Application {
    fn resize_and_center_window(&self, ctx: &Context, size: Vec2) {
        let monitor_size = ctx.input(|i: &egui::InputState| i.viewport().monitor_size);

        let position = {
            let mut x = 0.0;
            let mut y = 0.0;

            if let Some(resolution) = monitor_size {
                x = (resolution.x - size.x) / 2.0;
                y = (resolution.y - size.y) / 2.0;
            }

            egui::pos2(x, y)
        };

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
    }

    fn handle_server_data(&mut self) {
        // Access the latest server data
        if let Some(server_data) = SERVER_DATA.get() {
            let new_server_data = server_data.lock().unwrap().clone();

            self.server_data = new_server_data;
        }

        // Access the last error message
        if let Some(last_error_message) = ERROR_MESSAGE.get() {
            let error_message = last_error_message.lock().unwrap().clone();

            if !error_message.is_empty() {
                self.unavoidable_error = (true, error_message);
            } else {
                self.unavoidable_error = (false, String::new());
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
    }

    fn handle_close_popup(&mut self, ctx: &Context) {
        // Confirm exit functionality
        if ctx.input(|i| i.viewport().close_requested()) {
            // can_close_app
            if !self.close_app.1 {
                // Cancel closing the app if it's not allowed
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);

                // Check if there's unsaved layout

                // show_close_popup
                self.close_app.0 = true;
            }
        }

        // Confirm exit modal
        if self.close_app.0 {
            let modal = egui::Modal::new(egui::Id::new("Close Modal"))
                .frame(
                    egui::Frame::popup(&get_current_style()).inner_margin(egui::Margin::same(24.0)),
                )
                .backdrop_color(Color::BLACK.gamma_multiply(0.5))
                .show(ctx, |ui| {
                    ui.set_width(265.0);

                    ui.scope(|ui| {
                        let mut style = get_current_style();

                        style.text_styles.insert(
                            egui::TextStyle::Body,
                            egui::FontId::new(24.0, egui::FontFamily::Proportional),
                        );

                        style.visuals.override_text_color = Some(Color::WHITE);
                        style.visuals.widgets.noninteractive.bg_stroke =
                            egui::Stroke::new(1.0, Color::WHITE);

                        ui.set_style(style);

                        ui.vertical_centered(|ui| {
                            ui.label("Close Application");
                        });

                        ui.separator();

                        ui.add_space(ui.spacing().item_spacing.x);
                    });

                    ui.label("Are you sure you want to close the application?");

                    ui.add_space(ui.spacing().item_spacing.x * 2.5);

                    ui.horizontal_top(|ui| {
                        let spacing = ui.spacing().item_spacing.x;

                        let total_width = ui.available_width();
                        let button_width = (total_width - spacing) / 2.0;

                        if ui
                            .add_sized([button_width, 0.0], egui::Button::new("Cancel"))
                            .clicked()
                        {
                            self.close_app.0 = false;
                        }

                        if ui
                            .add_sized([button_width, 0.0], egui::Button::new("Close"))
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            self.close_app.0 = false;
                            self.close_app.1 = true;

                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });

            if modal.should_close() {
                self.close_app.0 = false;
            }
        }
    }

    fn handle_unavoidable_error(&mut self, ctx: &Context) {
        if self.unavoidable_error.0 {
            egui::Modal::new(egui::Id::new("unavoidable-error"))
                .frame(
                    egui::Frame::popup(&get_current_style())
                        .inner_margin(egui::Margin::same(24.0))
                        .stroke(egui::Stroke::new(1.0, Color::WHITE)),
                )
                .backdrop_color(Color::BLACK.gamma_multiply(0.95))
                .show(ctx, |ui| {
                    ui.set_width(350.0);

                    ui.vertical_centered(|ui| {
                        ui.label(self.unavoidable_error.1.clone());
                    });
                });
        }
    }

    fn handle_modal(&mut self, ctx: &Context) {
        let modals = match self.modal.lock() {
            Ok(modal) => modal.stack.clone(),
            Err(_) => vec![],
        };

        let mut close_indices = Vec::new();

        for (index, modal) in modals.iter().enumerate() {
            let modal_ui = egui::Modal::new(egui::Id::new(modal.id))
                .frame(
                    egui::Frame::popup(&get_current_style())
                        .inner_margin(egui::Margin::symmetric(48.0, 24.0)),
                )
                .backdrop_color(egui::Color32::from_black_alpha(64))
                .show(ctx, |ui| {
                    (modal.content)(ui, self);
                });

            if modal_ui.should_close() {
                close_indices.push(index);
            }
        }

        if !close_indices.is_empty() {
            if let Ok(mut modal) = self.modal.lock() {
                for &index in close_indices.iter().rev() {
                    modal.stack.remove(index);
                }
            }
        }
    }

    fn show_custom_modal(
        &self,
        id: &'static str,
        content: impl Fn(&mut Ui, &mut Application) + Send + Sync + 'static,
    ) {
        if let Ok(mut modal) = self.modal.lock() {
            if !modal.contains_modal(id) {
                modal.stack.push(Modal {
                    id,
                    content: Arc::new(content),
                });
            }
        }
    }

    fn show_message_modal(&self, id: &'static str, title: String, message: String) {
        self.show_custom_modal(id, move |ui, app| {
            ui.set_width(320.0);

            ui.scope(|ui| {
                let mut style = get_current_style();

                style.text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(24.0, egui::FontFamily::Proportional),
                );

                style.visuals.override_text_color = Some(Color::WHITE);
                style.visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color::WHITE);

                ui.set_style(style);

                ui.vertical_centered(|ui| {
                    ui.label(title.clone());
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.x);
            });

            ui.label(message.clone());

            ui.add_space(ui.spacing().item_spacing.x * 2.5);

            ui.vertical_centered(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Ok"))
                    .clicked()
                {
                    app.close_modal();
                }
            });
        });
    }

    /// If you want to show another modal from callbacks, you need to set the `auto_close` to false
    /// and close the modal manually, because `auto_close` closes the very latest modal
    fn show_yes_no_modal(
        &self,
        id: &'static str,
        title: String,
        question: String,
        on_confirm: impl Fn(&mut Application) + Send + Sync + 'static,
        on_deny: impl Fn(&mut Application) + Send + Sync + 'static,
        auto_close: bool,
    ) {
        self.show_custom_modal(id, move |ui, app| {
            ui.set_width(310.0);

            ui.scope(|ui| {
                let mut style = get_current_style();

                style.text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(24.0, egui::FontFamily::Proportional),
                );

                style.visuals.override_text_color = Some(Color::WHITE);
                style.visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, Color::WHITE);

                ui.set_style(style);

                ui.vertical_centered(|ui| {
                    ui.label(title.clone());
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.x);
            });

            ui.label(question.clone());

            ui.add_space(ui.spacing().item_spacing.x * 2.5);

            ui.horizontal_top(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Yes"))
                    .clicked()
                {
                    on_confirm(app);

                    if auto_close {
                        app.close_modal();
                    }
                }
                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("No"))
                    .clicked()
                {
                    on_deny(app);

                    if auto_close {
                        app.close_modal();
                    }
                }
            });
        });
    }

    fn close_modal(&self) {
        self.close_modals(1);
    }

    /// Closes the last number of modals, 1 means only last modal
    fn close_modals(&self, number: usize) {
        if let Ok(mut modal) = self.modal.lock() {
            modal.close_last_modals(number);
        }
    }

    fn show_not_paired_error(&self) {
        self.show_message_modal(
            "device-not-paired",
            "Unavailable".to_string(),
            "Device must be paired for this action!".to_string(),
        );
    }

    fn create_update_layout(&mut self, name: String, size: (f32, f32)) {
        if let Some(config) = &mut self.config {
            let components = if let Some(layout) = &config.layout {
                layout.components.clone()
            } else {
                Default::default()
            };

            let layout = Layout {
                name,
                size,
                components,
            };

            update_config_and_server(config, |c| {
                c.layout = Some(layout);
            });
        }
    }

    fn draw_new_layout_button(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() / 2.0) - 96.0);

            ui.scope(|ui| {
                let mut style = get_current_style();

                style.text_styles.insert(
                    egui::TextStyle::Button,
                    egui::FontId::new(64.0, egui::FontFamily::Proportional),
                );

                style.visuals.widgets.inactive.rounding = 24.0.into();
                style.visuals.widgets.hovered.rounding = 24.0.into();
                style.visuals.widgets.active.rounding = 24.0.into();

                style.visuals.widgets.inactive.weak_bg_fill = Color::OVERLAY0;
                style.visuals.widgets.hovered.weak_bg_fill = Color::OVERLAY1;
                style.visuals.widgets.hovered.bg_stroke.width = 2.0;
                style.visuals.widgets.active.weak_bg_fill = Color::OVERLAY0;

                ui.set_style(style);

                let new_layout_button = ui.add_sized((128.0, 128.0), egui::Button::new("+"));

                if new_layout_button.clicked() {
                    self.open_create_update_layout_modal();
                }
            });
        });
    }

    fn draw_layout(&mut self, ctx: &Context) {
        let mut layout_size = (0.0, 0.0);

        if let Some(config) = &self.config {
            if let Some(l) = &config.layout {
                layout_size = l.size;
            }
        }

        if layout_size.0 < 1.0 || layout_size.1 < 1.0 {
            return;
        }

        egui::Window::new("Layout")
            .movable(false)
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .hscroll(true)
            .vscroll(true)
            .fixed_size((layout_size.0, layout_size.1))
            .current_pos((
                (ctx.screen_rect().max.x - layout_size.0) / 2.0,
                (ctx.screen_rect().max.y - layout_size.1) / 2.0,
            ))
            .frame(egui::Frame {
                fill: Color::OVERLAY0,
                rounding: 8.0.into(),
                shadow: egui::Shadow {
                    offset: egui::vec2(0.0, 4.0),
                    blur: 8.0,
                    spread: 2.0,
                    color: Color::OVERLAY1.gamma_multiply(0.5),
                },
                stroke: {
                    if self.editing_layout {
                        egui::Stroke {
                            width: 2.0,
                            color: Color::RED,
                        }
                    } else {
                        egui::Stroke::default()
                    }
                },
                ..egui::Frame::default()
            })
            .show(ctx, |ui| {
                let layout = if let Some(config) = &mut self.config {
                    match &mut config.layout {
                        Some(l) => l,
                        None => {
                            return;
                        }
                    }
                } else {
                    return;
                };

                if layout.components.is_empty() {
                    ui.label("You haven't add any components yet!");

                    return;
                }

                for component in layout.components.clone() {
                    let mut response = None;

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
                    let _id = kind_id.get(1).unwrap_or(&"0").parse::<u8>().unwrap_or(0);
                    let value = match self.components.get(&component.0.to_string()) {
                        Some(v) => String::from(v),
                        None => String::new(),
                    };
                    let label = &component.1.label;
                    let position: Pos2 = component.1.position.into();
                    let mut size = (0.0, 0.0);
                    let scale = component.1.scale;
                    let style = component.1.style;

                    match kind {
                        ComponentKind::None => (),
                        ComponentKind::Button => {
                            size = self.component_button_size;

                            let button = self.draw_button(
                                ui,
                                label,
                                position,
                                size,
                                scale,
                                value.parse::<i8>().unwrap_or(0),
                            );

                            response = Some(button);
                        }
                        ComponentKind::LED => {
                            size = self.component_led_size;

                            let led = self.draw_led(ui, label, position, size, scale, {
                                // TODO: Actually return a value!
                                let r = 255;
                                let g = 0;
                                let b = 0;

                                (r, g, b)
                            });

                            response = Some(led);
                        }
                        ComponentKind::Potentiometer => {
                            size = self.component_potentiometer_size;

                            let potentiometer = self.draw_potentiometer(
                                ui,
                                label,
                                position,
                                size,
                                scale,
                                value.parse::<u8>().unwrap_or(0),
                                style,
                            );

                            response = Some(potentiometer);
                        }
                        ComponentKind::Joystick => {
                            size = self.component_joystick_size;

                            let joystick = self.draw_joystick(ui, label, position, size, scale, {
                                match value.split_once(SERIAL_MESSAGE_INNER_SEP) {
                                    Some((value_x, value_y)) => (
                                        value_x.parse::<f32>().unwrap_or(0.0),
                                        value_y.parse::<f32>().unwrap_or(0.0),
                                    ),
                                    None => (0.0, 0.0),
                                }
                            });

                            response = Some(joystick);
                        }
                        ComponentKind::RotaryEncoder => {
                            size = self.component_rotary_encoder_size;

                            let rotary_encoder =
                                self.draw_rotary_encoder(ui, label, position, size, scale);

                            response = Some(rotary_encoder);
                        }
                        ComponentKind::Display => {
                            size = (
                                self.component_display_size.0 * DASHBOARD_DISAPLY_PIXEL_SIZE,
                                self.component_display_size.1 * DASHBOARD_DISAPLY_PIXEL_SIZE,
                            );

                            let display = self.draw_display(
                                ui,
                                position,
                                (
                                    (size.0 / DASHBOARD_DISAPLY_PIXEL_SIZE) as usize,
                                    (size.1 / DASHBOARD_DISAPLY_PIXEL_SIZE) as usize,
                                ),
                                scale,
                                {
                                    match hex_bytes_string_to_vec(&label) {
                                        Ok(bytes) => bytes,
                                        Err(_) => vec![],
                                    }
                                },
                            );

                            response = Some(display);
                        }
                    }

                    if !self.editing_layout {
                        continue;
                    }

                    let response = if let Some(r) = response {
                        r
                    } else {
                        continue;
                    };

                    if response.drag_started() {
                        let mouse_pos = if let Some(mouse) = ui.input(|i| i.pointer.latest_pos()) {
                            mouse
                        } else {
                            continue;
                        };

                        self.dragged_component_offset =
                            ((mouse_pos.x - position.x), (mouse_pos.y - position.y));
                    }

                    if !response.dragged() {
                        continue;
                    }

                    let mouse_pos = if let Some(mouse) = ui.input(|i| i.pointer.latest_pos()) {
                        mouse
                    } else {
                        continue;
                    };

                    if let Some(config) = &mut self.config {
                        let mut new_position = (
                            mouse_pos.x - self.dragged_component_offset.0,
                            mouse_pos.y - self.dragged_component_offset.1,
                        );

                        // Prevent components from getting outside of the layout borders
                        if new_position.0 < 0.0 {
                            new_position.0 = 0.0;
                        }
                        if new_position.1 < 0.0 {
                            new_position.1 = 0.0;
                        }
                        if new_position.0 + (size.0 * scale) > layout_size.0 {
                            new_position.0 = layout_size.0 - (size.0 * scale);
                        }
                        if new_position.1 + (size.1 * scale) > layout_size.1 {
                            new_position.1 = layout_size.1 - (size.1 * scale);
                        }

                        // Apply grid snapping
                        if self.layout_grid.0 {
                            let grid_size = self.layout_grid.1;

                            new_position.0 = (new_position.0 / grid_size).round() * grid_size;
                            new_position.1 = (new_position.1 / grid_size).round() * grid_size;
                        }

                        match &mut config.layout {
                            Some(l) => {
                                if let Some(c) = l.components.get_mut(&component.0) {
                                    c.position = new_position;
                                }
                            }
                            None => continue,
                        }
                    }
                }
            });
    }

    fn save_current_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-save-elements-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for saving the elements!".to_string(),
                );

                return;
            }

            update_config_and_server(config, |_| {});

            self.editing_layout = false;
        } else {
            self.show_message_modal(
                "layout-save-elements-no-config",
                "Error".to_string(),
                "There was a problem was saving your layout to config!".to_string(),
            );

            return;
        }
    }

    fn add_button_to_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-add-element-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for adding elements!".to_string(),
                );

                return;
            }

            let kind = ComponentKind::Button.to_string();

            if let Some(layout) = &mut config.layout {
                let mut highest_id = 0;

                for component in layout.components.iter_mut() {
                    match component.0.split_once(SERIAL_MESSAGE_SEP) {
                        Some((component_kind, component_id)) => {
                            if component_kind != kind {
                                continue;
                            }

                            let current_id = component_id.parse::<u8>().unwrap_or(0);

                            if current_id > highest_id {
                                highest_id = current_id;
                            }
                        }
                        None => continue,
                    }
                }

                if highest_id >= u8::MAX {
                    self.show_message_modal(
                        "layout-add-element-too-many",
                        "Error".to_string(),
                        format!(
                            "You cannot add more than {} \"{}\" to you layout!",
                            u8::MAX,
                            kind
                        ),
                    );

                    return;
                }

                let new_id = highest_id + 1;

                let (component_global_id, component) =
                    Component::new_button(new_id, format!("{} {}", kind, new_id), {
                        let x = (layout.size.0 - self.component_button_size.0) / 2.0;
                        let y = (layout.size.1 - self.component_button_size.1) / 2.0;

                        (x, y)
                    });

                layout
                    .components
                    .insert(component_global_id.clone(), component);

                for profile in config.profiles.iter_mut() {
                    profile
                        .interactions
                        .insert(component_global_id.clone(), Interaction::default());
                }
            }
        }
    }

    fn add_led_to_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-add-element-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for adding elements!".to_string(),
                );

                return;
            }

            let kind = ComponentKind::LED.to_string();

            if let Some(layout) = &mut config.layout {
                let mut highest_id = 0;

                for component in layout.components.iter_mut() {
                    match component.0.split_once(SERIAL_MESSAGE_SEP) {
                        Some((component_kind, component_id)) => {
                            if component_kind != kind {
                                continue;
                            }

                            let current_id = component_id.parse::<u8>().unwrap_or(0);

                            if current_id > highest_id {
                                highest_id = current_id;
                            }
                        }
                        None => continue,
                    }
                }

                if highest_id >= u8::MAX {
                    self.show_message_modal(
                        "layout-add-element-too-many",
                        "Error".to_string(),
                        format!(
                            "You cannot add more than {} \"{}\" to you layout!",
                            u8::MAX,
                            kind
                        ),
                    );

                    return;
                }

                let new_id = highest_id + 1;

                let (component_global_id, component) =
                    Component::new_led(new_id, format!("{} {}", kind, new_id), {
                        let x = (layout.size.0 - self.component_led_size.0) / 2.0;
                        let y = (layout.size.1 - self.component_led_size.1) / 2.0;

                        (x, y)
                    });

                layout
                    .components
                    .insert(component_global_id.clone(), component);

                for profile in config.profiles.iter_mut() {
                    profile
                        .interactions
                        .insert(component_global_id.clone(), Interaction::default());
                }
            }
        }
    }

    fn add_potentiometer_to_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-add-element-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for adding elements!".to_string(),
                );

                return;
            }

            let kind = ComponentKind::Potentiometer.to_string();

            if let Some(layout) = &mut config.layout {
                let mut highest_id = 0;

                for component in layout.components.iter_mut() {
                    match component.0.split_once(SERIAL_MESSAGE_SEP) {
                        Some((component_kind, component_id)) => {
                            if component_kind != kind {
                                continue;
                            }

                            let current_id = component_id.parse::<u8>().unwrap_or(0);

                            if current_id > highest_id {
                                highest_id = current_id;
                            }
                        }
                        None => continue,
                    }
                }

                if highest_id >= u8::MAX {
                    self.show_message_modal(
                        "layout-add-element-too-many",
                        "Error".to_string(),
                        format!(
                            "You cannot add more than {} \"{}\" to you layout!",
                            u8::MAX,
                            kind
                        ),
                    );

                    return;
                }

                let new_id = highest_id + 1;

                let (component_global_id, component) =
                    Component::new_potentiometer(new_id, format!("{} {}", kind, new_id), {
                        let x = (layout.size.0 - self.component_potentiometer_size.0) / 2.0;
                        let y = (layout.size.1 - self.component_potentiometer_size.1) / 2.0;

                        (x, y)
                    });

                layout
                    .components
                    .insert(component_global_id.clone(), component);

                for profile in config.profiles.iter_mut() {
                    profile
                        .interactions
                        .insert(component_global_id.clone(), Interaction::default());
                }
            }
        }
    }

    fn add_joystick_to_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-add-element-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for adding elements!".to_string(),
                );

                return;
            }

            let kind = ComponentKind::Joystick.to_string();

            if let Some(layout) = &mut config.layout {
                let mut highest_id = 0;

                for component in layout.components.iter_mut() {
                    match component.0.split_once(SERIAL_MESSAGE_SEP) {
                        Some((component_kind, component_id)) => {
                            if component_kind != kind {
                                continue;
                            }

                            let current_id = component_id.parse::<u8>().unwrap_or(0);

                            if current_id > highest_id {
                                highest_id = current_id;
                            }
                        }
                        None => continue,
                    }
                }

                if highest_id >= u8::MAX {
                    self.show_message_modal(
                        "layout-add-element-too-many",
                        "Error".to_string(),
                        format!(
                            "You cannot add more than {} \"{}\" to you layout!",
                            u8::MAX,
                            kind
                        ),
                    );

                    return;
                }

                let new_id = highest_id + 1;

                let (component_global_id, component) =
                    Component::new_joystick(new_id, format!("{} {}", kind, new_id), {
                        let x = (layout.size.0 - self.component_joystick_size.0) / 2.0;
                        let y = (layout.size.1 - self.component_joystick_size.1) / 2.0;

                        (x, y)
                    });

                layout
                    .components
                    .insert(component_global_id.clone(), component);

                for profile in config.profiles.iter_mut() {
                    profile
                        .interactions
                        .insert(component_global_id.clone(), Interaction::default());
                }
            }
        }
    }

    fn add_rotary_encoder_to_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-add-element-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for adding elements!".to_string(),
                );

                return;
            }

            let kind = ComponentKind::RotaryEncoder.to_string();

            if let Some(layout) = &mut config.layout {
                let mut highest_id = 0;

                for component in layout.components.iter_mut() {
                    match component.0.split_once(SERIAL_MESSAGE_SEP) {
                        Some((component_kind, component_id)) => {
                            if component_kind != kind {
                                continue;
                            }

                            let current_id = component_id.parse::<u8>().unwrap_or(0);

                            if current_id > highest_id {
                                highest_id = current_id;
                            }
                        }
                        None => continue,
                    }
                }

                if highest_id >= u8::MAX {
                    self.show_message_modal(
                        "layout-add-element-too-many",
                        "Error".to_string(),
                        format!(
                            "You cannot add more than {} \"{}\" to you layout!",
                            u8::MAX,
                            kind
                        ),
                    );

                    return;
                }

                let new_id = highest_id + 1;

                let (component_global_id, component) =
                    Component::new_rotary_encoder(new_id, format!("{} {}", kind, new_id), {
                        let x = (layout.size.0 - self.component_rotary_encoder_size.0) / 2.0;
                        let y = (layout.size.1 - self.component_rotary_encoder_size.1) / 2.0;

                        (x, y)
                    });

                layout
                    .components
                    .insert(component_global_id.clone(), component);

                for profile in config.profiles.iter_mut() {
                    profile
                        .interactions
                        .insert(component_global_id.clone(), Interaction::default());
                }
            }
        }
    }

    fn add_display_to_layout(&mut self) {
        if let Some(config) = &mut self.config {
            if config.layout.is_none() {
                self.show_message_modal(
                    "layout-add-element-no-layout",
                    "Error".to_string(),
                    "Could not find any layout for adding elements!".to_string(),
                );

                return;
            }

            let kind = ComponentKind::Display.to_string();

            if let Some(layout) = &mut config.layout {
                let mut highest_id = 0;

                for component in layout.components.iter_mut() {
                    match component.0.split_once(SERIAL_MESSAGE_SEP) {
                        Some((component_kind, component_id)) => {
                            if component_kind != kind {
                                continue;
                            }

                            let current_id = component_id.parse::<u8>().unwrap_or(0);

                            if current_id > highest_id {
                                highest_id = current_id;
                            }
                        }
                        None => continue,
                    }
                }

                if highest_id >= u8::MAX {
                    self.show_message_modal(
                        "layout-add-element-too-many",
                        "Error".to_string(),
                        format!(
                            "You cannot add more than {} \"{}\" to you layout!",
                            u8::MAX,
                            kind
                        ),
                    );

                    return;
                }

                let new_id = highest_id + 1;

                let (component_global_id, component) = Component::new_display(
                    new_id,
                    HOME_IMAGE_DEFAULT_BYTES.to_string(), // Default image
                    {
                        let x = (layout.size.0
                            - (self.component_display_size.0 * DASHBOARD_DISAPLY_PIXEL_SIZE))
                            / 2.0;
                        let y = (layout.size.1
                            - (self.component_display_size.1 * DASHBOARD_DISAPLY_PIXEL_SIZE))
                            / 2.0;

                        (x, y)
                    },
                );

                layout
                    .components
                    .insert(component_global_id.clone(), component);

                for profile in config.profiles.iter_mut() {
                    profile
                        .interactions
                        .insert(component_global_id.clone(), Interaction::default());
                }
            }
        }
    }

    fn create_update_profile(&mut self, name: String) -> bool {
        if let Some(config) = &mut self.config {
            let is_updating = !self.last_profile_name.is_empty();

            if is_updating {
                update_config_and_server(config, |c| {
                    for profile in c.profiles.iter_mut() {
                        if profile.name != self.last_profile_name {
                            continue;
                        }

                        profile.name = name.clone();

                        break;
                    }
                });

                request_send_serial("refresh_device").ok();

                return true;
            }

            if config.does_profile_exist(&name) {
                return false;
            }

            let components = if let Some(layout) = &config.layout {
                layout.components.clone()
            } else {
                Default::default()
            };

            let new_profile = Profile {
                name,
                interactions: {
                    let mut interactions: HashMap<String, Interaction> = Default::default();

                    for (component_key, _component) in components.iter() {
                        interactions.insert(component_key.clone(), Interaction::default());
                    }

                    interactions
                },
            };

            update_config_and_server(config, |c| {
                c.profiles.push(new_profile);
            });

            request_send_serial("refresh_device").ok();

            return true;
        }

        return false;
    }

    fn delete_profile(&mut self, profile_name: &String) -> Result<String, String> {
        if let Some(config) = &mut self.config {
            let mut profile_index = -1;

            for (index, profile) in config.profiles.iter().enumerate() {
                if profile.name == *profile_name {
                    profile_index = index as i8;

                    break;
                }
            }

            if profile_index < 0 {
                return Err(format!("Could not find \"{}\" profile!", profile_name));
            }

            if profile_index == 0 {
                return Err(
                    "This profile is essential for your device and cannot be deleted.".to_string(),
                );
            }

            update_config_and_server(config, |c| {
                let index = profile_index as usize;
                let current_profile_name = c.profiles[c.settings.current_profile].name.clone();

                // If user is deleting the currently selected profile, reset to `internal`
                if c.settings.current_profile == index {
                    c.settings.current_profile = 0;
                }

                c.profiles.remove(index);

                // On deleting a profile we check if the order changed
                // For example if the current profile is `2` and user
                // tries to delete profile `1`, now profile `2` is `1`
                // and needs to be set properly to its new position
                for (index, profile) in c.profiles.iter().enumerate() {
                    if profile.name == current_profile_name {
                        c.settings.current_profile = index;

                        break;
                    }
                }
            });

            request_send_serial("refresh_device").ok();

            return Ok(format!(
                "Profile \"{}\" was successfully removed.",
                profile_name
            ));
        }

        Err("There was an unknown error!\nPlease try again.".to_string())
    }

    fn draw_status_indicator(&mut self, ui: &mut Ui) {
        use egui::*;

        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            let paired_status_color = if self.server_data.is_device_paired {
                Color::GREEN
            } else {
                Color::RED
            };

            let indicator = status_indicator(
                "device-paired-status-indicator",
                ui,
                paired_status_color,
                48.0,
            );

            let indicator_hovered = indicator.hovered() || indicator.contains_pointer();

            let panel_position_x = animate_value(
                ui.ctx(),
                "paired-status-indicator-panel-position",
                self.paired_status_panel.0,
                0.25,
            );

            const PANEL_OPENED_X: f32 = 0.0;
            const PANEL_CLOSED_X: f32 = -16.0;

            let panel_disabled = { panel_position_x == PANEL_CLOSED_X };

            let panel_opacity = animate_value(
                ui.ctx(),
                "paired-status-indicator-panel-opacity",
                self.paired_status_panel.1,
                0.2,
            );

            let panel_hovered = {
                let rect = ui.cursor();
                let position = pos2(rect.min.x + panel_position_x, rect.max.y);
                let padding = ui.style().spacing.button_padding;
                let size = pos2(212.0, 64.0 - (padding.y / 2.0) - 1.0); // I know! I hate myself too

                let rect =
                    Rect::from_min_size((position.x, position.y - size.y).into(), size.to_vec2());

                if panel_disabled {
                    ui.disable();
                }

                let response = ui
                    .allocate_new_ui(
                        UiBuilder::new()
                            .max_rect(rect)
                            .layout(Layout::right_to_left(Align::Center)),
                        |ui| {
                            ui.scope(|ui| {
                                let mut style = get_current_style();

                                style.text_styles.insert(
                                    egui::TextStyle::Body,
                                    egui::FontId::new(16.0, egui::FontFamily::Proportional),
                                );

                                ui.set_style(style);

                                Frame::default()
                                    .fill(Color::OVERLAY0.gamma_multiply(panel_opacity))
                                    .rounding(ui.visuals().widgets.noninteractive.rounding)
                                    .inner_margin(padding)
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            let device_status_color;
                                            let service_status_color;

                                            let device_status_text = format!(
                                                "Device is{} connected!",
                                                if self.server_data.is_device_paired {
                                                    device_status_color =
                                                        Color::GREEN.gamma_multiply(panel_opacity);

                                                    ""
                                                } else {
                                                    device_status_color =
                                                        Color::RED.gamma_multiply(panel_opacity);

                                                    " NOT"
                                                }
                                            );
                                            let service_status_text = format!(
                                                "Service app is{} running!",
                                                if self.server_data.is_client_connected {
                                                    service_status_color =
                                                        Color::GREEN.gamma_multiply(panel_opacity);

                                                    ""
                                                } else {
                                                    service_status_color =
                                                        Color::RED.gamma_multiply(panel_opacity);

                                                    " NOT"
                                                }
                                            );

                                            ui.label(
                                                RichText::new(device_status_text)
                                                    .color(device_status_color),
                                            );
                                            ui.label(
                                                RichText::new(service_status_text)
                                                    .color(service_status_color),
                                            );
                                        });
                                    });
                            });
                        },
                    )
                    .response;

                response.hovered() || (!panel_disabled && response.contains_pointer())
            };

            if indicator_hovered || panel_hovered {
                self.paired_status_panel = (PANEL_OPENED_X, 1.0);
            } else {
                self.paired_status_panel = (PANEL_CLOSED_X, 0.0);
            }
        });
    }

    fn draw_button(
        &mut self,
        ui: &mut Ui,
        _label: &String,
        relative_position: Pos2, /* relative to window position */
        size: (f32, f32),
        scale: f32,
        _value: i8,
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = Pos2::new(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let scaled_size = (size.0 * scale, size.1 * scale);
        let rect = Rect::from_min_size(position, (size.0 * scale, size.1 * scale).into());

        draw_rect_shadow(
            ui,
            rect,
            ui.style().visuals.menu_rounding.nw,
            self.global_shadow,
            (0.0, 0.0),
        );

        ui.put(rect, Button::new(scaled_size))
    }

    fn draw_led(
        &self,
        ui: &mut Ui,
        _label: &String,
        relative_position: Pos2, /* relative to window position */
        size: (f32, f32),
        scale: f32,
        value: (u8, u8, u8),
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let scaled_size = (size.0 * scale, size.1 * scale);
        let rect = Rect::from_min_size(position, scaled_size.into());

        draw_rect_shadow(
            ui,
            Rect::from_center_size(
                rect.center(),
                (scaled_size.0 * 0.55, scaled_size.1 * 0.55).into(),
            ),
            ui.style().visuals.menu_rounding.nw,
            self.global_shadow,
            (0.0, 0.0),
        );

        ui.put(rect, LED::new(value, scaled_size))
    }

    fn draw_potentiometer(
        &self,
        ui: &mut Ui,
        label: &String,
        relative_position: Pos2, /* relative to window position */
        size: (f32, f32),
        scale: f32,
        value: u8,
        style: u8,
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let scaled_size = (size.0 * scale, size.1 * scale);
        let rect = Rect::from_min_size(position, scaled_size.into());

        let value = value as f32;

        draw_circle_shadow(
            ui,
            rect.center(),
            scaled_size.0 / 2.0,
            self.global_shadow,
            (0.0, 0.0),
        );

        ui.put(
            rect,
            Potentiometer::new(
                format!("potentiometer-{:?}-value", egui::Id::new(label)),
                value,
                scaled_size,
            )
            .style(style),
        )
    }

    fn draw_joystick(
        &self,
        ui: &mut Ui,
        _label: &String,
        relative_position: Pos2, /* relative to window position */
        size: (f32, f32),
        scale: f32,
        value: (f32, f32),
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let scaled_size = (size.0 * scale, size.1 * scale);
        let rect = Rect::from_min_size(position, scaled_size.into());

        draw_circle_shadow(
            ui,
            rect.center(),
            scaled_size.0 / 2.0,
            self.global_shadow,
            (0.0, 0.0),
        );

        ui.put(rect, Joystick::new(value, scaled_size))
    }

    fn draw_rotary_encoder(
        &self,
        ui: &mut Ui,
        _label: &String,
        relative_position: Pos2, /* relative to window position */
        size: (f32, f32),
        scale: f32,
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let scaled_size = (size.0 * scale, size.1 * scale);
        let rect = Rect::from_min_size(position, scaled_size.into());

        draw_circle_shadow(
            ui,
            rect.center(),
            scaled_size.0 / 2.0,
            self.global_shadow,
            (0.0, 0.0),
        );

        ui.put(rect, RotaryEncoder::new(scaled_size))
    }

    fn draw_display(
        &self,
        ui: &mut Ui,
        relative_position: Pos2, /* relative to window position */
        size: (usize, usize),
        scale: f32,
        value: Vec<u8>,
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = egui::pos2(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let rect = Rect::from_min_size(
            position,
            (
                size.0 as f32 * DASHBOARD_DISAPLY_PIXEL_SIZE * scale,
                size.1 as f32 * DASHBOARD_DISAPLY_PIXEL_SIZE * scale,
            )
                .into(),
        );

        draw_rect_shadow(
            ui,
            rect,
            ui.style().visuals.window_rounding.nw,
            self.global_shadow,
            (0.0, 0.0),
        );

        ui.put(
            rect,
            GLCD::new(
                size,
                DASHBOARD_DISAPLY_PIXEL_SIZE,
                Color::BLACK,
                Color::WHITE,
                value,
                (HOME_IMAGE_WIDTH, HOME_IMAGE_HEIGHT),
                (
                    (size.0 - HOME_IMAGE_WIDTH) / 2,
                    (size.1 - HOME_IMAGE_HEIGHT) / 2,
                ), // Center icon
            )
            .scale(scale),
        )
    }

    fn detect_components(&mut self) -> Result<String, String> {
        let config = match &self.config {
            Some(config) => config,
            None => return Err("There was an error while accessing the config!".to_string()),
        };

        let current_layout = if let Some(layout) = &config.layout {
            (layout.name.clone(), layout.size)
        } else {
            let error =
                "Access violation: Tried to access layout without creating it first!".to_string();

            log_error!("{}", error);

            return Err(error);
        };

        let mut layout = Layout {
            name: current_layout.0,
            size: current_layout.1,
            ..Default::default()
        };

        let mut interactions: HashMap<String, Interaction> = Default::default();

        let mut index = 0;

        let component_size = self.component_button_size;
        let max_per_column = (layout.size.1 / component_size.1).trunc();
        let spacing =
            (layout.size.1 % component_size.1) / (max_per_column + 1.0/* bottom spacing */);

        let mut current_x = spacing;
        let mut current_y = spacing;

        // Buttons
        for (button_id, _button_normal, _button_mod) in self.get_buttons() {
            if current_y + (component_size.1 * 2.0) > layout.size.1 {
                current_x += component_size.0 + spacing;
                current_y = spacing;
            } else {
                if index > 0 {
                    current_y += component_size.1 + spacing;
                }
            }

            index += 1;

            let component_global_id = format!("{}:{}", ComponentKind::Button, button_id);
            let button_name = format!("{} {}", ComponentKind::Button, button_id);

            let layout_button = Component::new_button(
                button_id,
                button_name,
                (current_x.round(), current_y.round()),
            );

            layout.components.insert(layout_button.0, layout_button.1);

            interactions.insert(component_global_id, Interaction::default());
        }

        let component_size = self.component_potentiometer_size;
        let max_per_column = (layout.size.1 / component_size.1).trunc();
        let spacing =
            (layout.size.1 % component_size.1) / (max_per_column + 1.0/* bottom spacing */);

        // Potentiometers
        for (potentiometer_id, _potentiometer_value) in self.get_potentiometers() {
            if current_y + (component_size.1 * 2.0) > layout.size.1 {
                current_x += component_size.0 + spacing;
                current_y = spacing;
            } else {
                if index > 0 {
                    current_y += component_size.1 + spacing;
                }
            }

            index += 1;

            let component_global_id = format!("{}:{}", ComponentKind::Button, potentiometer_id);
            let potentiometer_name = format!("{} {}", ComponentKind::Button, potentiometer_id);

            let layout_potentiometer = Component::new_potentiometer(
                potentiometer_id,
                potentiometer_name,
                (current_x.round(), current_y.round()),
            );

            layout
                .components
                .insert(layout_potentiometer.0, layout_potentiometer.1);

            interactions.insert(component_global_id, Interaction::default());
        }

        if index < 1 {
            return Err("Couldn't find any componenets!\n\
                Make sure your device is connected."
                .to_string());
        }

        if let Some(config) = &mut self.config {
            update_config_and_server(config, |c| {
                c.layout = Some(layout);

                for profile in c.profiles.iter_mut() {
                    profile.interactions = interactions.clone();
                }
            });

            request_send_serial("refresh_device").ok();

            return Ok("Detected componenets were added to your layout.".to_string());
        }

        Err("Unknown problem occured!".to_string())
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
        // TODO: REMOVE THESE TEST VALUES
        let buttons_string = "1|97|98|2|99|100|3|101|102|4|103|104|5|105|106";
        //let buttons_string = self.server_data.raw_layout.0.clone();

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
        // TODO: REMOVE THESE TEST VALUES
        let potentiometers_string = "1|25|2|45|3|12|4|99|5|75";
        //let potentiometers_string = &self.server_data.raw_layout.1;

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

    fn draw_debug_panel(&mut self, ctx: &Context) {
        use egui::*;

        let mut port_name = String::new();
        let mut profiles = Default::default();
        let mut current_profile = String::new();

        if let Some(config) = &self.config {
            port_name = config.settings.port_name.clone();
            profiles = config.profiles.clone();
            current_profile = config.settings.current_profile.to_string();
        }

        Window::new("Debug")
            .default_pos((0.0, 0.0))
            .default_open(true)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Add Button").clicked() {
                            self.add_button_to_layout();
                        }
                        if ui.button("Add LED").clicked() {
                            self.add_led_to_layout();
                        }
                        if ui.button("Add Potentiometer").clicked() {
                            self.add_potentiometer_to_layout();
                        }
                        if ui.button("Add Joystick").clicked() {
                            self.add_joystick_to_layout();
                        }
                        if ui.button("Add Rotary Encoder").clicked() {
                            self.add_rotary_encoder_to_layout();
                        }
                        if ui.button("Add Display").clicked() {
                            self.add_display_to_layout();
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Save layout").clicked() {
                            self.save_current_layout();
                        }

                        ui.checkbox(&mut self.layout_grid.0, "Snap to grid")
                            .context_menu(|ui| self.open_grid_context_menu(ui));
                    });
                });

                let xbm_data = vec![
                    0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0x00, 0x00, 0xfe, 0x01, 0x00, 0xfc, 0x00,
                    0xe0, 0xff, 0x1f, 0x00, 0xfc, 0x00, 0xf8, 0xff, 0x7f, 0x00, 0xfc, 0x00, 0xfc,
                    0xff, 0xff, 0x00, 0xfc, 0x00, 0xff, 0xff, 0xff, 0x03, 0xfc, 0x80, 0xff, 0xff,
                    0xff, 0x07, 0xfc, 0xc0, 0xff, 0x03, 0xff, 0x0f, 0xfc, 0xe0, 0xff, 0x00, 0xfc,
                    0x1f, 0xfc, 0xe0, 0x7f, 0x00, 0xf8, 0x1f, 0xfc, 0xf0, 0x3f, 0x00, 0xf0, 0x3f,
                    0xfc, 0xf8, 0x3f, 0x00, 0xf0, 0x7f, 0xfc, 0xf8, 0x1f, 0x00, 0xe0, 0x7f, 0xfc,
                    0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc, 0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc, 0xfc,
                    0x1f, 0x00, 0xe0, 0xff, 0xfc, 0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc, 0xfe, 0x1f,
                    0x00, 0xe0, 0xff, 0xfd, 0xfe, 0x3f, 0x00, 0xf0, 0xff, 0xfd, 0xfe, 0x3f, 0x00,
                    0xf0, 0xff, 0xfd, 0xfe, 0x7f, 0x00, 0xf8, 0xff, 0xfd, 0xfe, 0xff, 0x00, 0xfc,
                    0xff, 0xfd, 0xfe, 0xff, 0x03, 0xff, 0xff, 0xfd, 0xfe, 0xff, 0xff, 0xff, 0xff,
                    0xfd, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xfd, 0xfc, 0xff, 0xff, 0xff, 0xff, 0xfc,
                    0xfc, 0xff, 0x00, 0xfc, 0xff, 0xfc, 0xfc, 0x0f, 0x00, 0xc0, 0xff, 0xfc, 0xfc,
                    0x01, 0x00, 0x00, 0xfe, 0xfc, 0xf8, 0x00, 0x00, 0x00, 0x7c, 0xfc, 0x78, 0x00,
                    0x00, 0x00, 0x78, 0xfc, 0x70, 0x00, 0x00, 0x00, 0x38, 0xfc, 0xe0, 0x00, 0x00,
                    0x00, 0x1c, 0xfc, 0xe0, 0x01, 0x00, 0x00, 0x1e, 0xfc, 0xc0, 0x03, 0x00, 0x00,
                    0x0f, 0xfc, 0x80, 0x0f, 0x00, 0xc0, 0x07, 0xfc, 0x00, 0x7f, 0x00, 0xf8, 0x03,
                    0xfc, 0x00, 0xfc, 0x03, 0xff, 0x00, 0xfc, 0x00, 0xf8, 0xff, 0x7f, 0x00, 0xfc,
                    0x00, 0xe0, 0xff, 0x1f, 0x00, 0xfc, 0x00, 0x00, 0xfe, 0x01, 0x00, 0xfc, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0xfc,
                ];

                ui.add(GLCD::new(
                    (128, 64),
                    3.0,
                    Color::BLACK,
                    Color::WHITE,
                    xbm_data,
                    (42, 42),
                    ((128 - 42) / 2, (64 - 42) / 2),
                ));

                ui.add(super::widgets::Button::new(self.component_button_size));

                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        Potentiometer::new(
                            format!("{:?}", Id::new("test-potentiometer")),
                            self.test_potentiometer_value,
                            (100.0, 100.0),
                        )
                        .style(self.test_potentiometer_style),
                    );

                    ui.horizontal_centered(|ui| {
                        ui.label("Value: ");
                        ui.add(DragValue::new(&mut self.test_potentiometer_value));

                        ui.add_space(ui.style().spacing.item_spacing.x);

                        ui.label("Style: ");
                        ui.add(DragValue::new(&mut self.test_potentiometer_style).speed(1));
                    });
                });

                ui.horizontal_wrapped(|ui| {
                    ui.add(Joystick::new(
                        self.test_joystick_value,
                        self.component_joystick_size,
                    ));

                    ui.horizontal_centered(|ui| {
                        ui.label("X: ");
                        ui.add(DragValue::new(&mut self.test_joystick_value.0).speed(0.1));

                        ui.add_space(ui.style().spacing.item_spacing.x);

                        ui.label("Y: ");
                        ui.add(DragValue::new(&mut self.test_joystick_value.1).speed(0.1));
                    });
                });

                ui.add(RotaryEncoder::new(self.component_rotary_encoder_size));

                ui.add(LED::new((255, 181, 0), self.component_led_size));

                ui.group(|ui| {
                    ui.label("Theme");

                    ui.vertical(|ui| {
                        let rect_size = egui::vec2(32.0, 32.0); // Width x Height of each rectangle
                        let spacing = 4.0;
                        let available_width = ui.available_width();

                        let mut current_x = 0.0;
                        let mut current_y = 0.0;

                        let colors = [
                            Color::BASE,
                            Color::TEXT,
                            Color::SURFACE0,
                            Color::SURFACE1,
                            Color::SURFACE2,
                            Color::OVERLAY0,
                            Color::OVERLAY1,
                            Color::YELLOW,
                            Color::GREEN,
                            Color::RED,
                            Color::BLUE,
                            Color::PURPLE,
                            Color::PINK,
                            Color::BLACK,
                            Color::WHITE,
                        ];

                        for color in colors {
                            // Check if adding the next rectangle would exceed available width
                            if current_x + rect_size.x > available_width {
                                current_x = 0.0; // Wrap to the next row
                                current_y += rect_size.y + spacing;
                            }

                            // Define rectangle position
                            let rect = egui::Rect::from_min_size(
                                ui.min_rect().min + egui::vec2(current_x, current_y),
                                rect_size,
                            );

                            // Allocate space and draw the rectangle
                            ui.allocate_rect(rect, egui::Sense::hover());

                            if ui.is_rect_visible(rect) {
                                ui.painter().rect_filled(rect, 8.0, color);
                                ui.painter().rect_stroke(
                                    rect,
                                    8.0,
                                    Stroke {
                                        width: 1.0,
                                        color: Color32::GRAY,
                                    },
                                );
                            }

                            // Move to the next rectangle position
                            current_x += rect_size.x + spacing;
                        }

                        // Reserve the vertical space for the last row of rectangles
                        ui.allocate_space(egui::vec2(0.0, current_y + rect_size.y));
                    });
                });

                if ui.button("Change size and position").clicked() {
                    self.resize_and_center_window(ctx, (1600.0, 900.0).into());
                }

                ui.group(|ui| {
                    ui.label("Application modals");

                    if ui.button("Create/Update Layout").clicked() {
                        self.open_create_update_layout_modal();
                    }

                    if ui.button("Auto-detect components").clicked() {
                        self.open_auto_detect_components_modal();
                    }

                    if ui.button("Create new Profile").clicked() {
                        self.open_create_update_profile_modal(false);
                    }
                });

                ui.group(|ui| {
                    for profile in profiles {
                        let name = profile.name;

                        ui.label(name.clone())
                            .context_menu(|ui| self.open_profile_context_menu(ui, name));
                    }
                });

                let button = ui.button("Hover or click me");

                if button.contains_pointer() || button.has_focus() {
                    ui.label("I appear on hover or when clicked!");
                }

                ui.separator();

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

                ui.text_edit_singleline(&mut port_name).enabled();

                ui.label(format!("Current profile: {}", current_profile));

                // Raw components layout
                ui.label(format!(
                    "Raw layout:\n- Buttons\n{}\n- Potentiometers\n{}",
                    self.server_data.raw_layout.0, self.server_data.raw_layout.1
                ));

                ui.separator();
                ui.group(|ui| {
                    ui.text_edit_multiline(&mut self.xbm_string);

                    if ui.button("Save to memory").clicked() {
                        if self.server_data.is_device_paired {
                            self.show_yes_no_modal(
                                "memory-override-confirmation",
                                "Override memory".to_string(),
                                "This operation will override the current memory!\n\
                                Are you sure you want to continue?"
                                    .to_string(),
                                |_app| {
                                    // `m` = `Memory`, `1` = true
                                    request_send_serial("m1").ok();
                                },
                                |_app| {},
                                true,
                            );
                        } else {
                            self.show_not_paired_error();
                        }
                    }

                    if ui.button("Upload and Test").clicked() {
                        if self.server_data.is_device_paired {
                            let xbm_string = self.xbm_string.clone();

                            match extract_hex_bytes(&xbm_string, HOME_IMAGE_BYTES_SIZE) {
                                Ok(bytes) => {
                                    // `ui` = `Upload *HOME* Image`
                                    let message = format!("ui{}", hex_bytes_vec_to_string(&bytes));

                                    request_send_serial(message.as_str()).ok();

                                    self.show_message_modal(
                                        "xbm-upload-ok",
                                        "Ok".to_string(),
                                        "New X BitMap image \
                                            was uploaded to the device."
                                            .to_string(),
                                    );
                                }
                                Err(error) => {
                                    self.show_message_modal(
                                        "xbm-upload-error",
                                        "Error".to_string(),
                                        error,
                                    );
                                }
                            }
                        } else {
                            self.show_not_paired_error();
                        }
                    }

                    if ui.button("Remove X BitMap").clicked() {
                        if self.server_data.is_device_paired {
                            self.show_yes_no_modal(
                                "xbm-remove-confirmation",
                                "Reset \"Home Image\"".to_string(),
                                "You're about to remove and reset current \"Home Image\" \
                                on your device!\nAre you sure you want to continue?"
                                    .to_string(),
                                |_app| {
                                    // `ui` = `Upload *HOME* Image`, and since there's no value
                                    // the device removes current image and set its default
                                    request_send_serial("ui").ok();
                                },
                                |_app| {},
                                true,
                            );
                        } else {
                            self.show_not_paired_error();
                        }
                    }
                });
            });
    }

    // Application modals

    fn open_create_update_layout_modal(&mut self) {
        use egui::*;

        let mut is_updating = false;

        if let Some(config) = &self.config {
            if let Some(layout) = &config.layout {
                is_updating = true;

                self.new_layout_name = layout.name.clone();
                self.new_layout_size = layout.size;
            }
        }

        self.show_custom_modal("create-update-layout", move |ui, app| {
            ui.set_width(350.0);

            ui.with_layout(
                Layout::from_main_dir_and_cross_align(Direction::TopDown, Align::Center),
                |ui| {
                    ui.scope(|ui| {
                        let mut style = get_current_style();

                        style
                            .text_styles
                            .insert(TextStyle::Body, FontId::new(24.0, FontFamily::Proportional));

                        style.visuals.override_text_color = Some(Color::WHITE);
                        style.visuals.widgets.noninteractive.bg_stroke =
                            Stroke::new(1.0, Color::WHITE);

                        ui.set_style(style);

                        if !is_updating {
                            ui.label("Create new Layout");
                        } else {
                            ui.label("Editing Layout");
                        }

                        ui.separator();
                    });

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 1.0);
                            ui.label("Name");
                        });

                        ui.add_sized(
                            ui.available_size(),
                            TextEdit::singleline(&mut app.new_layout_name).margin(vec2(8.0, 8.0)),
                        );
                    });

                    ui.add_space(16.0);

                    ui.horizontal_top(|ui| {
                        let spacing = ui.spacing().item_spacing.x;

                        let total_width = ui.available_width();
                        let input_width = (total_width - (spacing * 4.0)) / 2.0;

                        ui.allocate_ui_with_layout(
                            (input_width, 0.0).into(),
                            Layout::top_down(Align::Center),
                            |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.vertical(|ui| {
                                        ui.add_space(spacing / 2.0 + 1.0);
                                        ui.label("Width");
                                    });

                                    ui.add_sized(
                                        ui.available_size(),
                                        DragValue::new(&mut app.new_layout_size.0).speed(1.0),
                                    );
                                });
                            },
                        );

                        ui.add_space(16.0);

                        ui.allocate_ui_with_layout(
                            (input_width + spacing, 0.0).into(),
                            Layout::top_down(Align::Center),
                            |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.vertical(|ui| {
                                        ui.add_space(spacing / 2.0 + 1.0);
                                        ui.label("Height");
                                    });

                                    ui.add_sized(
                                        ui.available_size(),
                                        DragValue::new(&mut app.new_layout_size.1).speed(1.0),
                                    );
                                });
                            },
                        );
                    });

                    ui.add_space(16.0);

                    ui.horizontal_top(|ui| {
                        let create_update_button_name =
                            if !is_updating { "Create" } else { "Update" };

                        let spacing = ui.spacing().item_spacing.x;

                        let total_width = ui.available_width();
                        let button_width = (total_width - spacing) / 2.0;

                        if ui
                            .add_sized([button_width, 0.0], Button::new("Cancel"))
                            .clicked()
                        {
                            app.close_modal();
                        }

                        if ui
                            .add_sized([button_width, 0.0], Button::new(create_update_button_name))
                            .clicked()
                        {
                            if let Some(config) = &mut app.config {
                                if config.layout.is_none() {
                                    app.create_update_layout(
                                        app.new_layout_name.clone(),
                                        app.new_layout_size,
                                    );

                                    app.close_modal();
                                } else {
                                    app.show_yes_no_modal(
                                        "layout-override-confirmation-create",
                                        "Overriding current layout".to_string(),
                                        "You already have a layout, \
                                                        do you want to override it?\n\
                                                        You still keep your added components."
                                            .to_string(),
                                        |app| {
                                            app.create_update_layout(
                                                app.new_layout_name.clone(),
                                                app.new_layout_size,
                                            );

                                            app.close_modal();
                                        },
                                        |_app| {},
                                        true,
                                    );
                                }
                            }
                        }
                    });
                },
            );
        });
    }

    fn open_create_update_profile_modal(&mut self, is_updating: bool) {
        use egui::*;

        if let Some(config) = &self.config {
            self.profile_exists = config.does_profile_exist(&self.new_profile_name);

            if !is_updating {
                // Clear the last_profile_name
                self.last_profile_name = String::new();
            }
        }

        self.show_custom_modal("create-update-profile", move |ui, app| {
            ui.set_width(250.0);

            ui.scope(|ui| {
                let mut style = get_current_style();

                style
                    .text_styles
                    .insert(TextStyle::Body, FontId::new(24.0, FontFamily::Proportional));

                style.visuals.override_text_color = Some(Color::WHITE);
                style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color::WHITE);

                ui.set_style(style);

                if !is_updating {
                    ui.label("Create new Profile");
                } else {
                    ui.label("Editing Profile");
                }

                ui.separator();
            });

            ui.add_space(20.0);

            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 1.0);
                        ui.label("Name");
                    });

                    if app.new_profile_name.len() > DASHBOARD_PROFILE_MAX_CHARACTERS {
                        app.new_profile_name
                            .truncate(DASHBOARD_PROFILE_MAX_CHARACTERS);
                    }

                    let name_response = ui.add_sized(
                        ui.available_size(),
                        TextEdit::singleline(&mut app.new_profile_name).margin(vec2(8.0, 8.0)),
                    );

                    if app.new_profile_name.len() > DASHBOARD_PROFILE_MAX_CHARACTERS {
                        app.new_profile_name
                            .truncate(DASHBOARD_PROFILE_MAX_CHARACTERS);
                    }

                    if name_response.lost_focus() {
                        if let Some(config) = &app.config {
                            app.profile_exists = config.does_profile_exist(&app.new_profile_name);
                        }
                    }
                });

                if app.profile_exists {
                    ui.label(
                        RichText::new("A profile with this name already exists!").color(Color::RED),
                    );
                }
            });

            ui.add_space(20.0);

            ui.horizontal_top(|ui| {
                let create_update_button_name = if !is_updating { "Create" } else { "Update" };

                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                if ui
                    .add_sized([button_width, 0.0], Button::new("Cancel"))
                    .clicked()
                {
                    app.close_modal();
                }

                ui.scope(|ui| {
                    if app.profile_exists {
                        ui.disable();
                    }

                    if ui
                        .add_sized([button_width, 0.0], Button::new(create_update_button_name))
                        .clicked()
                    {
                        if app.new_profile_name.is_empty() {
                            app.show_message_modal(
                                "create-update-profile-empty",
                                "Error".to_string(),
                                "Profile name cannot be empty!".to_string(),
                            );

                            return;
                        }

                        if app.create_update_profile(app.new_profile_name.clone()) {
                            app.close_modal();

                            app.show_message_modal(
                                "create-update-profile-success",
                                "Success".to_string(),
                                format!(
                                    "Profile \"{}\" {} successfully.",
                                    app.new_profile_name,
                                    if !is_updating { "created" } else { "updated" }
                                ),
                            );
                        } else {
                            app.show_message_modal(
                                "create-update-profile-already-exist",
                                "Error".to_string(),
                                format!(
                                    "A profile with the name `{}` already exists!",
                                    app.new_profile_name
                                ),
                            );
                        }
                    }
                });
            });
        });
    }

    fn open_auto_detect_components_modal(&self) {
        self.show_yes_no_modal(
            "layout-override-confirmation-auto-detect-components",
            "Override layout".to_string(),
            "This operation will override the current layout!\n\
                            Are you sure you want to proceed?"
                .to_string(),
            |app| {
                app.close_modal();

                match app.detect_components() {
                    Ok(message) => app.show_message_modal(
                        "auto-detected-components-ok",
                        "Success".to_string(),
                        message,
                    ),
                    Err(error) => app.show_message_modal(
                        "auto-detected-components-error",
                        "Error".to_string(),
                        error,
                    ),
                }
            },
            |app| {
                app.close_modal();
            },
            false,
        );
    }

    // Context menus

    fn open_profile_context_menu(&mut self, ui: &mut Ui, profile_name: String) {
        ui.set_max_width(128.0);

        ui.scope(|ui| {
            let mut style = get_current_style();

            style.spacing.menu_spacing = -8.0;
            style.spacing.button_padding = (4.0, 2.0).into();
            style.spacing.item_spacing = (4.0, 4.0).into();

            ui.set_style(style.clone());

            ui.label(profile_name.clone());

            ui.separator();

            ui.menu_button("Edit profile", |ui| {
                ui.set_style(style);

                if ui.button("Update profile").clicked() {
                    self.new_profile_name = profile_name.clone();
                    self.last_profile_name = profile_name.clone();

                    self.open_create_update_profile_modal(true);
                }

                if ui.button("Delete profile").clicked() {
                    self.show_yes_no_modal(
                        "profile-delete-confirmation",
                        "Deleting Profile".to_string(),
                        format!(
                            "You're about to delete \"{}\"\n\
                            Are you sure you want to continue?",
                            profile_name
                        ),
                        move |app| {
                            app.close_modal();

                            match app.delete_profile(&profile_name) {
                                Ok(message) => {
                                    app.show_message_modal(
                                        "profile-delete-confirmation-result",
                                        "Success".to_string(),
                                        message,
                                    );
                                }
                                Err(error) => app.show_message_modal(
                                    "profile-delete-confirmation-result",
                                    "Error".to_string(),
                                    error,
                                ),
                            }
                        },
                        |app| {
                            app.close_modal();
                        },
                        false,
                    );
                }
            });
        });
    }

    fn open_grid_context_menu(&mut self, ui: &mut Ui) {
        ui.set_max_width(128.0);

        let mut layout_size = (0.0, 0.0);

        if let Some(config) = &self.config {
            if let Some(layout) = &config.layout {
                layout_size = layout.size;
            }
        };

        let max_range = (layout_size.0.min(layout_size.1)) / 8.0; // To avoid snapping to outside
                                                                  // of layout border
        let min_range = if max_range < 2.0 { 1.0 } else { 2.0 };

        ui.label("Grid settings");

        ui.separator();

        ui.add(
            DragValue::new(&mut self.layout_grid.1)
                .speed(2.0)
                .range(min_range..=max_range)
                .clamp_existing_to_range(true),
        );
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

        // IPC handling between dashboard and service self
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
            unavoidable_error: (false, String::new()),
            modal: Arc::new(Mutex::new(ModalManager::new())),
            config: match Config::default().read() {
                Ok(config) => Some(config),
                Err(err) => {
                    log_error!("Error reading config file: {}", err);

                    None
                }
            },
            server_data: ServerData::default(),
            components: HashMap::default(),
            editing_layout: false,
            dragged_component_offset: (0.0, 0.0),
            layout_grid: (true, 10.0),
            layout_backup_components: Default::default(),

            // Visuals
            global_shadow: 8.0,

            // TEMP VARIABLES
            new_layout_name: "New Layout".to_string(),
            new_layout_size: (1030.0, 580.0),
            new_profile_name: "My Profile".to_string(),
            last_profile_name: String::new(), // Used for updating a profile
            profile_exists: false,
            xbm_string: String::new(),
            paired_status_panel: (0.0, 0.0),

            // TODO: Remove this
            test_potentiometer_style: 0,
            test_potentiometer_value: 15.0,
            test_joystick_value: (0.0, 0.0),

            // Constants
            component_button_size: (100.0, 100.0),
            component_led_size: (100.0, 100.0),
            component_potentiometer_size: (100.0, 100.0),
            component_joystick_size: (100.0, 100.0),
            component_rotary_encoder_size: (100.0, 100.0),
            component_display_size: (128.0, 64.0),
        }
    }
}
