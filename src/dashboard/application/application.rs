use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use eframe::egui::{self, Button, Context, Pos2, ProgressBar, Rect, Response, Ui, Vec2};

use super::{get_current_style, utility::request_send_serial, widgets::*};
use padpad_software::{
    config::{
        update_config_and_server, Component, ComponentKind, Config, Interaction, Layout, Profile,
    },
    constants::{
        DASHBOARD_PROFILE_MAX_CHARACTERS, HOME_IMAGE_SIZE, SERIAL_MESSAGE_INNER_SEP,
        SERVER_DATA_UPDATE_INTERVAL,
    },
    log_error, log_print,
    tcp::{client_to_server_message, ServerData},
    utility::extract_hex_bytes_and_serialize,
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

    // TEMP VARIABLES
    new_layout_name: String,
    new_layout_size: (f32, f32),
    new_profile_name: String,
    profile_exists: bool,
    xbm_string: String,
    paired_status_panel: (f32 /* position_x */, f32 /* opacity */),

    // Constants
    component_button_size: (f32 /* width */, f32 /* height */),
    component_potentiometer_size: (f32 /* width */, f32 /* height */),
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

                // show_close_popup
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
                    .add_sized([button_width, 0.0], Button::new("Ok"))
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
                    .add_sized([button_width, 0.0], Button::new("Yes"))
                    .clicked()
                {
                    on_confirm(app);

                    if auto_close {
                        app.close_modal();
                    }
                }
                if ui
                    .add_sized([button_width, 0.0], Button::new("No"))
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

                let new_layout_button = ui.add_sized((128.0, 128.0), Button::new("+"));

                if new_layout_button.clicked() {
                    self.open_create_update_layout_modal();
                }
            });
        });
    }

    fn draw_layout(&mut self, ctx: &Context) {
        let mut layout: Option<&Layout> = None;

        if let Some(config) = &self.config {
            if let Some(l) = &config.layout {
                layout = Some(l);
            }
        }

        let layout = if let Some(layout) = layout {
            layout
        } else {
            return;
        };

        egui::Window::new("Layout")
            .movable(false)
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .hscroll(true)
            .vscroll(true)
            .fixed_size((layout.size.0, layout.size.1))
            .current_pos((
                (ctx.screen_rect().max.x - layout.size.0) / 2.0,
                (ctx.screen_rect().max.y - layout.size.1) / 2.0,
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
                ..egui::Frame::default()
            })
            .show(ctx, |ui| {
                if layout.components.is_empty() {
                    ui.label("You haven't add any components yet!");

                    return;
                }

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

                    match kind {
                        ComponentKind::None => (),
                        ComponentKind::Button => {
                            let button = self.draw_button(
                                ui,
                                label,
                                position,
                                Vec2::from(self.component_button_size),
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
                                Vec2::from(self.component_potentiometer_size),
                                value.parse::<u8>().unwrap_or(0),
                            );
                        }
                        ComponentKind::Joystick => (),
                        ComponentKind::RotaryEncoder => (),
                        ComponentKind::Display => (),
                    }
                }
            });
    }

    fn create_update_profile(&mut self, name: String) -> bool {
        if let Some(config) = &mut self.config {
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
                c.profiles.remove(profile_index as usize);
            });

            return Ok(format!(
                "Profile {} was successfully removed.",
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
                let size = pos2(212.0, 64.0 - (padding.y / 2.0) - 1.0);

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
        &self,
        ui: &mut Ui,
        label: &String,
        relative_position: Pos2, /* relative to window position */
        size: Vec2,
        value: i8,
    ) -> Response {
        let window_position = ui.min_rect().min;
        let position = Pos2::new(
            relative_position.x + window_position.x,
            relative_position.y + window_position.y,
        );
        let rect = Rect::from_min_size(position, size.into());

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

            let layout_button =
                Component::new_button(button_id, button_name, (current_x, current_y));

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
                (current_x, current_y),
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
        //let buttons_string = "1|97|98|2|99|100|3|101|102|4|103|104|5|105|106";
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

                    if ui.button("Create/Update Profile").clicked() {
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

                circular_progress(ui, 0.325, 50.0);

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

                            match extract_hex_bytes_and_serialize(&xbm_string, HOME_IMAGE_SIZE) {
                                Ok(bytes) => {
                                    // `ui` = `Upload *HOME* Image`
                                    let message = format!("ui{}", &bytes);

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

    fn open_create_update_profile_modal(&mut self, is_device_internal: bool) {
        use egui::*;

        if let Some(config) = &self.config {
            self.profile_exists = config.does_profile_exist(&self.new_profile_name);
        }

        let is_updating = false;

        self.show_custom_modal("create-update-profile", move |ui, app| {
            ui.set_width(350.0);

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
                        if app.create_update_profile(app.new_profile_name.clone()) {
                            app.close_modal();

                            app.show_message_modal(
                                "create-profile-success",
                                "Success".to_string(),
                                format!(
                                    "Profile \"{}\" created successfully.",
                                    app.new_profile_name
                                ),
                            );
                        } else {
                            app.show_message_modal(
                                "create-profile-already-exist",
                                "Error".to_string(),
                                format!(
                                    "A profile with the name `{}` already exists!",
                                    app.new_profile_name
                                ),
                            );
                        }

                        //app.close_modal();
                        if !app.profile_exists {
                        } else {
                            //app.show_yes_no_modal(
                            //    "layout-override-confirmation-create",
                            //    "Overriding current layout".to_string(),
                            //    "You already have a layout, \
                            //                            do you want to override it?\n\
                            //                            You still keep your added components."
                            //        .to_string(),
                            //    |app| {
                            //        app.create_update_profile(app.new_profile_name.clone());
                            //
                            //        app.close_modal();
                            //    },
                            //    |_app| {},
                            //    true,
                            //);
                        }
                    }
                });
            });

            if is_device_internal {
                ui.label("You are editing device's internal profile!");

                return;
            }
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

    fn open_profile_context_menu(&self, ui: &mut Ui, profile_name: String) {
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
                    //println!("Updating profile {}", profile_name);
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

            // TEMP VARIABLES
            new_layout_name: "New Layout".to_string(),
            new_layout_size: (1030.0, 580.0),
            new_profile_name: "My Profile".to_string(),
            profile_exists: false,
            xbm_string: String::new(),
            paired_status_panel: (0.0, 0.0),

            // Constants
            component_button_size: (100.0, 100.0),
            component_potentiometer_size: (100.0, 100.0),
        }
    }
}
