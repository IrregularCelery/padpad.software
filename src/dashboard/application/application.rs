use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use eframe::egui::{self, Context, DragValue, Pos2, Rect, Response, Ui, Vec2};

use super::{
    get_current_style,
    utility::{
        blend_colors, request_device_upload, request_refresh_device, request_restart_service,
        request_send_serial,
    },
    widgets::*,
};
use padpad_software::{
    config::{
        update_config_and_server, Component, ComponentKind, Config, Interaction, Layout, Profile,
    },
    constants::{
        APP_MIN_HEIGHT, APP_MIN_WIDTH, APP_NAME, APP_PADDING_X, APP_PADDING_Y, APP_VERSION,
        DASHBOARD_DISAPLY_PIXEL_SIZE, DASHBOARD_PROFILE_MAX_CHARACTERS, DEFAULT_BAUD_RATE,
        DEFAULT_DEVICE_NAME, FORBIDDEN_CHARACTERS, HOME_IMAGE_BYTES_SIZE, HOME_IMAGE_DEFAULT_BYTES,
        HOME_IMAGE_HEIGHT, HOME_IMAGE_WIDTH, KEYS, SERIAL_MESSAGE_END, SERIAL_MESSAGE_INNER_SEP,
        SERIAL_MESSAGE_SEP, SERVER_DATA_UPDATE_INTERVAL,
    },
    log_error,
    service::interaction::InteractionKind,
    tcp::{client_to_server_message, ServerData},
    utility::{extract_hex_bytes, hex_bytes_string_to_vec, hex_bytes_vec_to_string, restart},
};

static SERVER_DATA: OnceLock<Arc<Mutex<ServerData>>> = OnceLock::new();
static ERROR_MESSAGE: OnceLock<Arc<Mutex<String>>> = OnceLock::new(); // Global vairable to keep the
                                                                      // last unavoidable error message

pub struct Application {
    update_available: bool,
    build_date: String,
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
    needs_resize: bool,
    device_name: String,
    baud_rate_string: String,
    baud_rate: Option<u32>,
    port_name: (String, bool /* overridden */),
    server_needs_restart: bool,
    server_data: ServerData,
    components: HashMap<String /* component_global_id */, String /* value */>,
    component_properties: (Option<Component>, Option<Interaction>), // Current editing component properties
    button_memory: HashMap<
        String,                             /* component_global_id */
        ((u8, String), (u8, String), bool), /* (normal (byte, str), mod (byte, str), is_modkey) */
    >,
    is_editing_layout: bool,
    dragged_component_offset: (f32, f32),
    layout_grid: (bool /* enabled/disabled */, f32 /* size */),
    /// For storing last components state before editing layout
    components_backup: (HashMap<String, Component>, HashMap<String, Interaction>),
    // Visuals
    global_shadow: f32,

    // TEMP VARIABLES (per modal)
    properties_selected_interaction: bool, // `true` -> normal, `false` -> modkey (if available)
    properties_shortcut_key_filter: String,
    properties_shortcut_kind: (bool, bool), // (normal, modkey) `true` -> keys, `false` -> text
    component_id: u8,
    component_id_exists: bool,
    new_layout_name: String,
    new_layout_size: (f32, f32),
    new_profile_name: String,
    last_profile_name: String, // Used for updating a profile
    profile_exists: bool,
    xbm_string: String,
    xbm_serialized: (
        String, /* value */
        String, /* component_global_id */
    ),
    current_display_image: Vec<u8>,
    paired_status_panel: (f32 /* position_x */, f32 /* opacity */),
    components_panel: f32, /* position_x */
    toolbar_panel: f32,    /* position_x */

    #[cfg(debug_assertions)]
    test_potentiometer_style: u8,
    #[cfg(debug_assertions)]
    test_potentiometer_value: f32,
    #[cfg(debug_assertions)]
    test_joystick_value: (f32, f32),
    #[cfg(debug_assertions)]
    test_ascii_char_input: String,

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

        // Resize the application's window if needed
        self.resize_window_based_on_layout(ctx);

        if self.server_needs_restart {
            request_restart_service().ok();

            self.server_needs_restart = false;
        }

        // Server data
        self.handle_server_data();

        // Modal manager
        self.handle_modal(ctx);

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
                Id::new("title-bar"),
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
                    // Close, Minimize and About Button
                    let button_size = (32.0, 32.0);

                    ui.scope(|ui| {
                        let mut style = get_current_style();

                        style.visuals.widgets.inactive.weak_bg_fill =
                            Color::SURFACE2.gamma_multiply(0.95);
                        style.visuals.widgets.hovered.weak_bg_fill =
                            Color::OVERLAY0.gamma_multiply(0.5);
                        style.visuals.widgets.hovered.bg_stroke.color =
                            Color::WHITE.gamma_multiply(0.5);
                        style.visuals.widgets.active.weak_bg_fill =
                            Color::BLACK.gamma_multiply(0.25);
                        style.visuals.widgets.noninteractive.bg_stroke.color =
                            Color::WHITE.gamma_multiply(0.5);

                        style.spacing.button_padding = Vec2::ZERO;

                        ui.set_style(style);

                        let close_button = ui
                            .add_sized(button_size, Button::new(RichText::new("❌")))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Close the window");

                        let minimized_button = ui
                            .add_sized(button_size, Button::new(RichText::new("🗕").size(13.0)))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Minimize the window");

                        let about_button = ui
                            .add_sized(button_size, Button::new(RichText::new("♥").size(20.0)))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(format!("About {}", APP_NAME));

                        if close_button.clicked() {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                        }

                        if minimized_button.clicked() {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                        }

                        if about_button.clicked() {
                            self.open_about_modal();
                        }
                    });
                },
            );

            // Custom main window content

            self.draw_status_indicator(ui);

            if let Some(config) = &self.config {
                if config.layout.is_none() {
                    self.draw_new_layout_button(ui);
                } else {
                    self.draw_profile_select(ui);
                }
            }

            self.draw_components_panel(ui);
            self.draw_toolbar_panel(ui);
        });

        // Unavoidable errors
        // Handle it last to ensure it's the topmost element
        self.handle_unavoidable_error(ctx);

        if cfg!(debug_assertions) {
            self.draw_debug_panel(ctx);
        }

        // Redraw continuously at 60 FPS
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

impl Application {
    fn resize_window_based_on_layout(&mut self, ctx: &Context) {
        if !self.needs_resize {
            return;
        }

        if let Some(config) = &self.config {
            if let Some(layout) = &config.layout {
                let mut new_window_size = (
                    layout.size.0 + (APP_PADDING_X * 2) as f32,
                    layout.size.1 + (APP_PADDING_Y * 2) as f32,
                );

                if new_window_size.0 < APP_MIN_WIDTH as f32 {
                    new_window_size.0 = APP_MIN_WIDTH as f32;
                }

                if new_window_size.1 < APP_MIN_HEIGHT as f32 {
                    new_window_size.1 = APP_MIN_HEIGHT as f32;
                }

                self.resize_and_center_window(ctx, new_window_size.into());

                self.needs_resize = false;
            }
        }
    }

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

                // Check if editing mode is active
                if !self.components_backup.0.is_empty()
                    || !self.components_backup.1.is_empty() && self.components_changed()
                {
                    self.show_message_modal(
                        "layout-editing-mode-active-exit-popup",
                        "Editing Mode Active".to_string(),
                        "Editing Mode is currently active.\n\
                        Please exit Editing Mode before closing the application."
                            .to_string(),
                    );

                    return;
                }

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

                        let cancel_button_response =
                            ui.add_sized([button_width, 0.0], egui::Button::new("Cancel"));

                        if cancel_button_response.clicked() {
                            self.close_app.0 = false;
                        }

                        if ui
                            .add_sized([button_width, 0.0], egui::Button::new("Close"))
                            .clicked()
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
                    if modal.width > 0.0 {
                        ui.set_width(modal.width);
                    }

                    (modal.content)(ui, self);
                });

            if modal_ui.should_close() && modal.can_close {
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
                    can_close: true,
                    width: 0.0,
                    content: Arc::new(content),
                });
            }
        }
    }

    /// Set the `can_close` field for the last modal in stack
    fn set_can_close_modal(&mut self, can_close: bool) {
        if let Ok(mut modal) = self.modal.lock() {
            if let Some(last_modal) = modal.stack.last_mut() {
                last_modal.can_close = can_close;
            }
        }
    }

    fn set_width_modal(&mut self, width: f32) {
        if let Ok(mut modal) = self.modal.lock() {
            if let Some(last_modal) = modal.stack.last_mut() {
                last_modal.width = width;
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
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
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
                    .add_sized([button_width, 0.0], egui::Button::new("No"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    on_deny(app);

                    if auto_close {
                        app.close_modal();
                    }
                }

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Yes"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    on_confirm(app);

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
                name: name.trim().to_string(),
                size,
                components,
            };

            update_config_and_server(config, |c| {
                c.layout = Some(layout);
            });

            self.needs_resize = true;
        }
    }

    fn draw_new_layout_button(&mut self, ui: &mut Ui) {
        egui::Window::new("Add layout button")
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .movable(false)
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .frame(egui::Frame::none())
            .fixed_size(((APP_MIN_WIDTH / 2) as f32, 0.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
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

                        let new_layout_button = ui
                            .add_sized((128.0, 128.0), egui::Button::new("+"))
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        if new_layout_button.clicked() {
                            self.open_create_update_layout_modal();
                        }
                    });

                    ui.label(
                        egui::RichText::new("You haven't created your layout yet")
                            .color(egui::Color32::from_gray(150))
                            .size(28.0),
                    );

                    ui.add_space(-ui.style().spacing.item_spacing.y);

                    ui.label(
                        egui::RichText::new("Click the add button to start...")
                            .color(egui::Color32::from_gray(110)),
                    );
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
                    offset: egui::vec2(0.0, if self.is_editing_layout { 0.0 } else { 4.0 }),
                    blur: 8.0,
                    spread: if self.is_editing_layout { 8.0 } else { 2.0 },
                    color: if self.is_editing_layout {
                        Color::RED.gamma_multiply(0.15)
                    } else {
                        Color::OVERLAY1.gamma_multiply(0.5)
                    },
                },
                stroke: {
                    if self.is_editing_layout {
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
                    egui::Area::new("layout-empty-hint-texts".into())
                        .order(egui::Order::Foreground)
                        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                        .show(ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(-ui.style().spacing.item_spacing.y);

                                ui.label(
                                    egui::RichText::new(
                                        "You can click the (+) button on the left to add \
                                        components\nRemember to to save your work using \
                                        (🖴) button.",
                                    )
                                    .size(22.0)
                                    .color(egui::Color32::from_gray(135)),
                                );
                            });
                        });

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
                                let color = blend_colors(Color::WHITE, Color::ACCENT, 0.5);

                                let r = color.r();
                                let g = color.g();
                                let b = color.b();

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

                    let response = if let Some(r) = response {
                        r
                    } else {
                        continue;
                    };

                    if !self.is_editing_layout {
                        if response.double_clicked() {
                            self.toggle_layout_state();

                            self.open_component_properties_modal(component.0.clone());
                        }

                        continue;
                    }

                    // Show label and Id on hover
                    let response = response.on_hover_ui(|ui| {
                        ui.group(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Id\t\t\t");

                                ui.add_space(-1.0);

                                ui.label(
                                    egui::RichText::new(component.0.clone()).color(Color::ACCENT),
                                );

                                ui.add_space(ui.style().spacing.item_spacing.x);
                            });

                            if kind != ComponentKind::Display {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("Label \t");

                                    ui.label(egui::RichText::new(label).color(Color::ACCENT));

                                    ui.add_space(ui.style().spacing.item_spacing.x);
                                });
                            }
                        });
                    });

                    // Open the interactions modal
                    if response.clicked() {
                        self.open_component_properties_modal(component.0.clone());
                    }

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

            self.is_editing_layout = false;
            self.components_backup = Default::default();
        } else {
            self.show_message_modal(
                "layout-save-elements-no-config",
                "Error".to_string(),
                "There was a problem was saving your layout to config!".to_string(),
            );

            return;
        }
    }

    /// Toggle layout state between editing and viewing
    fn toggle_layout_state(&mut self) {
        self.is_editing_layout = !self.is_editing_layout;

        if self.is_editing_layout {
            // Started editing layout

            if let Some(config) = &self.config {
                if let Some(layout) = &config.layout {
                    self.components_backup = (
                        layout.components.clone(),
                        config.profiles[config.settings.current_profile]
                            .interactions
                            .clone(),
                    );
                }
            }
        } else {
            // Stopped editing layout

            if self.components_changed() {
                self.show_yes_no_modal(
                    "layout-edited",
                    "Edited Layout".to_string(),
                    "You have changed your current layout, Do you want to save it?".to_string(),
                    |app| {
                        app.close_modal();

                        app.save_current_layout();

                        app.show_message_modal(
                            "layout-saved-successfully",
                            "Success".to_string(),
                            "Your current layout was saved successfully!".to_string(),
                        )
                    },
                    |app| {
                        if let Some(config) = &mut app.config {
                            if let Some(layout) = &mut config.layout {
                                layout.components = app.components_backup.0.clone();
                            }

                            config.profiles[config.settings.current_profile].interactions =
                                app.components_backup.1.clone();
                        }

                        app.components_backup = Default::default();

                        app.close_modal();
                    },
                    false,
                );

                self.set_can_close_modal(false);
            } else {
                self.components_backup = Default::default();
            }
        }
    }

    fn add_button_to_layout(&mut self) {
        if !self.is_editing_layout {
            self.toggle_layout_state();
        }

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
        if !self.is_editing_layout {
            self.toggle_layout_state();
        }

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
        if !self.is_editing_layout {
            self.toggle_layout_state();
        }

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
        if !self.is_editing_layout {
            self.toggle_layout_state();
        }

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
        if !self.is_editing_layout {
            self.toggle_layout_state();
        }

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
        if !self.is_editing_layout {
            self.toggle_layout_state();
        }

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

    fn delete_component(&mut self, component_global_id: &String) {
        if let Some(config) = &mut self.config {
            if let Some(layout) = &mut config.layout {
                layout.components.remove(component_global_id);
            }

            for profile in &mut config.profiles {
                profile.interactions.remove(component_global_id);
            }
        }
    }

    fn components_changed(&self) -> bool {
        if let Some(config) = &self.config {
            if let Some(layout) = &config.layout {
                if self.components_backup.0 != layout.components {
                    return true;
                }
            }

            if self.components_backup.1
                != config.profiles[config.settings.current_profile].interactions
            {
                return true;
            }
        }

        false
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

                        profile.name = name.clone().trim().to_string();

                        break;
                    }
                });

                request_refresh_device();

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
                name: name.trim().to_string(),
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

            request_refresh_device();

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

            request_refresh_device();

            return Ok(format!(
                "Profile \"{}\" was successfully removed.",
                profile_name
            ));
        }

        Err("There was an unknown error!\nPlease try again.".to_string())
    }

    fn draw_status_indicator(&mut self, ui: &mut Ui) {
        use egui::*;

        let indicator_size = 48.0;
        let padding = ui.style().spacing.item_spacing.x;

        let app_rect = ui.clip_rect();
        let footer_height = indicator_size;
        let footer_rect = {
            let mut rect = app_rect;

            rect.min.y = rect.max.y - footer_height - (padding * 2.0);

            rect.shrink(padding)
        };

        let ui_builder = UiBuilder::new()
            .max_rect(footer_rect)
            .layout(Layout::left_to_right(Align::Max));

        ui.allocate_new_ui(ui_builder, |ui| {
            let paired_status_color = if self.server_data.is_device_paired {
                Color::GREEN
            } else {
                Color::RED
            };

            let indicator = status_indicator(
                "device-paired-status-indicator",
                ui,
                paired_status_color,
                indicator_size,
            );

            if indicator.clicked() {
                self.open_connection_modal();
            }

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

    fn draw_profile_select(&mut self, ui: &mut Ui) {
        use egui::*;

        let profile_name_height = 48.0;
        let padding = ui.style().spacing.item_spacing.x;

        let app_rect = ui.clip_rect();
        let footer_height = profile_name_height;
        let footer_rect = {
            let mut rect = app_rect;

            rect.min.y = rect.max.y - footer_height - (padding * 4.0);

            rect.shrink2((padding, padding * 4.0).into())
        };

        let mut style = get_current_style();

        style.visuals.widgets.inactive.weak_bg_fill = Color::SURFACE2.gamma_multiply(0.95);
        style.visuals.widgets.hovered.weak_bg_fill = Color::OVERLAY0.gamma_multiply(0.5);
        style.visuals.widgets.hovered.bg_stroke.color = Color::WHITE.gamma_multiply(0.5);
        style.visuals.widgets.active.weak_bg_fill = Color::BLACK.gamma_multiply(0.25);
        style.visuals.widgets.open.weak_bg_fill = Color::BLACK.gamma_multiply(0.25);
        style.visuals.widgets.noninteractive.bg_stroke.color = Color::WHITE.gamma_multiply(0.5);

        ui.set_style(style);

        let ui_builder = UiBuilder::new()
            .max_rect(footer_rect)
            .layout(Layout::right_to_left(Align::Max));

        ui.allocate_new_ui(ui_builder, |ui| {
            egui::ComboBox::new("profile-select", "")
                .width(135.0)
                .selected_text({
                    if let Some(config) = &self.config {
                        config.profiles[config.settings.current_profile]
                            .name
                            .as_str()
                    } else {
                        "Loading profile..."
                    }
                })
                .show_ui(ui, |ui| {
                    let mut style = get_current_style();

                    style.visuals.widgets.inactive.weak_bg_fill = Color::WHITE.gamma_multiply(0.15);
                    style.visuals.widgets.hovered.weak_bg_fill =
                        Color::OVERLAY0.gamma_multiply(0.95);
                    style.visuals.widgets.hovered.bg_stroke.color =
                        Color::WHITE.gamma_multiply(0.5);
                    style.visuals.widgets.active.weak_bg_fill = Color::BLACK.gamma_multiply(0.25);
                    style.visuals.widgets.noninteractive.bg_stroke.color =
                        Color::WHITE.gamma_multiply(0.5);

                    ui.set_style(style);

                    if let Some(config) = &mut self.config {
                        let mut selected_profile = -1;

                        for (index, profile) in &mut config.profiles.iter_mut().enumerate() {
                            let name = profile.name.clone();

                            if ui
                                .selectable_label(config.settings.current_profile == index, name)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                if config.settings.current_profile != index {
                                    selected_profile = index as i32;
                                }
                            }
                        }

                        if selected_profile >= 0 {
                            update_config_and_server(config, |c| {
                                c.settings.current_profile = selected_profile as usize;
                            });

                            request_refresh_device();
                        }
                    } else {
                        ui.label("Loading");
                    }
                })
                .response
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Current profile");

            ui.add_space(-ui.style().spacing.item_spacing.x);

            if ui
                .button(RichText::new("✏"))
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Edit current profile")
                .clicked()
            {
                if let Some(config) = &self.config {
                    let profile_name = &config.profiles[config.settings.current_profile].name;

                    self.new_profile_name = profile_name.to_string();
                    self.last_profile_name = profile_name.to_string();

                    self.open_create_update_profile_modal(true);
                }
            }

            if ui
                .button("➕")
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Create a new profile")
                .clicked()
            {
                self.open_create_update_profile_modal(false);
            }
        });
    }

    /// Left side panel for adding components to the layout (Only available when a layout exists)
    fn draw_components_panel(&mut self, ui: &mut Ui) {
        if let Some(config) = &self.config {
            if config.layout.is_none() {
                return;
            }
        } else {
            return;
        }

        use egui::*;

        let panel_position_x = animate_value(
            ui.ctx(),
            "components-panel-position",
            self.components_panel,
            0.25,
        );

        let buttons_count = 9; // 8 buttons + extra spacing

        let padding = ui.style().spacing.item_spacing.x - 2.0;
        let button_size = vec2(42.0, 42.0);
        let open_button_size = button_size;

        let screen_rect = ui.ctx().screen_rect();
        let panel_width = button_size.x + (padding + 2.0) * 2.0;
        let panel_height =
            (buttons_count as f32 * button_size.y) + ((buttons_count - 1) as f32 * (padding + 2.0));

        let panel_opened_x = 0.0;
        let panel_closed_x = -(panel_width + padding);

        let panel_x = (padding + 2.0) + panel_closed_x - panel_position_x;
        let panel_y = screen_rect.center().y - panel_height / 2.0;

        let panel_open_button_x = (padding + 2.0) + panel_position_x;
        let panel_open_button_y = screen_rect.center().y - open_button_size.y / 2.0;

        Area::new("components-panel-items".into())
            .constrain(false)
            .order(Order::Foreground)
            .fixed_pos(egui::pos2(panel_x, panel_y))
            .show(ui.ctx(), |ui| {
                Frame::menu(ui.style())
                    .fill(Color::SURFACE2.gamma_multiply(0.75))
                    .inner_margin(Margin::same(padding))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            let mut style = get_current_style();

                            style.visuals.widgets.inactive.weak_bg_fill =
                                Color::WHITE.gamma_multiply(0.15);
                            style.visuals.widgets.hovered.weak_bg_fill =
                                Color::OVERLAY0.gamma_multiply(0.95);
                            style.visuals.widgets.hovered.bg_stroke.color =
                                Color::WHITE.gamma_multiply(0.5);
                            style.visuals.widgets.active.weak_bg_fill =
                                Color::BLACK.gamma_multiply(0.25);
                            style.visuals.widgets.noninteractive.bg_stroke.color =
                                Color::WHITE.gamma_multiply(0.5);

                            ui.set_style(style);

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🔃").size(24.0)))
                                .on_hover_text(
                                    "Automatically detect device components\n\
                                    (currently only buttons and potentiometers)",
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.open_auto_detect_components_modal();
                            }

                            ui.separator();

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🇧").size(24.0)))
                                .on_hover_text("Add Button to your layout")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.add_button_to_layout();
                            }

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🇱").size(24.0)))
                                .on_hover_text("Add LED to your layout")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.add_led_to_layout();
                            }

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🇵").size(24.0)))
                                .on_hover_text("Add Potentiometer to your layout")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.add_potentiometer_to_layout();
                            }

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🇯").size(24.0)))
                                .on_hover_text("Add Joystick to your layout")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.add_joystick_to_layout();
                            }

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🇷").size(24.0)))
                                .on_hover_text("Add RotaryEncoder to your layout")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.add_rotary_encoder_to_layout();
                            }

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🇩").size(24.0)))
                                .on_hover_text("Add Display to your layout")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.add_display_to_layout();
                            }

                            ui.separator();

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🖴").size(24.0)))
                                .on_hover_text(
                                    "Save/Discard current layout\n\
                                    and close components panel",
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                                || (!self.is_editing_layout
                                    && self.components_panel != panel_opened_x)
                            {
                                if self.components_panel == panel_closed_x {
                                    self.components_panel = panel_opened_x;

                                    if self.is_editing_layout {
                                        self.toggle_layout_state();
                                    }
                                }
                            }
                        });
                    });
            });

        Area::new("components-panel-button".into())
            .constrain(false)
            .order(Order::Foreground)
            .fixed_pos(egui::pos2(panel_open_button_x, panel_open_button_y))
            .show(ui.ctx(), |ui| {
                let mut style = get_current_style();

                style.visuals.widgets.inactive.weak_bg_fill = Color::SURFACE2.gamma_multiply(0.95);
                style.visuals.widgets.hovered.weak_bg_fill = Color::OVERLAY0.gamma_multiply(0.5);
                style.visuals.widgets.hovered.bg_stroke.color = Color::WHITE.gamma_multiply(0.5);
                style.visuals.widgets.active.weak_bg_fill = Color::BLACK.gamma_multiply(0.25);
                style.visuals.widgets.noninteractive.bg_stroke.color =
                    Color::WHITE.gamma_multiply(0.5);

                ui.set_style(style);

                if ui
                    .add_sized(open_button_size, Button::new("➕"))
                    .on_hover_text("Add components to your layout")
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked()
                    || self.is_editing_layout
                {
                    if self.components_panel == panel_opened_x {
                        self.components_panel = panel_closed_x;

                        if !self.is_editing_layout {
                            self.toggle_layout_state();
                        }
                    }
                }
            });

        Area::new("is-editing-layout-indicator-panel".into())
            .constrain(false)
            .order(Order::Foreground)
            .anchor(
                Align2::CENTER_TOP,
                vec2(0.0, -panel_width - panel_position_x),
            )
            .show(ui.ctx(), |ui| {
                ui.set_max_width(350.0);

                ui.add_space(ui.style().spacing.item_spacing.y * 2.0);

                Frame::menu(ui.style())
                    .fill(blend_colors(Color::RED, Color::SURFACE2, 0.93))
                    .stroke(Stroke::new(1.0, Color::RED))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Editing Mode Active").color(Color::RED));
                    });
            });
    }

    /// Right side panel for settings and configurations (Only available when a layout exists)
    fn draw_toolbar_panel(&mut self, ui: &mut Ui) {
        if let Some(config) = &self.config {
            if config.layout.is_none() {
                return;
            }
        } else {
            return;
        }

        use egui::*;

        let panel_position_x =
            animate_value(ui.ctx(), "toolbar-panel-position", self.toolbar_panel, 0.25);

        let buttons_count = 4; // 3 buttons + extra spacing

        let padding = ui.style().spacing.item_spacing.x - 2.0;
        let button_size = vec2(42.0, 42.0);
        let open_button_size = button_size;

        let screen_rect = ui.ctx().screen_rect();
        let panel_width = button_size.x + (padding + 2.0) * 2.0;
        let panel_height =
            (buttons_count as f32 * button_size.y) + ((buttons_count - 1) as f32 * (padding + 2.0));

        let panel_opened_x = 0.0;
        let panel_closed_x = panel_width + padding;

        //let panel_x = (padding + 2.0) + panel_closed_x - panel_position_x;
        let panel_x =
            -(padding - 2.0) + screen_rect.max.x - panel_width - panel_position_x + panel_closed_x;
        let panel_y = screen_rect.center().y - panel_height / 2.0;

        let panel_open_button_x =
            (padding + 2.0) + screen_rect.max.x - panel_width + panel_position_x;
        //let panel_open_button_x = (padding + 2.0) + panel_position_x;
        let panel_open_button_y = screen_rect.center().y - open_button_size.y / 2.0;

        Area::new("toolbar-panel-items".into())
            .constrain(false)
            .order(Order::Foreground)
            .fixed_pos(egui::pos2(panel_x, panel_y))
            .show(ui.ctx(), |ui| {
                Frame::menu(ui.style())
                    .inner_margin(Margin::same(padding))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            let mut style = get_current_style();

                            style.visuals.widgets.inactive.weak_bg_fill =
                                Color::WHITE.gamma_multiply(0.15);
                            style.visuals.widgets.hovered.weak_bg_fill =
                                Color::OVERLAY0.gamma_multiply(0.95);
                            style.visuals.widgets.hovered.bg_stroke.color =
                                Color::WHITE.gamma_multiply(0.5);
                            style.visuals.widgets.active.weak_bg_fill =
                                Color::BLACK.gamma_multiply(0.25);
                            style.visuals.widgets.noninteractive.bg_stroke.color =
                                Color::WHITE.gamma_multiply(0.5);

                            ui.set_style(style);

                            if ui
                                .add_sized(button_size, Button::new(RichText::new("🖮").size(24.0)))
                                .on_hover_text("Open Button Memory Manager")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.open_button_memory_manager_modal();
                            }

                            let layout_settings_button = ui
                                .add_sized(button_size, Button::new(RichText::new("📰").size(24.0)))
                                .on_hover_text(
                                    "Open Layout settings\n\
                                    Right click to view grid settings",
                                )
                                .on_hover_cursor(CursorIcon::PointingHand);

                            layout_settings_button
                                .context_menu(|ui| self.open_grid_context_menu(ui));

                            if layout_settings_button.clicked() {
                                self.open_layout_settings_modal();
                            }

                            ui.separator();

                            if ui
                                .add_sized(button_size, Button::new("⮫"))
                                .on_hover_text("Close Toolbar panel")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                if self.toolbar_panel == panel_closed_x {
                                    self.toolbar_panel = panel_opened_x;
                                }
                            }
                        });
                    });
            });

        Area::new("toolbar-panel-button".into())
            .constrain(false)
            .order(Order::Foreground)
            .fixed_pos(egui::pos2(panel_open_button_x, panel_open_button_y))
            .show(ui.ctx(), |ui| {
                let mut style = get_current_style();

                style.visuals.widgets.inactive.weak_bg_fill = Color::SURFACE2.gamma_multiply(0.95);
                style.visuals.widgets.hovered.weak_bg_fill = Color::OVERLAY0.gamma_multiply(0.5);
                style.visuals.widgets.hovered.bg_stroke.color = Color::WHITE.gamma_multiply(0.5);
                style.visuals.widgets.active.weak_bg_fill = Color::BLACK.gamma_multiply(0.25);
                style.visuals.widgets.noninteractive.bg_stroke.color =
                    Color::WHITE.gamma_multiply(0.5);

                ui.set_style(style);

                if ui
                    .add_sized(open_button_size, Button::new("⛭"))
                    .on_hover_text("Open Toolbar panel")
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked()
                {
                    if self.toolbar_panel == panel_opened_x {
                        self.toolbar_panel = panel_closed_x;
                    }
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
        value: i8,
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

        ui.put(
            rect,
            Button::new(scaled_size).set_pressed(if value > 0 { true } else { false }),
        )
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
            Rect::from_center_size(rect.center(), scaled_size.into()),
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

            let component_global_id =
                format!("{}:{}", ComponentKind::Potentiometer, potentiometer_id);
            let potentiometer_name =
                format!("{} {}", ComponentKind::Potentiometer, potentiometer_id);

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

            request_refresh_device();

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
    // 25 => value of the potentiometer which is just the starting value and doesn't update
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

    fn draw_debug_panel(&mut self, ctx: &Context) {
        use egui::*;

        let mut profiles = Default::default();
        let mut current_profile = String::new();

        if let Some(config) = &self.config {
            profiles = config.profiles.clone();
            current_profile = config.settings.current_profile.to_string();
        }

        Window::new("Debug")
            .default_pos((16.0, 16.0))
            .default_open(false)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.label(format!("Software Version: {}", APP_VERSION));

                if self.server_data.is_device_paired {
                    ui.label(format!(
                        "Firmware Version: {}",
                        self.server_data.firmware_version
                    ));
                }

                if ui.button("Restart application").clicked() {
                    restart();
                }

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

                #[cfg(debug_assertions)]
                {
                    ui.horizontal(|ui| {
                        ui.label("ASCII char");

                        let response =
                            ui.add(egui::TextEdit::singleline(&mut self.test_ascii_char_input));

                        if response.changed() {
                            // Keep only the last valid ASCII character
                            self.test_ascii_char_input.retain(|c| {
                                c.is_ascii()
                                    && !FORBIDDEN_CHARACTERS.contains(&c.to_string().as_str())
                            });

                            if self.test_ascii_char_input.len() > 1 {
                                self.test_ascii_char_input = self
                                    .test_ascii_char_input
                                    .chars()
                                    .last()
                                    .unwrap_or('\0')
                                    .to_string();
                            }
                        }
                    });

                    let xbm_data = vec![
                        0x00, 0x00, 0x00, 0x00, 0x00, 0xfc, 0x00, 0x00, 0xfe, 0x01, 0x00, 0xfc,
                        0x00, 0xe0, 0xff, 0x1f, 0x00, 0xfc, 0x00, 0xf8, 0xff, 0x7f, 0x00, 0xfc,
                        0x00, 0xfc, 0xff, 0xff, 0x00, 0xfc, 0x00, 0xff, 0xff, 0xff, 0x03, 0xfc,
                        0x80, 0xff, 0xff, 0xff, 0x07, 0xfc, 0xc0, 0xff, 0x03, 0xff, 0x0f, 0xfc,
                        0xe0, 0xff, 0x00, 0xfc, 0x1f, 0xfc, 0xe0, 0x7f, 0x00, 0xf8, 0x1f, 0xfc,
                        0xf0, 0x3f, 0x00, 0xf0, 0x3f, 0xfc, 0xf8, 0x3f, 0x00, 0xf0, 0x7f, 0xfc,
                        0xf8, 0x1f, 0x00, 0xe0, 0x7f, 0xfc, 0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc,
                        0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc, 0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc,
                        0xfc, 0x1f, 0x00, 0xe0, 0xff, 0xfc, 0xfe, 0x1f, 0x00, 0xe0, 0xff, 0xfd,
                        0xfe, 0x3f, 0x00, 0xf0, 0xff, 0xfd, 0xfe, 0x3f, 0x00, 0xf0, 0xff, 0xfd,
                        0xfe, 0x7f, 0x00, 0xf8, 0xff, 0xfd, 0xfe, 0xff, 0x00, 0xfc, 0xff, 0xfd,
                        0xfe, 0xff, 0x03, 0xff, 0xff, 0xfd, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xfd,
                        0xfe, 0xff, 0xff, 0xff, 0xff, 0xfd, 0xfc, 0xff, 0xff, 0xff, 0xff, 0xfc,
                        0xfc, 0xff, 0x00, 0xfc, 0xff, 0xfc, 0xfc, 0x0f, 0x00, 0xc0, 0xff, 0xfc,
                        0xfc, 0x01, 0x00, 0x00, 0xfe, 0xfc, 0xf8, 0x00, 0x00, 0x00, 0x7c, 0xfc,
                        0x78, 0x00, 0x00, 0x00, 0x78, 0xfc, 0x70, 0x00, 0x00, 0x00, 0x38, 0xfc,
                        0xe0, 0x00, 0x00, 0x00, 0x1c, 0xfc, 0xe0, 0x01, 0x00, 0x00, 0x1e, 0xfc,
                        0xc0, 0x03, 0x00, 0x00, 0x0f, 0xfc, 0x80, 0x0f, 0x00, 0xc0, 0x07, 0xfc,
                        0x00, 0x7f, 0x00, 0xf8, 0x03, 0xfc, 0x00, 0xfc, 0x03, 0xff, 0x00, 0xfc,
                        0x00, 0xf8, 0xff, 0x7f, 0x00, 0xfc, 0x00, 0xe0, 0xff, 0x1f, 0x00, 0xfc,
                        0x00, 0x00, 0xfe, 0x01, 0x00, 0xfc, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfc,
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
                                self.component_potentiometer_size,
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
                }

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
                            Color::LIGHT_BLUE,
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

                if ui.button("Auto resize").clicked() {
                    self.needs_resize = true;
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
                ui.label(format!("{} is under construction!", APP_NAME));
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

                ui.label(format!("Current profile: {}", current_profile));

                // Raw components layout
                ui.label(format!(
                    "Raw layout:\n- Buttons\n{}\n- Potentiometers\n{}",
                    self.server_data.raw_layout.0, self.server_data.raw_layout.1
                ));

                ui.separator();
                ui.group(|ui| {
                    ui.text_edit_multiline(&mut self.xbm_string);

                    if ui.button("Upload and Test").clicked() {
                        if self.server_data.is_device_paired {
                            let xbm_string = self.xbm_string.clone();

                            match extract_hex_bytes(&xbm_string, HOME_IMAGE_BYTES_SIZE) {
                                Ok(bytes) => {
                                    // `i` = *HOME* Image
                                    let data = format!("i{}", hex_bytes_vec_to_string(&bytes));

                                    request_device_upload(data, false).ok();

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
                                    // `i` = *HOME* Image, and since there's no value
                                    // the device removes current image and set its default
                                    request_device_upload("i".to_string(), false).ok();
                                },
                                |_app| {},
                                true,
                            );
                        } else {
                            self.show_not_paired_error();
                        }
                    }

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
                });
            });
    }

    // Application modals

    fn open_connection_modal(&mut self) {
        use egui::*;

        let mut device_name = String::new();

        if let Some(config) = &self.config {
            self.port_name.0 = config.settings.port_name.clone();
            self.port_name.1 = config.settings.device_name.is_empty();

            device_name = config.settings.device_name.clone();
        }

        self.baud_rate = self.baud_rate_string.parse().ok();

        self.show_custom_modal("connection-modal", move |ui, app| {
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

                        ui.label("Connection status");

                        ui.separator();
                    });

                    ui.add_space(ui.style().spacing.item_spacing.x * 2.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Connection"));
                        ui.add_space(ui.style().spacing.item_spacing.x * 3.25);
                        ui.label(
                            RichText::new(if app.server_data.is_device_paired {
                                "Connected"
                            } else {
                                "Not connected"
                            })
                            .color(
                                if app.server_data.is_device_paired {
                                    Color::GREEN
                                } else {
                                    Color::RED
                                },
                            ),
                        );
                    });

                    ui.add_space(ui.style().spacing.item_spacing.x);

                    if !device_name.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Device name"));
                            ui.add_space(ui.style().spacing.item_spacing.x * 2.0);
                            ui.label(
                                RichText::new(if let Some(config) = &app.config {
                                    config.settings.device_name.clone()
                                } else {
                                    "Loading...".to_string()
                                })
                                .color(Color::BLUE),
                            );

                            if ui
                                .small_button(RichText::new("✏"))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .on_hover_text("Edit device name")
                                .clicked()
                            {
                                app.open_set_device_name_modal();
                                app.set_width_modal(350.0);
                            }
                        });
                    }

                    ui.add_space(ui.style().spacing.item_spacing.x);

                    if app.server_data.is_device_paired {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Firmware"));
                            ui.add_space(ui.style().spacing.item_spacing.x * 5.65);
                            ui.label(
                                RichText::new(app.server_data.firmware_version.clone())
                                    .color(Color::BLUE),
                            );
                        });
                    }

                    ui.add_space(ui.style().spacing.item_spacing.x);

                    ui.scope(|ui| {
                        if !app.port_name.1 {
                            ui.disable();

                            if let Some(config) = &app.config {
                                if app.port_name.0 != config.settings.port_name {
                                    app.port_name.0 = config.settings.port_name.clone();
                                }
                            }
                        }

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 2.0);
                                ui.label("Port name");
                            });

                            ui.add_sized(
                                ui.available_size()
                                    - if app.port_name.1 {
                                        vec2(65.0, 0.0)
                                    } else {
                                        Vec2::ZERO
                                    },
                                TextEdit::singleline(&mut app.port_name.0).margin(vec2(8.0, 8.0)),
                            );

                            ui.scope(|ui| {
                                if app.port_name.0.is_empty() {
                                    ui.disable();
                                }

                                if app.port_name.1
                                    && ui
                                        .button("Save")
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                {
                                    if let Some(config) = &mut app.config {
                                        update_config_and_server(config, |c| {
                                            c.settings.port_name =
                                                app.port_name.0.clone().trim().to_string();
                                        });
                                    }

                                    app.server_needs_restart = true;

                                    app.show_message_modal(
                                        "port-name-updated-success",
                                        "Success".to_string(),
                                        "Port name was updated successfully!".to_string(),
                                    );
                                }
                            });
                        });
                    });

                    ui.add_space(ui.style().spacing.item_spacing.x);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 2.0);
                            ui.label("Baud rate ");
                        });

                        ui.add_space(2.0);

                        let baud_rate_response = ui.add_sized(
                            ui.available_size()
                                - if app.port_name.1 {
                                    vec2(65.0, 0.0)
                                } else {
                                    vec2(70.0, 0.0)
                                },
                            TextEdit::singleline(&mut app.baud_rate_string).margin(vec2(8.0, 8.0)),
                        );

                        if baud_rate_response.changed() {
                            app.baud_rate = app.baud_rate_string.parse().ok();
                        }

                        ui.scope(|ui| {
                            if app.baud_rate.is_none() {
                                ui.disable();
                            }

                            if ui
                                .button("Save")
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                if let Some(config) = &mut app.config {
                                    if let Some(baud_rate) = app.baud_rate {
                                        update_config_and_server(config, |c| {
                                            c.settings.baud_rate = baud_rate;
                                        });
                                    }
                                }

                                app.server_needs_restart = true;

                                app.show_message_modal(
                                    "baud-rate-updated-success",
                                    "Success".to_string(),
                                    "Baud rate was updated successfully!".to_string(),
                                );
                            }
                        });
                    });

                    if !device_name.is_empty() {
                        ui.vertical_centered_justified(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(ui.style().spacing.item_spacing.x / 2.0);

                                ui.group(|ui| {
                                    ui.label(
                                        RichText::new(
                                            "It's highly recommended to use \
                                            auto-detecting mode, however, if your device \
                                            doesn't support HID simulation, and cannot \
                                            have an HID device name, you can establish \
                                            connection by using manual mode, \
                                            and entering a port name!",
                                        )
                                        .color(Color::YELLOW.gamma_multiply(0.75))
                                        .size(13.5),
                                    );
                                });
                            });
                        });
                    }

                    ui.add_space(ui.style().spacing.item_spacing.x * 2.0);

                    ui.horizontal_top(|ui| {
                        let spacing = ui.spacing().item_spacing.x;

                        let total_width = ui.available_width();
                        let button_width = (total_width - spacing) / 2.0;

                        if ui
                            .add_sized([button_width, 0.0], egui::Button::new("Cancel"))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            app.close_modal();
                        }

                        if !device_name.is_empty()
                            && ui
                                .add_sized(
                                    [button_width, 0.0],
                                    egui::Button::new("Switch to Manual"),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                        {
                            app.show_yes_no_modal(
                                "serial-manual-connection-confirmation",
                                "Manual Device Connection".to_string(),
                                "You are about to set the device connection mode to \"manual\".\n\
                                You can switch back to \"auto-detecting\" mode at anytime.\n\
                                Are you sure you want to continue?"
                                    .to_string(),
                                |app| {
                                    // Close this modal and the connection modal
                                    app.close_modals(2);

                                    // Remove `device_name`
                                    if let Some(config) = &mut app.config {
                                        update_config_and_server(config, |c| {
                                            c.settings.device_name = String::new();
                                        });
                                    }

                                    app.server_needs_restart = true;

                                    // Re-open the connection modal to update data
                                    app.open_connection_modal();

                                    app.show_message_modal(
                                        "serial-manual-connection-success",
                                        "Success".to_string(),
                                        "Device name was cleared and serial connection \
                                        was set to manual mode.\nYou need to enter the port name!"
                                            .to_string(),
                                    );
                                },
                                |app| {
                                    app.close_modal();
                                },
                                false,
                            );

                            app.set_width_modal(375.0);
                        }

                        if device_name.is_empty()
                            && ui
                                .add_sized([button_width, 0.0], egui::Button::new("Auto-detect"))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                        {
                            app.show_yes_no_modal(
                                "serial-auto-connection-confirmation",
                                "Automatic Device Connection".to_string(),
                                "You are about to set the device \
                                connection mode to \"auto-detecting\".\n\
                                You can switch back to \"manual\" mode at anytime.\n\
                                Are you sure you want to continue?"
                                    .to_string(),
                                |app| {
                                    app.close_modal();

                                    // Add `device_name`
                                    app.open_set_device_name_modal();

                                    app.set_width_modal(350.0);
                                },
                                |app| {
                                    app.close_modal();
                                },
                                false,
                            );

                            app.set_width_modal(400.0);
                        }
                    });
                },
            );
        });
    }

    fn open_button_memory_manager_modal(&mut self) {
        use egui::*;

        self.button_memory.clear();

        if !self.server_data.is_device_paired {
            self.show_message_modal(
                "button-memory-manager-device-not-paired",
                "Error".to_string(),
                "Your device needs to be connected!".to_string(),
            );

            return;
        }

        for (button_id, button_normal, button_mod) in self.get_buttons() {
            let component_global_id = format!("{}:{}", ComponentKind::Button, button_id);

            if let Some(config) = &self.config {
                if let Some(layout) = &config.layout {
                    if layout.components.contains_key(&component_global_id) {
                        let is_modkey = button_normal == 255;

                        let corrected_mod = if is_modkey { 0 } else { button_mod };

                        self.button_memory.insert(
                            component_global_id,
                            (
                                (button_normal, (button_normal as char).to_string()),
                                (corrected_mod, (corrected_mod as char).to_string()),
                                is_modkey,
                            ),
                        );
                    }
                }
            }
        }

        self.show_custom_modal("button-memory-manager-modal", |ui, app| {
            ui.set_width(650.0);

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
                    ui.label("Button Memory Manager");
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.x);
            });

            // Content

            if app.button_memory.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.group(|ui| {
                        ui.label(
                            RichText::new("You haven't added any buttons to your layout!")
                                .color(Color::RED.gamma_multiply(0.75)),
                        );
                    });
                });

                ui.add_space(-ui.style().spacing.item_spacing.y);
            }

            let card_width = 216.0;

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.style().spacing.item_spacing.x * 1.5);

                        ui.horizontal_wrapped(|ui| {
                            for (button_id, _, _) in app.get_buttons() {
                                let component_global_id =
                                    format!("{}:{}", ComponentKind::Button, button_id);

                                let button_memory = app.button_memory.get_mut(&component_global_id);

                                if let Some(memory) = button_memory {
                                    ui.allocate_ui_with_layout(
                                        (card_width, ui.available_height()).into(),
                                        Layout::left_to_right(Align::Center),
                                        |ui| {
                                            ui.vertical(|ui| {
                                                ui.group(|ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.label(
                                                            RichText::new(component_global_id)
                                                                .color(Color32::GRAY)
                                                                .size(16.0),
                                                        );

                                                        ui.add_space(
                                                            ui.style().spacing.item_spacing.x,
                                                        );

                                                        let modkey_response = ui
                                                            .checkbox(
                                                                &mut memory.2,
                                                                RichText::new("ModKey?")
                                                                    .color(Color32::GRAY)
                                                                    .size(16.0),
                                                            )
                                                            .on_hover_text(
                                                                RichText::new(
                                                                    "Assigns this button as \
                                                                    the modifier.",
                                                                )
                                                                .size(18.0),
                                                            );

                                                        if modkey_response.changed() {
                                                            if memory.2 {
                                                                memory.0 = (
                                                                    255,
                                                                    (255 as char).to_string(),
                                                                );
                                                                memory.1 =
                                                                    (0, (0 as char).to_string());
                                                            } else {
                                                                memory.0 = (0, String::new());
                                                                memory.1 = (0, String::new());
                                                            }
                                                        }
                                                    });

                                                    if memory.2 {
                                                        ui.horizontal_centered(|ui| {
                                                            ui.allocate_ui_with_layout(
                                                                (
                                                                    ui.available_width()
                                                                        - ui.style()
                                                                            .spacing
                                                                            .item_spacing
                                                                            .x
                                                                            * 1.5
                                                                        - 1.0,
                                                                    ui.available_height(),
                                                                )
                                                                    .into(),
                                                                Layout::centered_and_justified(
                                                                    Direction::TopDown,
                                                                ),
                                                                |ui| {
                                                                    ui.group(|ui| {
                                                                        ui.label(
                                                                            RichText::new("MODKEY")
                                                                                .color(Color::GREEN)
                                                                                .size(23.0),
                                                                        )
                                                                        .on_hover_text(
                                                                            RichText::new(
                                                                                "This is \
                                                                            a modifier key. When \
                                                                            held down, pressing \
                                                                            other buttons will \
                                                                            trigger an alternative \
                                                                            action.",
                                                                            )
                                                                            .size(16.0),
                                                                        );
                                                                    });
                                                                },
                                                            );
                                                        });

                                                        return;
                                                    }

                                                    ui.horizontal(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.add_space(
                                                                ui.style().spacing.item_spacing.x
                                                                    / 2.0
                                                                    + 2.0,
                                                            );
                                                            ui.label("Key").on_hover_text(
                                                                "Pressing the button",
                                                            );
                                                        });

                                                        let key_response = ui.add_sized(
                                                            (48.0, ui.available_height()),
                                                            TextEdit::singleline(&mut memory.0 .1)
                                                                .margin(vec2(8.0, 8.0))
                                                                .horizontal_align(Align::Center),
                                                        );

                                                        if key_response.changed() {
                                                            // Keep only the last valid character in range 0-255
                                                            memory.0 .1.retain(|c| {
                                                                ((c as u32) < 256)
                                                                    && !FORBIDDEN_CHARACTERS
                                                                        .contains(
                                                                            &c.to_string().as_str(),
                                                                        )
                                                            });

                                                            // Keep only the last valid character
                                                            memory.0 .1 = memory
                                                                .0
                                                                 .1
                                                                .chars()
                                                                .last()
                                                                .unwrap_or('\0')
                                                                .to_string();
                                                            memory.0 .0 = memory
                                                                .0
                                                                 .1
                                                                .chars()
                                                                .next()
                                                                .unwrap_or('\0')
                                                                as u8;
                                                        }

                                                        ui.vertical(|ui| {
                                                            ui.add_space(
                                                                ui.style().spacing.item_spacing.x
                                                                    / 2.0
                                                                    + 2.0,
                                                            );
                                                            ui.label("Mod").on_hover_text(
                                                                "Pressing the button \
                                                                while holding ModKey",
                                                            );
                                                        });

                                                        let key_response = ui.add_sized(
                                                            (48.0, ui.available_height()),
                                                            TextEdit::singleline(&mut memory.1 .1)
                                                                .margin(vec2(8.0, 8.0))
                                                                .horizontal_align(Align::Center),
                                                        );

                                                        if key_response.changed() {
                                                            // Keep only the last valid character in range 0-255
                                                            memory.1 .1.retain(|c| {
                                                                ((c as u32) < 256)
                                                                    && !FORBIDDEN_CHARACTERS
                                                                        .contains(
                                                                            &c.to_string().as_str(),
                                                                        )
                                                            });

                                                            // Keep only the last valid character
                                                            memory.1 .1 = memory
                                                                .1
                                                                 .1
                                                                .chars()
                                                                .last()
                                                                .unwrap_or('\0')
                                                                .to_string();
                                                            memory.1 .0 = memory
                                                                .1
                                                                 .1
                                                                .chars()
                                                                .next()
                                                                .unwrap_or('\0')
                                                                as u8;
                                                        }
                                                    });
                                                });
                                            });
                                        },
                                    );
                                }
                            }
                        });
                    });
                });

            ui.add_space(ui.spacing().item_spacing.x * 1.0);

            ui.horizontal(|ui| {
                ui.add_space(35.0);

                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "⚫If some buttons are missing, make sure \
                                you've added them to your layout.",
                            )
                            .color(Color::YELLOW.gamma_multiply(0.75))
                            .size(16.0),
                        );

                        ui.label(
                            egui::RichText::new(
                                "\n⚫If you want to set software-based interaction, \
                                you can leave the field empty.",
                            )
                            .color(Color::YELLOW.gamma_multiply(0.75))
                            .size(16.0),
                        );

                        ui.label(
                            egui::RichText::new(
                                "\n⚫Your device's flash memory has a limited number of \
                                write/erase cycles.\n\t\t\
                                Excessive writing can shorten its lifespan. \
                                (usually 10,000-100,000)",
                            )
                            .color(Color::YELLOW.gamma_multiply(0.75))
                            .size(16.0),
                        );

                        ui.label(
                            egui::RichText::new(
                                "\n⚫These keyboard shortcuts are only available when \
                                the current profile is set to\n\t\t\
                                the device's internal profile.",
                            )
                            .color(Color::YELLOW.gamma_multiply(0.75))
                            .size(16.0),
                        );
                    });
                });
            });

            ui.add_space(ui.spacing().item_spacing.x * 1.5);

            ui.vertical_centered(|ui| {
                ui.set_max_width(350.0);
                ui.horizontal_top(|ui| {
                    let spacing = ui.spacing().item_spacing.x;

                    let total_width = ui.available_width();
                    let button_width = (total_width - spacing) / 2.0;

                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("Close"))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        app.close_modal();
                    }

                    if app.button_memory.is_empty() {
                        ui.disable();
                    }

                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("Save to Device"))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        // Upload new memory button layout to device
                        // Format: id:key|mod; e.g. 1:98|112;2:99|113;
                        let mut data = String::new();

                        for (button_id, (button_key, button_mod, _)) in app.button_memory.iter() {
                            let id: u8 = button_id
                                .split(SERIAL_MESSAGE_SEP)
                                .last()
                                .unwrap_or("0")
                                .parse()
                                .unwrap_or(0);

                            if id < 1 {
                                continue;
                            }

                            data.push_str(id.to_string().as_str());
                            data.push_str(SERIAL_MESSAGE_SEP);
                            data.push_str(button_key.0.to_string().as_str());
                            data.push_str(SERIAL_MESSAGE_INNER_SEP);
                            data.push_str(button_mod.0.to_string().as_str());
                            data.push_str(SERIAL_MESSAGE_END);
                        }

                        app.show_yes_no_modal(
                            "button-memory-manager-upload-to-device-confirmation",
                            "Update to Device".to_string(),
                            "You're about to upload this button memory layout to you device!\n\
                            Are you sure you want to continue?"
                                .to_string(),
                            move |app| {
                                // `b` = buttons_layout
                                request_device_upload(format!("b{}", data), true).ok();

                                // Close this modal and the `ButtonMemoryManger` modal
                                app.close_modals(1);

                                app.show_message_modal(
                                    "button-memory-manager-upload-to-device",
                                    "Success".to_string(),
                                    "New button memory layout was successfully uploaded to your \
                                    device."
                                        .to_string(),
                                )
                            },
                            |app| {
                                app.close_modal();
                            },
                            false,
                        );
                    }
                });
            });
        });

        self.set_can_close_modal(false);
    }

    fn open_set_device_name_modal(&mut self) {
        use egui::*;

        self.show_custom_modal("serial-auto-set-device-name", |ui, app| {
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

                        ui.label("Set your device name");

                        ui.separator();
                    });

                    ui.add_space(20.0);

                    let mut can_save = false;

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 1.0);
                            ui.label("Name");
                        });

                        ui.add_sized(
                            ui.available_size(),
                            TextEdit::singleline(&mut app.device_name).margin(vec2(8.0, 8.0)),
                        );

                        if app.device_name.is_empty() {
                            can_save = false;
                        } else {
                            can_save = true;
                        }
                    });

                    ui.add_space(8.0);

                    ui.vertical_centered_justified(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.group(|ui| {
                                ui.label(
                                    RichText::new(
                                        "By default, the app prioritizes \
                                        connecting using the port name. \
                                        However, if you enter your device \
                                        name and the app fails to connect \
                                        via the specified port, it will \
                                        attempt to connect using the \
                                        device name instead. \
                                        If successful, the port name will \
                                        be updated accordingly.",
                                    )
                                    .color(Color32::GRAY)
                                    .size(13.5),
                                );
                            });
                        });
                    });

                    ui.add_space(16.0);

                    ui.horizontal_top(|ui| {
                        let spacing = ui.spacing().item_spacing.x;

                        let total_width = ui.available_width();
                        let button_width = (total_width - spacing) / 2.0;

                        if ui
                            .add_sized([button_width, 0.0], Button::new("Cancel"))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            app.close_modal();
                        }

                        ui.scope(|ui| {
                            if !can_save {
                                ui.disable();
                            }

                            if ui
                                .add_sized([button_width, 0.0], Button::new("Save"))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                // Close this modal and the connection modal
                                app.close_modals(2);

                                if let Some(config) = &mut app.config {
                                    update_config_and_server(config, |c| {
                                        c.settings.device_name =
                                            app.device_name.clone().trim().to_string();
                                    });
                                }

                                app.server_needs_restart = true;

                                // Re-open the connection modal to update data
                                app.open_connection_modal();

                                app.show_message_modal(
                                    "serial-auto-connection-success",
                                    "Success".to_string(),
                                    "Serial connection mode \
                                    was set to auto-detecting."
                                        .to_string(),
                                );
                            }
                        });
                    });
                },
            );
        });
    }

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
                            ui.label("Edit Layout");
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
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            app.close_modal();
                        }

                        if ui
                            .add_sized([button_width, 0.0], Button::new(create_update_button_name))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            if let Some(config) = &mut app.config {
                                // Check if the entered size can fit the user's monitor
                                let monitor_size = ui
                                    .ctx()
                                    .input(|i: &egui::InputState| i.viewport().monitor_size);

                                if let Some(size) = monitor_size {
                                    if app.new_layout_size.0 + (APP_PADDING_X * 2) as f32 > size.x
                                        || app.new_layout_size.1 + (APP_PADDING_Y * 2) as f32
                                            > size.y
                                    {
                                        app.show_message_modal(
                                            "layout-create-update-size-too-big",
                                            "Error".to_string(),
                                            format!(
                                                "Your current monitor supports a maximum layout \
                                                size of {}x{}.",
                                                size.x - (APP_PADDING_Y * 2) as f32,
                                                size.y - (APP_PADDING_Y * 2) as f32
                                            ),
                                        );

                                        return;
                                    }
                                }

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
                                        (You will keep your components)"
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

    fn open_layout_settings_modal(&mut self) {
        if self.is_editing_layout {
            self.show_message_modal(
                "layout-settings-is-editing-error",
                "Error".to_string(),
                "Please save your current changes by clicking the (🖴) button first!".to_string(),
            );

            return;
        }

        self.show_custom_modal("layout-settings-modal", |ui, app| {
            ui.set_max_width(350.0);

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
                    ui.label("Layout Settings");
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.x);
            });

            ui.label("Layout");

            ui.add_space(ui.style().spacing.item_spacing.y / 2.0);

            ui.horizontal_top(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Auto-size Layout"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(
                        "Automatically adjusts the layout size to fit all components \
                        with a small padding",
                    )
                    .clicked()
                {
                    app.show_yes_no_modal(
                        "resize-layout-to-fit-components",
                        "Resizing Layout".to_string(),
                        "This will adjust the layout size to fit components with a small padding\n\
                        Are you sure you want to continue?"
                            .to_string(),
                        |app| app.resize_layout_to_fit_components(),
                        |_app| {},
                        true,
                    );
                }

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Customize Layout"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(
                        "Rename your layout and adjust its dimensions to better fit your needs",
                    )
                    .clicked()
                {
                    app.open_create_update_layout_modal();
                }
            });

            ui.separator();

            ui.add_space(ui.style().spacing.item_spacing.y * 2.0);

            ui.label("Components");

            ui.add_space(ui.style().spacing.item_spacing.y / 2.0);

            ui.horizontal_top(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Align to Center"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(
                        "Moves all components to be centered within \
                        the layout while maintaining their relative positions",
                    )
                    .clicked()
                {
                    app.show_yes_no_modal(
                        "components-center-to-layout",
                        "Aligning Components".to_string(),
                        "This will move all components to the center while \
                        keeping their relative positions.\n\
                        Are you sure you want to continue?"
                            .to_string(),
                        |app| app.center_components_to_layout(),
                        |_app| {},
                        true,
                    );
                }

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Align to Top-Left"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(
                        "Moves all components to the top-left corner of the layout \
                        while keeping their relative positions.\n\
                        Useful for grid snapping after centering",
                    )
                    .clicked()
                {
                    app.show_yes_no_modal(
                        "components-top-left-to-layout",
                        "Aligning Components".to_string(),
                        "This will reposition all components to the top-left \
                        corner while maintaining their layout.\n\
                        Are you sure you want to continue?"
                            .to_string(),
                        |app| app.top_left_components_to_layout(),
                        |_app| {},
                        true,
                    );
                }
            });

            ui.separator();

            ui.add_space(ui.style().spacing.item_spacing.y * 2.0);

            ui.vertical_centered(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Close"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    app.close_modal();
                }
            });
        });
    }

    fn open_create_update_profile_modal(&mut self, is_updating: bool) {
        use egui::*;

        if let Some(config) = &self.config {
            if !is_updating {
                // Clear the last_profile_name
                self.last_profile_name = String::new();
                self.new_profile_name = String::new();
            }

            self.profile_exists = config.does_profile_exist(&self.new_profile_name);
        }

        self.show_custom_modal("create-update-profile", move |ui, app| {
            ui.set_width(285.0);

            ui.vertical_centered(|ui| {
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
                        ui.label("Edit Profile");
                    }

                    ui.separator();
                });
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

                    let button_size = vec2(if is_updating { 42.0 } else { 0.0 }, 0.0);

                    let name_response = ui.add_sized(
                        ui.available_size() - button_size, /* accouting for the delete button */
                        TextEdit::singleline(&mut app.new_profile_name).margin(vec2(8.0, 8.0)),
                    );

                    if is_updating
                        && ui
                            .button("🗑")
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text("Delete this profile")
                            .clicked()
                    {
                        // Delete this profile
                        let profile_name = app.last_profile_name.clone();

                        app.show_yes_no_modal(
                            "profile-delete-confirmation",
                            "Deleting Profile".to_string(),
                            format!(
                                "You're about to delete \"{}\"\n\
                            Are you sure you want to continue?",
                                profile_name
                            ),
                            move |app| {
                                // Close this and the update profile modals
                                app.close_modals(2);

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

                    if app.new_profile_name.len() > DASHBOARD_PROFILE_MAX_CHARACTERS {
                        app.new_profile_name
                            .truncate(DASHBOARD_PROFILE_MAX_CHARACTERS);
                    }

                    if name_response.changed() {
                        if let Some(config) = &app.config {
                            app.profile_exists = config.does_profile_exist(&app.new_profile_name);
                        }
                    }
                });

                if app.profile_exists {
                    ui.label(
                        RichText::new("A profile with this name already exists!")
                            .size(16.5)
                            .color(Color::RED),
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
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
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
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
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

                // Force disable the editing mode to avoid bugs if user didn't save
                app.is_editing_layout = false;
                app.components_backup = Default::default();

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

    fn open_component_properties_modal(&mut self, component_global_id: String) {
        let update_component_properties =
            |id: &String, properties: &mut Component, config: &mut Option<Config>| {
                if let Some(c) = config {
                    if let Some(layout) = &mut c.layout {
                        if let Some(component) = layout.components.get_mut(id) {
                            if component.label != properties.label {
                                component.label = properties.label.clone().trim().to_string();
                            }

                            if component.scale != properties.scale {
                                component.scale = properties.scale;
                            }

                            if component.style != properties.style {
                                component.style = properties.style;
                            }
                        }
                    }
                }
            };

        let update_component_interactions =
            |id: &String, interactions: &mut Interaction, config: &mut Option<Config>| {
                if let Some(c) = config {
                    let current_profile = &mut c.profiles[c.settings.current_profile];

                    if let Some(i) = current_profile.interactions.get_mut(id) {
                        if i.normal != interactions.normal {
                            i.normal = interactions.normal.clone();
                        }

                        if i.modkey != interactions.modkey {
                            i.modkey = interactions.modkey.clone();
                        }
                    }
                }
            };

        if let Some(config) = &self.config {
            let current_profile = &config.profiles[config.settings.current_profile];

            if let Some(layout) = &config.layout {
                self.component_properties.0 = layout.components.get(&component_global_id).cloned();
            }

            let current_interactions = current_profile
                .interactions
                .get(&component_global_id)
                .cloned();

            self.component_properties.1 = current_interactions.clone();

            if let Some(interactions) = current_interactions {
                match &interactions.normal {
                    InteractionKind::Shortcut(_keys, text) => {
                        self.properties_shortcut_kind.0 = text.is_empty();
                    }
                    _ => (),
                }

                match &interactions.modkey {
                    InteractionKind::Shortcut(_keys, text) => {
                        self.properties_shortcut_kind.1 = text.is_empty();
                    }
                    _ => (),
                }
            } else {
                self.properties_shortcut_kind = (false, false);
            };
        }

        self.properties_selected_interaction = true;

        self.show_custom_modal("component-properties-modal", move |ui, app| {
            let mut current_profile_name = String::new();
            let (kind_string, id_string) = component_global_id
                .split_once(SERIAL_MESSAGE_SEP)
                .unwrap_or(("", ""));

            if kind_string.is_empty() || id_string.is_empty() {
                app.close_modal();

                app.show_message_modal(
                    "properties-modal-error",
                    "Error".to_string(),
                    "There was an error while loading component's properties!".to_string(),
                );

                return;
            }

            let kind = match kind_string {
                "Button" => ComponentKind::Button,
                "LED" => ComponentKind::LED,
                "Potentiometer" => ComponentKind::Potentiometer,
                "Joystick" => ComponentKind::Joystick,
                "RotaryEncoder" => ComponentKind::RotaryEncoder,
                "Display" => ComponentKind::Display,
                _ => ComponentKind::None,
            };

            let interactable = match kind {
                ComponentKind::None => false,
                ComponentKind::Button => true,
                ComponentKind::LED => false,
                ComponentKind::Potentiometer => true,
                ComponentKind::Joystick => false,
                ComponentKind::RotaryEncoder => false,
                ComponentKind::Display => false,
            };

            // For now, only `Buttons` can have modkey interaction
            let modkey_interaction = match kind {
                ComponentKind::None => false,
                ComponentKind::Button => true,
                ComponentKind::LED => false,
                ComponentKind::Potentiometer => false,
                ComponentKind::Joystick => false,
                ComponentKind::RotaryEncoder => false,
                ComponentKind::Display => false,
            };

            // Does the component have a value? e.g. potentiometer has 0-99
            // returns (bool, &str) -> `true/false`, `hint_text`
            let has_value = match kind {
                ComponentKind::None => (false, ""),
                ComponentKind::Button => (false, ""),
                ComponentKind::LED => (false, ""),
                ComponentKind::Potentiometer => (true, "value is 0-99"),
                ComponentKind::Joystick => (false, ""),
                ComponentKind::RotaryEncoder => (false, ""),
                ComponentKind::Display => (false, ""),
            };

            // Check if component have multiple styles
            let multiple_styles = match kind {
                ComponentKind::None => 0,
                ComponentKind::Button => Button::STYLES_COUNT,
                ComponentKind::LED => LED::STYLES_COUNT,
                ComponentKind::Potentiometer => Potentiometer::STYLES_COUNT,
                ComponentKind::Joystick => Joystick::STYLES_COUNT,
                ComponentKind::RotaryEncoder => RotaryEncoder::STYLES_COUNT,
                ComponentKind::Display => GLCD::STYLES_COUNT,
            };

            let mut is_internal_profile = false;

            if let Some(config) = &app.config {
                is_internal_profile = config.settings.current_profile == 0;

                let current_profile = &config.profiles[config.settings.current_profile];

                current_profile_name = current_profile.name.clone();
            }

            // `button_memory.0` = normal, `button_memory.1` = mod
            let mut button_memory = (false, false);

            // Retrieve buttons memory
            if kind == ComponentKind::Button {
                for (button_id, button_normal, button_mod) in app.get_buttons() {
                    if button_id == id_string.parse::<u8>().unwrap_or(0) {
                        // Return if this is the modkey
                        if button_normal == 255 {
                            app.close_modal();

                            app.show_custom_modal("properties-modkey-modal", |ui, app| {
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
                                        ui.label("Info");
                                    });

                                    ui.separator();

                                    ui.add_space(ui.spacing().item_spacing.x);
                                });

                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        "This component is a \"ModKey\"\n\
                                        You can change this functionality from",
                                    );

                                    if ui
                                        .add(
                                            egui::Label::new(
                                                egui::RichText::new("Button Memory Manager")
                                                    .color(Color::BLUE)
                                                    .underline(),
                                            )
                                            .sense(egui::Sense::click()),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        app.close_modal();
                                        app.open_button_memory_manager_modal();
                                    }
                                });

                                ui.add_space(ui.spacing().item_spacing.x * 1.5);

                                ui.vertical_centered(|ui| {
                                    let spacing = ui.spacing().item_spacing.x;

                                    let total_width = ui.available_width();
                                    let button_width = (total_width - spacing) / 2.0;

                                    if ui
                                        .add_sized([button_width, 0.0], egui::Button::new("Ok"))
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        app.close_modal();
                                    }
                                });
                            });

                            app.set_width_modal(290.0);

                            return;
                        }

                        // 0 is nothing and 255 is reserved for modkey
                        if button_normal != 0 && button_normal != 255 {
                            button_memory.0 = true;
                        }
                        if button_mod != 0 && button_mod != 255 {
                            button_memory.1 = true;
                        }

                        break;
                    }
                }
            }

            ui.set_width(375.0);

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

                ui.horizontal(|ui| {
                    if !interactable {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new("ℹ")
                                    .color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                            )
                            .sense(egui::Sense::hover()),
                        )
                        .on_hover_cursor(egui::CursorIcon::Help)
                        .on_hover_text(
                            egui::RichText::new(
                                "This component is decorative and \n\
                                cannot be assigned any interactions",
                            )
                            .color(Color::LIGHT_BLUE)
                            .size(16.0),
                        );
                    }

                    ui.vertical_centered(|ui| {
                        ui.label("Properties");
                    });
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.y / 2.0);
            });

            ui.add_space(ui.spacing().item_spacing.y / 2.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Component"));
                ui.add_space(ui.style().spacing.item_spacing.x * 4.0);
                ui.label(egui::RichText::new(component_global_id.clone()).color(Color::BLUE));

                let component_id = component_global_id.clone();

                if ui
                    .small_button("✏")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    let (kind_string, id_string) = component_global_id
                        .split_once(SERIAL_MESSAGE_SEP)
                        .unwrap_or(("", ""));

                    let id: u8 = id_string.parse().unwrap_or(0);

                    app.open_update_component_id_modal(kind_string.to_string(), id);
                }

                if ui
                    .small_button("🗑")
                    .on_hover_text("Delete this component")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    app.show_yes_no_modal(
                        "component-delete-confirmation-modal",
                        "Delete Component".to_string(),
                        "You are about to delete this component!\n\
                        Are you sure you want to continue?"
                            .to_string(),
                        move |app| {
                            app.delete_component(&component_id);
                            app.close_modal();

                            return;
                        },
                        |_app| {},
                        true,
                    );
                }
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Profile"));
                ui.add_space(ui.style().spacing.item_spacing.x * 9.75);
                ui.label(egui::RichText::new(current_profile_name).color(Color::BLUE));
            });

            let properties = if let Some(p) = &mut app.component_properties.0 {
                p
            } else {
                ui.vertical_centered_justified(|ui| {
                    ui.label("Loading Properties...");
                });

                return;
            };

            let mut should_open_display_icon_manager = (false, None);

            // Since the value for `Display` icon is stored in its label, we won't show
            // the `label` field for the `Display` component
            if kind != ComponentKind::Display {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 1.0);
                        ui.label("Label");
                    });

                    let label_response = ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::singleline(&mut properties.label)
                            .margin(Vec2::new(8.0, 8.0)),
                    );

                    if label_response.lost_focus() {
                        update_component_properties(
                            &component_global_id,
                            properties,
                            &mut app.config,
                        );
                    }
                });
            } else {
            }

            ui.horizontal_top(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let input_width = (total_width - (spacing * 4.0)) / 2.0;

                ui.allocate_ui_with_layout(
                    (input_width, 0.0).into(),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.vertical(|ui| {
                                ui.add_space(spacing / 2.0 + 1.0);
                                ui.label("Scale");
                            });

                            let scale_response = ui.add_sized(
                                ui.available_size(),
                                DragValue::new(&mut properties.scale)
                                    .speed(0.1)
                                    .range(0.1..=5.0)
                                    .clamp_existing_to_range(true),
                            );

                            if scale_response.changed() {
                                update_component_properties(
                                    &component_global_id,
                                    properties,
                                    &mut app.config,
                                );
                            }
                        });
                    },
                );

                if kind == ComponentKind::Display {
                    if ui
                        .add_sized(
                            (input_width + spacing, 0.0),
                            egui::Button::new("Display Icon Manager"),
                        )
                        .clicked()
                    {
                        let xbm_data = match hex_bytes_string_to_vec(&properties.label) {
                            Ok(bytes) => bytes,
                            Err(_) => vec![],
                        };

                        should_open_display_icon_manager = (true, Some(xbm_data));
                    }
                }

                if multiple_styles > 0 {
                    ui.add_space(16.0);

                    ui.allocate_ui_with_layout(
                        (input_width + spacing, 0.0).into(),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.horizontal_centered(|ui| {
                                ui.vertical(|ui| {
                                    ui.add_space(spacing / 2.0 + 2.0);
                                    ui.label("Style");
                                });

                                let style_response = ui.add_sized(
                                    ui.available_size(),
                                    DragValue::new(&mut properties.style)
                                        .speed(1)
                                        .range(0..=multiple_styles - 1)
                                        .clamp_existing_to_range(true),
                                );

                                if style_response.changed() {
                                    update_component_properties(
                                        &component_global_id,
                                        properties,
                                        &mut app.config,
                                    );
                                }
                            });
                        },
                    );
                }
            });

            if should_open_display_icon_manager.0 {
                app.open_update_display_image_modal(
                    should_open_display_icon_manager.1.unwrap(),
                    component_global_id.clone(),
                );
            }

            if !interactable {
                return;
            }

            let interactions = if let Some(i) = &mut app.component_properties.1 {
                i
            } else {
                ui.vertical_centered_justified(|ui| {
                    ui.label("Loading interactions...");
                });

                return;
            };

            ui.separator();

            if is_internal_profile && !app.server_data.is_device_paired {
                ui.vertical_centered(|ui| {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("Warning")
                                .color(Color::YELLOW)
                                .size(20.0),
                        )
                        .on_hover_cursor(egui::CursorIcon::Help)
                        .on_hover_ui(|ui| {
                            ui.set_max_width(320.0);

                            ui.label(
                                egui::RichText::new(
                                    "The device is not currently paired, so the app cannot \
                                    determine whether it has button memory. To ensure proper \
                                    functionality, connect your device before adding \
                                    interactions to any components.",
                                )
                                .color(Color::YELLOW.gamma_multiply(0.75))
                                .size(13.5),
                            );
                        });
                    });
                });
            }

            let mut should_open_button_memory_manager = false;

            ui.allocate_ui_with_layout(
                (ui.available_width(), ui.available_height()).into(),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Interactions

                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Interaction{}",
                            if modkey_interaction { "s" } else { "" }
                        ));

                        if modkey_interaction {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("ℹ")
                                        .color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                                )
                                .sense(egui::Sense::hover()),
                            )
                            .on_hover_cursor(egui::CursorIcon::Help)
                            .on_hover_text(
                                egui::RichText::new(
                                    "You can trigger an alternate action with\n\
                                this component by holding ModKey.",
                                )
                                .color(Color::LIGHT_BLUE)
                                .size(16.0),
                            );
                        }
                    });

                    if modkey_interaction {
                        ui.horizontal_top(|ui| {
                            let spacing = ui.spacing().item_spacing.x;

                            let total_width = ui.available_width();
                            let button_width = (total_width - spacing) / 2.0;

                            if ui
                                .add_sized(
                                    [button_width, 0.0],
                                    egui::Button::new("Normal")
                                        .fill(if app.properties_selected_interaction {
                                            Color::ACCENT.gamma_multiply(0.5)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .stroke(egui::Stroke::new(
                                            2.0,
                                            if app.properties_selected_interaction {
                                                Color::ACCENT.gamma_multiply(0.1)
                                            } else {
                                                egui::Color32::WHITE.gamma_multiply(0.25)
                                            },
                                        )),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_ui(|ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            "Activating this component will\ntrigger an action.",
                                        )
                                        .color(egui::Color32::GRAY)
                                        .size(15.0),
                                    );
                                })
                                .clicked()
                            {
                                app.properties_selected_interaction = true;
                            }

                            if ui
                                .add_sized(
                                    [button_width, 0.0],
                                    egui::Button::new("Alternative")
                                        .fill(if !app.properties_selected_interaction {
                                            Color::ACCENT.gamma_multiply(0.5)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .stroke(egui::Stroke::new(
                                            2.0,
                                            if !app.properties_selected_interaction {
                                                Color::ACCENT.gamma_multiply(0.1)
                                            } else {
                                                egui::Color32::WHITE.gamma_multiply(0.25)
                                            },
                                        )),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_ui(|ui| {
                                    ui.label(
                                        egui::RichText::new(
                                            "Activating this component while\nholding ModKey \
                                            will trigger an\nalternative action.",
                                        )
                                        .color(egui::Color32::GRAY)
                                        .size(15.0),
                                    );
                                })
                                .clicked()
                            {
                                app.properties_selected_interaction = false;
                            }
                        });
                    }

                    ui.add_space(-ui.style().spacing.item_spacing.y);

                    let hid_warning_normal_condition = kind == ComponentKind::Button
                        && interactions.normal != InteractionKind::None()
                        && is_internal_profile
                        && button_memory.0;

                    let hid_warning_modkey_condition = kind == ComponentKind::Button
                        && interactions.modkey != InteractionKind::None()
                        && is_internal_profile
                        && button_memory.1;

                    let mut should_update_interactions = false;

                    if app.properties_selected_interaction {
                        draw_normal_interaction_panel(
                            ui,
                            interactions,
                            has_value,
                            hid_warning_normal_condition,
                            &mut app.properties_shortcut_kind,
                            &mut app.properties_shortcut_key_filter,
                            &mut should_open_button_memory_manager,
                            &mut should_update_interactions,
                        );
                    } else {
                        // Check if this component supports modkey interaction
                        if modkey_interaction {
                            draw_modkey_interaction_panel(
                                ui,
                                interactions,
                                has_value,
                                hid_warning_modkey_condition,
                                &mut app.properties_shortcut_kind,
                                &mut app.properties_shortcut_key_filter,
                                &mut should_open_button_memory_manager,
                                &mut should_update_interactions,
                            );
                        }
                    }

                    if should_update_interactions {
                        update_component_interactions(
                            &component_global_id,
                            interactions,
                            &mut app.config,
                        );
                    }
                },
            );

            if should_open_button_memory_manager {
                app.close_modal();
                app.open_button_memory_manager_modal();
            }
        });
    }

    fn open_update_component_id_modal(&mut self, kind: String, id: u8) {
        use egui::*;

        if id < 1 || kind.is_empty() {
            self.show_message_modal(
                "component-update-id-unknown-id",
                "Error".to_string(),
                "Could not retrieve id! Please try again.".to_string(),
            );

            self.set_width_modal(250.0);

            return;
        }

        self.component_id = id;

        self.show_custom_modal("component-id-update-modal", move |ui, app| {
            ui.set_max_width(265.0);

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
                    ui.label("Update Id");
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.x);
            });

            // Content

            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Make sure to enter a valid component ID that your device \
                        can recognize based on the component type.",
                    )
                    .color(Color::YELLOW.gamma_multiply(0.75))
                    .size(13.5),
                );
            });

            let mut can_save = false;

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(ui.style().spacing.item_spacing.x / 2.0 + 1.0);
                    ui.label(format!("{} Id ", kind));
                });

                let response = ui.add_sized(
                    ui.available_size(),
                    DragValue::new(&mut app.component_id).speed(1),
                );

                if response.changed() {
                    if let Some(config) = &app.config {
                        if let Some(layout) = &config.layout {
                            let current_id = format!("{}:{}", kind, &app.component_id);

                            if layout.components.contains_key(&current_id) {
                                app.component_id_exists = true;
                            } else {
                                app.component_id_exists = false;
                            }
                        }
                    }
                }

                if app.component_id > 0 && !app.component_id_exists {
                    can_save = true;
                } else {
                    can_save = false;
                }
            });

            if app.component_id_exists {
                ui.label(
                    RichText::new(format!("A {} with this Id already exists!", kind))
                        .size(16.5)
                        .color(Color::RED),
                );
            }

            ui.add_space(ui.spacing().item_spacing.x * 2.5);

            ui.horizontal_top(|ui| {
                let spacing = ui.spacing().item_spacing.x;

                let total_width = ui.available_width();
                let button_width = (total_width - spacing) / 2.0;

                ui.scope(|ui| {
                    if !can_save {
                        ui.disable();
                    }

                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("Save"))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        let new_id = format!("{}:{}", kind, app.component_id);

                        if let Some(config) = &mut app.config {
                            let last_id = format!("{}:{}", kind, id);

                            if let Some(layout) = &mut config.layout {
                                if let Some(component) = layout.components.remove(&last_id) {
                                    layout.components.insert(new_id.clone(), component);
                                }
                            }

                            for profile in &mut config.profiles {
                                if let Some(component) = profile.interactions.remove(&last_id) {
                                    profile.interactions.insert(new_id.clone(), component);
                                }
                            }
                        }

                        // Procedure: Close modals -> Disable edit mode -> Re-enable edit mode ->
                        // Open properties modal with new info
                        app.close_modals(2);

                        app.open_component_properties_modal(new_id);
                    }
                });

                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("Close"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    app.close_modal();
                }
            });
        });
    }

    fn open_update_display_image_modal(&mut self, xbm_data: Vec<u8>, component_global_id: String) {
        self.current_display_image = xbm_data;

        self.show_custom_modal("display-image-update-modal", move |ui, app| {
            ui.set_width(350.0);

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
                    ui.label("Display Icon Manager");
                });

                ui.separator();

                ui.add_space(ui.spacing().item_spacing.x);
            });

            const DISPLAY_SIZE: (usize, usize) = (64, 64);

            ui.vertical_centered(|ui| {
                ui.add(GLCD::new(
                    DISPLAY_SIZE,
                    DASHBOARD_DISAPLY_PIXEL_SIZE,
                    Color::BLACK,
                    Color::WHITE,
                    app.current_display_image.clone(),
                    (HOME_IMAGE_WIDTH, HOME_IMAGE_HEIGHT),
                    (
                        (DISPLAY_SIZE.0 - HOME_IMAGE_WIDTH) / 2,
                        (DISPLAY_SIZE.1 - HOME_IMAGE_HEIGHT) / 2,
                    ), // Center icon
                ))
                .on_hover_cursor(egui::CursorIcon::Default);

                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(egui::Rect::from_min_size(
                            ui.cursor().min
                                - (Vec2::new(
                                    -ui.available_width(),
                                    ui.available_height() / 2.0 + 24.0,
                                ) - Vec2::new(0.0, DISPLAY_SIZE.1 as f32 / 2.0))
                                    / 2.0,
                            (DISPLAY_SIZE.0 as f32 * DASHBOARD_DISAPLY_PIXEL_SIZE, 32.0).into(),
                        ))
                        .layout(egui::Layout::centered_and_justified(
                            egui::Direction::TopDown,
                        )),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Preview").color(egui::Color32::from_gray(127)),
                            );

                            ui.add_space(-ui.style().spacing.item_spacing.x / 2.0);

                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("ℹ")
                                        .color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                                )
                                .sense(egui::Sense::hover()),
                            )
                            .on_hover_cursor(egui::CursorIcon::Help)
                            .on_hover_text(
                                egui::RichText::new(
                                    "Make sure to check your device\n\
                                    as well before saving!",
                                )
                                .color(Color::LIGHT_BLUE)
                                .size(16.0),
                            );
                        });
                    },
                );

                const ROWS: usize = 3;

                egui::ScrollArea::vertical()
                    .max_height((ROWS + 1) as f32 * 20.0)
                    .show(ui, |ui| {
                        Some(
                            ui.add(
                                egui::TextEdit::multiline(&mut app.xbm_string)
                                    .hint_text("Paste your icon data here...")
                                    .desired_rows(ROWS)
                                    .desired_width(f32::INFINITY),
                            ),
                        );
                    });

                ui.add_space(ui.spacing().item_spacing.x);

                ui.horizontal_top(|ui| {
                    let spacing = ui.spacing().item_spacing.x;

                    let total_width = ui.available_width();
                    let button_width = (total_width - spacing) / 2.0;

                    ui.scope(|ui| {
                        if app.xbm_serialized.0.is_empty() || app.xbm_serialized.1.is_empty() {
                            ui.disable();
                        }

                        if ui
                            .add_sized([button_width, 0.0], egui::Button::new("Save to Device"))
                            .on_hover_text("Save this icon to device memory")
                            .on_disabled_hover_text(
                                "You need to upload an icon or reset\n\
                                the current one before Saving.",
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            if app.server_data.is_device_paired {
                                app.show_yes_no_modal(
                                    "memory-override-confirmation",
                                    "Override memory".to_string(),
                                    "This operation will override the current \
                                    memory on your device!\n\
                                    Are you sure you want to continue?"
                                        .to_string(),
                                    move |app| {
                                        app.close_modal();

                                        // `m` = `Memory`, `1` = true
                                        request_send_serial("m1").ok();

                                        if !app.xbm_serialized.0.is_empty()
                                            && !app.xbm_serialized.1.is_empty()
                                        {
                                            if let Some(config) = &mut app.config {
                                                update_config_and_server(config, |c| {
                                                    if let Some(layout) = &mut c.layout {
                                                        if let Some(component) = layout
                                                            .components
                                                            .get_mut(&app.xbm_serialized.1)
                                                        {
                                                            component.label =
                                                                app.xbm_serialized.0.clone();
                                                        }
                                                    }
                                                });

                                                app.xbm_serialized.0.clear();
                                                app.xbm_serialized.1.clear();
                                            }
                                        }

                                        app.show_message_modal(
                                            "upload-image-to-device-success",
                                            "Success".to_string(),
                                            "Device's memory was successfully updated.".to_string(),
                                        );
                                    },
                                    |app| {
                                        app.close_modal();
                                    },
                                    false,
                                );
                            } else {
                                app.show_not_paired_error();
                            }
                        }
                    });
                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("Upload and Test"))
                        .on_hover_text(
                            "This operation won't save to device memory, \n\
                            you need to do it manually.",
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        if app.server_data.is_device_paired {
                            let xbm_string = app.xbm_string.clone();

                            match extract_hex_bytes(&xbm_string, HOME_IMAGE_BYTES_SIZE) {
                                Ok(bytes) => {
                                    let data = hex_bytes_vec_to_string(&bytes);
                                    // `i` = *HOME* Image
                                    let data_str = format!("i{}", data);

                                    app.current_display_image = bytes;

                                    request_device_upload(data_str.clone(), false).ok();

                                    app.xbm_serialized = (data, component_global_id.to_string());

                                    app.show_message_modal(
                                        "xbm-upload-ok",
                                        "Ok".to_string(),
                                        "New X BitMap image \
                                        was uploaded to the device."
                                            .to_string(),
                                    );
                                }
                                Err(error) => {
                                    app.show_message_modal(
                                        "xbm-upload-error",
                                        "Error".to_string(),
                                        error,
                                    );
                                }
                            }
                        } else {
                            app.show_not_paired_error();
                        }
                    }
                });

                ui.horizontal_top(|ui| {
                    let spacing = ui.spacing().item_spacing.x;

                    let total_width = ui.available_width();
                    let button_width = (total_width - spacing) / 2.0;

                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("Reset to Default"))
                        .on_hover_text(
                            "Remove current saved icon from the device\n\
                            and show default.",
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        if app.server_data.is_device_paired {
                            app.xbm_serialized = (
                                HOME_IMAGE_DEFAULT_BYTES.to_string(),
                                component_global_id.to_string(),
                            );

                            app.show_yes_no_modal(
                                "xbm-remove-confirmation",
                                "Reset Display Icon".to_string(),
                                "You're about to remove and reset current \"Home Image\" \
                                on your device!\nAre you sure you want to continue?"
                                    .to_string(),
                                |app| {
                                    match hex_bytes_string_to_vec(HOME_IMAGE_DEFAULT_BYTES) {
                                        Ok(bytes) => {
                                            app.current_display_image = bytes;
                                        }
                                        Err(_) => (),
                                    }

                                    // `i` = *HOME* Image, and since there's no value
                                    // the device removes current image and set its default
                                    request_device_upload("i".to_string(), false).ok();
                                },
                                |app| {
                                    app.xbm_serialized.0.clear();
                                    app.xbm_serialized.1.clear();
                                },
                                true,
                            );
                        } else {
                            app.show_not_paired_error();
                        }
                    }
                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("Close"))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        app.close_modal();
                    }
                });
            });
        });
    }

    fn open_about_modal(&mut self) {
        use egui::*;

        self.show_custom_modal("about-modal", |ui, app| {
            ui.set_max_width(450.0);

            ui.vertical_centered(|ui| {
                // App Name Section
                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);
                ui.label(
                    RichText::new(format!("🎮 {}", APP_NAME))
                        .size(40.0)
                        .color(Color::YELLOW)
                        .strong(),
                );
                ui.add_space(ui.style().spacing.item_spacing.y * 0.75);
                ui.label(RichText::new("The Ultimate Macro Pad Companion").size(16.0));

                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);

                // Version Info
                ui.horizontal(|ui| {
                    ui.add_space(119.0); // Sorry!
                    ui.label(RichText::new("Version:   \t").size(15.0).strong());
                    ui.add_space(1.0);
                    ui.label(
                        RichText::new(APP_VERSION)
                            .color(
                                blend_colors(Color::YELLOW, Color::RED, 0.5).gamma_multiply(0.85),
                            )
                            .size(15.0),
                    );

                    if app.update_available {
                        ui.label(
                            RichText::new("• Update Available!")
                                .size(14.0)
                                .color(Color::GREEN),
                        );
                    }
                });
                ui.label(RichText::new(format!("Build Date:\t{}", app.build_date)).size(15.0));
                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);

                ui.separator();
                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);

                // Credits Section
                ui.heading(
                    RichText::new("Created with ❤ by")
                        .color(Color::RED.gamma_multiply(0.8))
                        .size(20.0),
                );
                ui.label(
                    RichText::new(env!("CARGO_PKG_AUTHORS"))
                        .color(Color::GREEN)
                        .size(16.0)
                        .strong(),
                );

                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);

                // Links Section
                ui.heading(RichText::new("Links").size(20.0));
                ui.add_space(-ui.style().spacing.item_spacing.y * 0.5);
                ui.horizontal(|ui| {
                    ui.add_space(45.0);
                    ui.hyperlink_to(
                        RichText::new("GitHub").color(Color::BLUE),
                        "https://github.com/IrregularCelery",
                    );
                    ui.label(" • ");
                    ui.hyperlink_to(
                        RichText::new("Documentation").color(Color::BLUE),
                        "https://github.com/IrregularCelery/padpad.software",
                    );
                    ui.label(" • ");
                    ui.hyperlink_to(
                        RichText::new("Report Bug").color(Color::BLUE),
                        "https://github.com/IrregularCelery/padpad.software/issues",
                    );
                });

                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);
                ui.separator();
                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);

                // License Section
                ui.heading(RichText::new("License").size(20.0));
                ui.add_space(ui.style().spacing.item_spacing.y * 1.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "{} is an open source software licensed under the",
                        APP_NAME
                    ));
                    ui.add_space(-4.0);
                    ui.hyperlink_to(
                        RichText::new("MIT License").color(Color::BLUE),
                        "https://github.com/IrregularCelery/padpad.software/blob/master/LICENSE",
                    );
                });

                ui.add_space(ui.style().spacing.item_spacing.y * 2.5);
            });
        });
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
        ui.set_max_width(120.0);

        let mut layout_size = (0.0, 0.0);

        if let Some(config) = &self.config {
            if let Some(layout) = &config.layout {
                layout_size = layout.size;
            }
        };

        let max_range = (layout_size.0.min(layout_size.1)) / 8.0; // To avoid snapping to outside
                                                                  // of layout border
        let min_range = if max_range < 2.0 { 1.0 } else { 2.0 };

        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Grid settings").size(17.0));
        });

        ui.add_space(-ui.style().spacing.item_spacing.y / 2.0);

        ui.separator();

        let spacing = ui.spacing().item_spacing.x;

        let total_width = ui.available_width();
        let input_width = (total_width - (spacing * 4.0)) / 2.0;

        ui.allocate_ui_with_layout(
            (input_width, 0.0).into(),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Enabled");
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(-2.0);

                        let response = ui.add(ToggleSwitch::new(self.layout_grid.0, (24.0, 16.0)));

                        if response.clicked() {
                            self.layout_grid.0 = !self.layout_grid.0;
                        }
                    });
                });

                ui.horizontal_centered(|ui| {
                    if !self.layout_grid.0 {
                        ui.disable();
                    }

                    ui.vertical(|ui| {
                        ui.label("Size");
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_sized(
                            (0.0, ui.available_height()),
                            DragValue::new(&mut self.layout_grid.1)
                                .speed(2.0)
                                .range(min_range..=max_range)
                                .clamp_existing_to_range(true),
                        );
                    });
                });
            },
        );
    }

    // Helper methods

    fn resize_layout_to_fit_components(&mut self) {
        let components_rect = self.get_layout_components_rect();

        if let Some(config) = &mut self.config {
            if let Some(layout) = &mut config.layout {
                // TODO: Maybe show a modal to get user's desired paddings
                let padding_x = 50.0;
                let padding_y = 50.0;

                layout.size.0 = components_rect.width() + (padding_x * 2.0);
                layout.size.1 = components_rect.height() + (padding_y * 2.0);

                self.center_components_to_layout();

                self.needs_resize = true;
            }
        }
    }

    fn center_components_to_layout(&mut self) {
        let components_rect = self.get_layout_components_rect();

        if let Some(config) = &mut self.config {
            if let Some(layout) = &mut config.layout {
                let layout_center = (layout.size.0 / 2.0, layout.size.1 / 2.0);
                let components_center = (
                    (components_rect.min.x + components_rect.max.x) / 2.0,
                    (components_rect.min.y + components_rect.max.y) / 2.0,
                );

                let offset_x = layout_center.0 - components_center.0;
                let offset_y = layout_center.1 - components_center.1;

                for component in layout.components.values_mut() {
                    component.position.0 += offset_x;
                    component.position.1 += offset_y;
                }

                update_config_and_server(config, |_| {});
            }
        }
    }

    /// Useful for when you want to edit the layout and you want to enable snap to grid option
    fn top_left_components_to_layout(&mut self) {
        let components_rect = self.get_layout_components_rect();

        if let Some(config) = &mut self.config {
            if let Some(layout) = &mut config.layout {
                let components_top_left = components_rect.min;

                let offset_x = components_top_left.x;
                let offset_y = components_top_left.y;

                for component in layout.components.values_mut() {
                    component.position.0 -= offset_x;
                    component.position.1 -= offset_y;
                }

                update_config_and_server(config, |_| {});
            }
        }
    }

    fn get_layout_components_rect(&self) -> egui::Rect {
        let mut rect = egui::Rect::NOTHING;

        if let Some(config) = &self.config {
            if let Some(layout) = &config.layout {
                let (min_x, min_y, max_x, max_y) = layout.components.iter().fold(
                    (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
                    |(min_x, min_y, max_x, max_y), (component_global_id, component)| {
                        let kind_string = component_global_id
                            .split(SERIAL_MESSAGE_SEP)
                            .next()
                            .unwrap_or("");
                        let kind = match kind_string {
                            "Button" => ComponentKind::Button,
                            "LED" => ComponentKind::LED,
                            "Potentiometer" => ComponentKind::Potentiometer,
                            "Joystick" => ComponentKind::Joystick,
                            "RotaryEncoder" => ComponentKind::RotaryEncoder,
                            "Display" => ComponentKind::Display,
                            _ => ComponentKind::None,
                        };

                        let (width, height) = match kind {
                            ComponentKind::None => (0.0, 0.0),
                            ComponentKind::Button => self.component_button_size,
                            ComponentKind::LED => self.component_led_size,
                            ComponentKind::Potentiometer => self.component_potentiometer_size,
                            ComponentKind::Joystick => self.component_joystick_size,
                            ComponentKind::RotaryEncoder => self.component_rotary_encoder_size,
                            ComponentKind::Display => (
                                self.component_display_size.0 * DASHBOARD_DISAPLY_PIXEL_SIZE,
                                self.component_display_size.1 * DASHBOARD_DISAPLY_PIXEL_SIZE,
                            ),
                        };

                        (
                            min_x.min(component.position.0),
                            min_y.min(component.position.1),
                            max_x.max(component.position.0 + (width * component.scale)),
                            max_y.max(component.position.1 + (height * component.scale)),
                        )
                    },
                );

                rect.min.x = min_x;
                rect.min.y = min_y;
                rect.max.x = max_x;
                rect.max.y = max_y;
            }
        }

        rect
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
            update_available: false,
            build_date: "February 08, 2025".to_string(),
            close_app: (false, false),
            unavoidable_error: (false, String::new()),
            modal: Arc::new(Mutex::new(ModalManager::new())),
            config: match Config::default().read() {
                Ok(mut config) => {
                    config.validate_config();

                    update_config_and_server(&mut config, |_| {});

                    Some(config)
                }
                Err(err) => {
                    log_error!("Error reading config file: {}", err);

                    None
                }
            },
            needs_resize: true,
            device_name: DEFAULT_DEVICE_NAME.to_string(),
            baud_rate_string: DEFAULT_BAUD_RATE.to_string(),
            baud_rate: None,
            port_name: (String::new(), false),
            server_data: ServerData::default(),
            server_needs_restart: false,
            components: HashMap::default(),
            component_properties: (None, None), // Current editing component properties
            button_memory: Default::default(),
            is_editing_layout: false,
            dragged_component_offset: (0.0, 0.0),
            layout_grid: (true, 10.0),
            components_backup: Default::default(),

            // Visuals
            global_shadow: 8.0,

            // TEMP VARIABLES (per modal)
            properties_selected_interaction: true, // `true` -> normal, `false` -> modkey (if available)
            properties_shortcut_key_filter: String::new(),
            properties_shortcut_kind: (true, true), // (normal, modkey) `true` -> keys, `false` -> text
            component_id: 0,
            component_id_exists: true,
            new_layout_name: "New Layout".to_string(),
            new_layout_size: (1000.0, 540.0),
            new_profile_name: String::new(),
            last_profile_name: String::new(), // Used for updating a profile
            profile_exists: false,
            xbm_string: String::new(),
            xbm_serialized: (
                String::new(), /* value */
                String::new(), /* component_global_id */
            ),
            current_display_image: vec![],
            paired_status_panel: (0.0, 0.0),
            components_panel: 0.0,
            toolbar_panel: 0.0,

            #[cfg(debug_assertions)]
            test_potentiometer_style: 0,
            #[cfg(debug_assertions)]
            test_potentiometer_value: 15.0,
            #[cfg(debug_assertions)]
            test_joystick_value: (0.0, 0.0),
            #[cfg(debug_assertions)]
            test_ascii_char_input: String::new(),

            // Constants
            component_button_size: (100.0, 100.0),
            component_led_size: (60.0, 60.0),
            component_potentiometer_size: (130.0, 130.0),
            component_joystick_size: (120.0, 120.0),
            component_rotary_encoder_size: (80.0, 80.0),
            component_display_size: (128.0, 64.0),
        }
    }
}

// Panels

fn draw_normal_interaction_panel(
    ui: &mut Ui,
    interactions: &mut Interaction,
    // (bool, &str) -> `true/false`, `hint_text`
    has_value: (bool, &str), // does the component have a value? e.g. potentiometer has 0-99
    hid_warning_condition: bool,
    properties_shortcut_kind: &mut (bool, bool), // (normal, modkey) `true` -> keys, `false` -> text
    properties_shortcut_key_filter: &mut String,
    should_open_button_memory_manager: &mut bool,
    should_update: &mut bool,
) {
    if hid_warning_condition {
        ui.add_space(ui.style().spacing.item_spacing.y);

        ui.vertical_centered_justified(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Warning: This button already has an HID keyboard shortcut \
                            stored in the device. It's recommended to clear it before \
                            assigning a new software-based interaction.",
                        )
                        .color(Color::YELLOW.gamma_multiply(0.75))
                        .size(13.5),
                    );

                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new("\nClick here to open manager")
                                    .color(Color::BLUE)
                                    .underline()
                                    .size(14.0),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        *should_open_button_memory_manager = true;
                    }
                });
            });
        });
    }

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.add_space(ui.style().spacing.item_spacing.y * 2.0 + 2.0);
            ui.label("Kind");
        });

        ui.add_space(ui.style().spacing.item_spacing.y * 2.0);

        const INTERACTION_NONE: InteractionKind = InteractionKind::None();
        const INTERACTION_COMMAND: InteractionKind =
            InteractionKind::Command(String::new(), String::new());
        const INTERACTION_APPLICATION: InteractionKind =
            InteractionKind::Application(String::new());
        const INTERACTION_WEBSITE: InteractionKind = InteractionKind::Website(String::new());
        const INTERACTION_SHORTCUT: InteractionKind =
            InteractionKind::Shortcut(vec![], String::new());
        const INTERACTION_FILE: InteractionKind = InteractionKind::File(String::new());

        egui::ComboBox::new("properties-interactions-normal", "")
            .selected_text(format!("{}", interactions.normal))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(interactions.normal.equals_kind(&INTERACTION_NONE), "None")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.normal = INTERACTION_NONE;
                }

                if ui
                    .selectable_label(
                        interactions.normal.equals_kind(&INTERACTION_COMMAND),
                        "Command",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.normal = INTERACTION_COMMAND;
                }

                if ui
                    .selectable_label(
                        interactions.normal.equals_kind(&INTERACTION_APPLICATION),
                        "Application",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.normal = INTERACTION_APPLICATION;
                }

                if ui
                    .selectable_label(
                        interactions.normal.equals_kind(&INTERACTION_WEBSITE),
                        "Website",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.normal = INTERACTION_WEBSITE;
                }

                if ui
                    .selectable_label(
                        interactions.normal.equals_kind(&INTERACTION_SHORTCUT),
                        "Shortcut",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.normal = INTERACTION_SHORTCUT;
                }

                if ui
                    .selectable_label(interactions.normal.equals_kind(&INTERACTION_FILE), "File")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.normal = INTERACTION_FILE;
                }
            })
            .response
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if interactions.normal.equals_kind(&INTERACTION_SHORTCUT) {
            ui.vertical(|ui| {
                ui.add_space(ui.style().spacing.item_spacing.y * 2.0 + 3.0);

                ui.horizontal(|ui| {
                    ui.add_space(ui.style().spacing.item_spacing.y * 1.5);

                    let mode_label_response = ui
                        .add(egui::Label::new("Text Mode").sense(egui::Sense::click()))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(
                            "When this shortcut is triggered, it simulates \n\
                            typing a text.",
                        );

                    ui.add_space(ui.style().spacing.item_spacing.y * 2.0 + 2.0);

                    let mode_switch_response = ui
                        .add(ToggleSwitch::new(!properties_shortcut_kind.0, (50.0, 26.0)))
                        .on_hover_text(
                            "When this shortcut is triggered, it simulates \n\
                            typing a text.",
                        );

                    if mode_label_response.clicked() || mode_switch_response.clicked() {
                        properties_shortcut_kind.0 = !properties_shortcut_kind.0;
                    }
                });
            });
        }
    });

    let default_hint = if has_value.0 {
        "You can pass this component's value\n\
        to your interaction by adding {value}\n\n"
    } else {
        ""
    }
    .to_string();

    match &mut interactions.normal {
        InteractionKind::None() => {
            *should_update = true;
        }
        InteractionKind::Command(command, _shell) => {
            ui.horizontal(|ui| {
                ui.label("Command");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\techo \"{value}\""
                    } else {
                        "Example:\n\techo \"Hello world!\""
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(command)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
        InteractionKind::Application(path) => {
            ui.horizontal(|ui| {
                ui.label("Application Full Path");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\t~/.local/bin/brightness {value}\nor\n\
                        \tC:\\Program Files\\Volume Changer\\Volume.exe {value}"
                    } else {
                        "Example:\n\t~/.local/bin/terminal \nor\n\
                        \tC:\\Program Files\\My Application\\App.exe"
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(path)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
        InteractionKind::Website(url) => {
            ui.horizontal(|ui| {
                ui.label("Website URL");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\thttps://www.google.com/search?q=number {value}"
                    } else {
                        "Example:\n\thttps://www.github.com/IrregularCelery"
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(url)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
        InteractionKind::Shortcut(keys, text) => {
            // Keys
            if properties_shortcut_kind.0 {
                // `text` must be empty in `keys` mode
                *text = String::new();

                ui.vertical_centered_justified(|ui| {
                    if keys.is_empty() {
                        ui.group(|ui| {
                            ui.label("No keys were added yet!");
                        });
                    } else {
                        ui.add(
                            ItemList::new(
                                keys,
                                26.0,
                                egui::Color32::from_gray(50),
                                Color::WHITE,
                                Color::ACCENT.gamma_multiply(0.15),
                                egui::Color32::from_gray(12),
                            )
                            .spacing(2.0)
                            .on_item_removed(|_item| {
                                *should_update = true;
                            }),
                        );
                    }
                });

                let keys_response = egui::ComboBox::new("properties-interactions-normal-keys", "")
                    .selected_text("Add Keys")
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show_ui(ui, |ui| {
                        let filter_response = ui.add_sized(
                            (160.0, 0.0),
                            egui::TextEdit::singleline(properties_shortcut_key_filter)
                                .margin(Vec2::new(8.0, 8.0))
                                .hint_text("Search"),
                        );

                        filter_response.request_focus();

                        let filtered_options: Vec<_> = KEYS
                            .iter()
                            .filter(|option| {
                                format!("{}", option)
                                    .to_lowercase()
                                    .contains(&properties_shortcut_key_filter.to_lowercase())
                            })
                            .collect();

                        for key in filtered_options {
                            if ui
                                .selectable_label(false, format!("{}", key))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                ui.memory_mut(|mem| mem.toggle_popup(ui.id()));

                                keys.push(key.clone());

                                properties_shortcut_key_filter.clear();

                                *should_update = true;
                            }
                        }

                        // Dummy items to fill the space even if there's no item
                        ui.add_space((32.0 * 5.0) - 4.0);
                    });

                if keys_response
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    properties_shortcut_key_filter.clear();
                }

            // Text
            } else {
                ui.label("Text");

                const ROWS: usize = 2;

                let mut response = None;

                egui::ScrollArea::vertical()
                    .max_height((ROWS + 1) as f32 * 20.0)
                    .show(ui, |ui| {
                        response = Some(
                            ui.add(
                                egui::TextEdit::multiline(text)
                                    .desired_rows(ROWS)
                                    .desired_width(f32::INFINITY),
                            ),
                        );
                    });

                if let Some(r) = response {
                    if r.changed() {
                        *should_update = true;
                    }
                }
            }
        }
        InteractionKind::File(path) => {
            ui.horizontal(|ui| {
                ui.label("File Full Path");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\t~/media/videos/never-gonna-give-you-up.mkv\nor\n\
                        \tC:\\media\\pictures\\{value}.jpg"
                    } else {
                        "Example:\n\t~/media/videos/never-gonna-give-you-up.mkv\nor\n\
                        \tC:\\media\\pictures\\bird.jpg"
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(path)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
    }
}

fn draw_modkey_interaction_panel(
    ui: &mut Ui,
    interactions: &mut Interaction,
    // (bool, &str) -> `true/false`, `hint_text`
    has_value: (bool, &str), // does the component have a value? e.g. potentiometer has 0-99
    hid_warning_condition: bool,
    properties_shortcut_kind: &mut (bool, bool), // (normal, modkey) `true` -> keys, `false` -> text
    properties_shortcut_key_filter: &mut String,
    should_open_button_memory_manager: &mut bool,
    should_update: &mut bool,
) {
    if hid_warning_condition {
        ui.add_space(ui.style().spacing.item_spacing.y);

        ui.vertical_centered_justified(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Warning: This button already has an HID keyboard shortcut \
                            stored in the device. It's recommended to clear it before \
                            assigning a new software-based interaction.",
                        )
                        .color(Color::YELLOW.gamma_multiply(0.75))
                        .size(13.5),
                    );

                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new("\nClick here to open manager")
                                    .color(Color::BLUE)
                                    .underline()
                                    .size(14.0),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        *should_open_button_memory_manager = true;
                    }
                });
            });
        });
    }

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.add_space(ui.style().spacing.item_spacing.y * 2.0 + 2.0);
            ui.label("Kind");
        });

        ui.add_space(ui.style().spacing.item_spacing.y * 2.0);

        const INTERACTION_NONE: InteractionKind = InteractionKind::None();
        const INTERACTION_COMMAND: InteractionKind =
            InteractionKind::Command(String::new(), String::new());
        const INTERACTION_APPLICATION: InteractionKind =
            InteractionKind::Application(String::new());
        const INTERACTION_WEBSITE: InteractionKind = InteractionKind::Website(String::new());
        const INTERACTION_SHORTCUT: InteractionKind =
            InteractionKind::Shortcut(vec![], String::new());
        const INTERACTION_FILE: InteractionKind = InteractionKind::File(String::new());

        egui::ComboBox::new("properties-interactions-modkey", "")
            .selected_text(format!("{}", interactions.modkey))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(interactions.modkey.equals_kind(&INTERACTION_NONE), "None")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.modkey = INTERACTION_NONE;
                }

                if ui
                    .selectable_label(
                        interactions.modkey.equals_kind(&INTERACTION_COMMAND),
                        "Command",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.modkey = INTERACTION_COMMAND;
                }

                if ui
                    .selectable_label(
                        interactions.modkey.equals_kind(&INTERACTION_APPLICATION),
                        "Application",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.modkey = INTERACTION_APPLICATION;
                }

                if ui
                    .selectable_label(
                        interactions.modkey.equals_kind(&INTERACTION_WEBSITE),
                        "Website",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.modkey = INTERACTION_WEBSITE;
                }

                if ui
                    .selectable_label(
                        interactions.modkey.equals_kind(&INTERACTION_SHORTCUT),
                        "Shortcut",
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.modkey = INTERACTION_SHORTCUT;
                }

                if ui
                    .selectable_label(interactions.modkey.equals_kind(&INTERACTION_FILE), "File")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    interactions.modkey = INTERACTION_FILE;
                }
            })
            .response
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if interactions.modkey.equals_kind(&INTERACTION_SHORTCUT) {
            ui.vertical(|ui| {
                ui.add_space(ui.style().spacing.item_spacing.y * 2.0 + 3.0);

                ui.horizontal(|ui| {
                    ui.add_space(ui.style().spacing.item_spacing.y * 1.5);

                    let mode_label_response = ui
                        .add(egui::Label::new("Text Mode").sense(egui::Sense::click()))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(
                            "When this shortcut is triggered, it simulates \n\
                            typing a text.",
                        );

                    ui.add_space(ui.style().spacing.item_spacing.y * 2.0 + 2.0);

                    let mode_switch_response = ui
                        .add(ToggleSwitch::new(!properties_shortcut_kind.1, (50.0, 26.0)))
                        .on_hover_text(
                            "When this shortcut is triggered, it simulates \n\
                            typing a text.",
                        );

                    if mode_label_response.clicked() || mode_switch_response.clicked() {
                        properties_shortcut_kind.1 = !properties_shortcut_kind.1;
                    }
                });
            });
        }
    });

    let default_hint = if has_value.0 {
        "You can pass this component's value\n\
        to your interaction by adding {value}\n\n"
    } else {
        ""
    }
    .to_string();

    match &mut interactions.modkey {
        InteractionKind::None() => {
            *should_update = true;
        }
        InteractionKind::Command(command, _shell) => {
            ui.horizontal(|ui| {
                ui.label("Command");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\techo \"{value}\""
                    } else {
                        "Example:\n\techo \"Hello world!\""
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(command)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
        InteractionKind::Application(path) => {
            ui.horizontal(|ui| {
                ui.label("Application Full Path");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\t~/.local/bin/brightness {value}\nor\n\
                        \tC:\\Program Files\\Volume Changer\\Volume.exe {value}"
                    } else {
                        "Example:\n\t~/.local/bin/terminal \nor\n\
                        \tC:\\Program Files\\My Application\\App.exe"
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(path)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
        InteractionKind::Website(url) => {
            ui.horizontal(|ui| {
                ui.label("Website URL");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\thttps://www.google.com/search?q=number {value}"
                    } else {
                        "Example:\n\thttps://www.github.com/IrregularCelery"
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(url)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
        InteractionKind::Shortcut(keys, text) => {
            // Keys
            if properties_shortcut_kind.1 {
                // `text` must be empty in `keys` mode
                *text = String::new();

                ui.vertical_centered_justified(|ui| {
                    if keys.is_empty() {
                        ui.group(|ui| {
                            ui.label("No keys were added yet!");
                        });
                    } else {
                        ui.add(
                            ItemList::new(
                                keys,
                                26.0,
                                egui::Color32::from_gray(50),
                                Color::WHITE,
                                Color::ACCENT.gamma_multiply(0.15),
                                egui::Color32::from_gray(12),
                            )
                            .spacing(2.0)
                            .on_item_removed(|_item| {
                                *should_update = true;
                            }),
                        );
                    }
                });

                let keys_response = egui::ComboBox::new("properties-interactions-modkey-keys", "")
                    .selected_text("Add Keys")
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show_ui(ui, |ui| {
                        let filter_response = ui.add_sized(
                            (160.0, 0.0),
                            egui::TextEdit::singleline(properties_shortcut_key_filter)
                                .margin(Vec2::new(8.0, 8.0))
                                .hint_text("Search"),
                        );

                        filter_response.request_focus();

                        let filtered_options: Vec<_> = KEYS
                            .iter()
                            .filter(|option| {
                                format!("{}", option)
                                    .to_lowercase()
                                    .contains(&properties_shortcut_key_filter.to_lowercase())
                            })
                            .collect();

                        for key in filtered_options {
                            if ui
                                .selectable_label(false, format!("{}", key))
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                ui.memory_mut(|mem| mem.toggle_popup(ui.id()));

                                keys.push(key.clone());

                                properties_shortcut_key_filter.clear();

                                *should_update = true;
                            }
                        }

                        // Dummy items to fill the space even if there's no item
                        ui.add_space((32.0 * 5.0) - 4.0);
                    });

                if keys_response
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    properties_shortcut_key_filter.clear();
                }

            // Text
            } else {
                ui.label("Text");

                const ROWS: usize = 2;

                let mut response = None;

                egui::ScrollArea::vertical()
                    .max_height((ROWS + 1) as f32 * 20.0)
                    .show(ui, |ui| {
                        response = Some(
                            ui.add(
                                egui::TextEdit::multiline(text)
                                    .desired_rows(ROWS)
                                    .desired_width(f32::INFINITY),
                            ),
                        );
                    });

                if let Some(r) = response {
                    if r.changed() {
                        *should_update = true;
                    }
                }
            }
        }
        InteractionKind::File(path) => {
            ui.horizontal(|ui| {
                ui.label("File Full Path");

                let hint = default_hint
                    + if has_value.0 {
                        "Example:\n\t~/media/videos/never-gonna-give-you-up.mkv\nor\n\
                        \tC:\\media\\pictures\\{value}.jpg"
                    } else {
                        "Example:\n\t~/media/videos/never-gonna-give-you-up.mkv\nor\n\
                        \tC:\\media\\pictures\\bird.jpg"
                    };
                let hint = hint
                    + if has_value.0 {
                        format!("\n\n({})", has_value.1)
                    } else {
                        String::new()
                    }
                    .as_str();

                ui.add(
                    egui::Label::new(
                        egui::RichText::new("ℹ").color(Color::LIGHT_BLUE.gamma_multiply(0.75)),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_cursor(egui::CursorIcon::Help)
                .on_hover_text(
                    egui::RichText::new(hint)
                        .color(Color::LIGHT_BLUE)
                        .size(16.0),
                );
            });

            const ROWS: usize = 2;

            let mut response = None;

            egui::ScrollArea::vertical()
                .max_height((ROWS + 1) as f32 * 20.0)
                .show(ui, |ui| {
                    response = Some(
                        ui.add(
                            egui::TextEdit::multiline(path)
                                .desired_rows(ROWS)
                                .desired_width(f32::INFINITY),
                        ),
                    );
                });

            if let Some(r) = response {
                if r.changed() {
                    *should_update = true;
                }
            }
        }
    }
}
