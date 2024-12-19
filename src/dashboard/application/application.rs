use eframe::egui::{self, Button, Context, Pos2, Response, Ui, Vec2};

use padpad_software::{
    config::{ComponentKind, Config},
    log_error,
};

use super::get_current_style;

pub struct Application {
    config: Option<Config>,
}

impl eframe::App for Application {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        use egui::*;
        let style = get_current_style();
        ctx.set_style(style);
        //ctx.set_pixels_per_point(1.0);

        // Custom window
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

            ui.allocate_ui_at_rect(title_bar_rect, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                });
            });

            // Custom window content
            ui.heading("Hello World!");
            ui.label("PadPad is under construction!");

            let button = ui.button("hi");

            if button.hovered() {
                ui.label("YES");
            }

            if button.clicked() {
                println!("Button was clicked");
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
                .frame(egui::Frame {
                    fill: egui::Color32::RED,
                    rounding: 4.0.into(),
                    ..egui::Frame::default()
                })
                .show(ctx, |ui| {
                    //println!("test");

                    self.draw_layout(ctx, ui);
                });
        });
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
        Self {
            config: match Config::default().read() {
                Ok(config) => Some(config),
                Err(err) => {
                    log_error!("Error reading config file: {}", err);

                    None
                }
            },
        }
    }
}
