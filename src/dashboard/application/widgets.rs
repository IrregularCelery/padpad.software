use std::sync::Arc;

use eframe::{
    egui::{
        Color32, Context, CursorIcon, Id, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui,
        Vec2, Widget,
    },
    epaint,
};

pub use super::theme::Color;

#[derive(Default)]
pub struct ModalManager {
    pub stack: Vec<Modal>,
}

#[derive(Clone)]
pub struct Modal {
    pub id: &'static str,
    pub content: Arc<dyn Fn(&mut Ui, &mut super::Application) + Send + Sync + 'static>,
}

impl ModalManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains_modal(&self, id: &str) -> bool {
        self.stack.iter().any(|modal| modal.id == id)
    }

    pub fn close_last_modals(&mut self, number: usize) {
        let new_len = self.stack.len().saturating_sub(number);

        self.stack.truncate(new_len);
    }
}

// TODO: Needs to be changed quite a lot!
pub fn circular_progress(ui: &mut Ui, progress: f32, radius: f32) -> Response {
    use std::f32::consts::PI;

    let stroke_width = 4.0;
    let total_size = (radius + stroke_width) * 2.0;

    let (response, painter) =
        ui.allocate_painter(Vec2::new(total_size, total_size), Sense::hover());

    // Adjust center for stroke width
    let center = response.rect.center();
    let progress = progress.clamp(0.0, 1.0);

    // Background circle
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(stroke_width, Color32::from_gray(60)),
    );

    // Progress arc
    if progress > 0.0 {
        let start_angle = -PI / 2.0;
        let end_angle = start_angle + (2.0 * PI * progress);

        let mut points = Vec::new();
        let steps = 50;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            points.push(Pos2::new(x, y));
        }

        // Forground arc
        painter.add(epaint::PathShape::line(
            points,
            Stroke::new(stroke_width, Color32::BLUE),
        ));
    }

    response
}

pub struct GLCD {
    /// GLCD's resolution
    glcd_size: (usize, usize),
    /// Size of each drawn square for a virtual pixel
    pixel_size: f32,
    background_color: Color32,
    pixel_color: Color32,
    xbm_data: Vec<u8>,
    xbm_size: (usize, usize),
    /// Position of the xbm image inside the virtual GLCD
    xbm_position: (usize, usize),
}

impl GLCD {
    pub fn new(
        glcd_size: (usize, usize),
        pixel_size: f32,
        background_color: Color32,
        pixel_color: Color32,
        xbm_data: Vec<u8>,
        xbm_size: (usize, usize),
        xbm_position: (usize, usize),
    ) -> Self {
        Self {
            glcd_size,
            pixel_size,
            background_color,
            pixel_color,
            xbm_data,
            xbm_size,
            xbm_position,
        }
    }
}

impl Widget for GLCD {
    fn ui(self, ui: &mut Ui) -> Response {
        let glcd_width = self.glcd_size.0 as f32 * self.pixel_size;
        let glcd_height = self.glcd_size.1 as f32 * self.pixel_size;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(glcd_width, glcd_height), Sense::hover());

        let painter = ui.painter();
        let start_x = rect.min.x + self.xbm_position.0 as f32 * self.pixel_size;
        let start_y = rect.min.y + self.xbm_position.1 as f32 * self.pixel_size;

        let bytes_per_row = (self.xbm_size.0 + 7) / 8;

        painter.rect_filled(
            rect,
            ui.style().visuals.window_rounding,
            self.background_color,
        );

        for row in 0..self.xbm_size.1 {
            for col in 0..self.xbm_size.0 {
                let byte_index = row * bytes_per_row + (col / 8);
                let bit_index = col % 8;

                if let Some(&byte) = self.xbm_data.get(byte_index) {
                    if (byte >> bit_index) & 1 == 1 {
                        let x = start_x + col as f32 * self.pixel_size;
                        let y = start_y + row as f32 * self.pixel_size;

                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(x.floor(), y.floor()),
                                Vec2::new(self.pixel_size.ceil(), self.pixel_size.ceil()),
                            ),
                            0.0,
                            self.pixel_color,
                        );
                    }
                }
            }
        }

        response
    }
}

pub fn status_indicator(id: &'static str, ui: &mut Ui, color: Color32, size: f32) -> Response {
    let desired_size = Vec2::splat(size);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    let center = rect.center();
    let radius = rect.width() / 3.75;

    let background_rect = Rect::from_center_size(center, rect.size());

    let circle_shape = Shape::circle_filled(center, radius, color);

    let glow_effect_spread;
    let hover_effect_gamma;

    if response.contains_pointer() || response.has_focus() {
        glow_effect_spread = size / 24.0;
        hover_effect_gamma = 0.35;
    } else {
        glow_effect_spread = size / 32.0;
        hover_effect_gamma = 0.75;
    }

    let glow_effect_spread_value = animate_value(ui.ctx(), id, glow_effect_spread, 0.25);
    let hover_effect_value =
        animate_value(ui.ctx(), format!("{}-hover", id), hover_effect_gamma, 0.25);

    // Glow effect
    for i in 1..4 {
        ui.painter().circle(
            center,
            radius + i as f32 * glow_effect_spread_value,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 100 / (i * 2)),
            Stroke::NONE,
        );
    }

    // Background rectangle
    ui.painter().rect(
        background_rect,
        Rounding::same(size / 4.0),
        Color::OVERLAY0.gamma_multiply(hover_effect_value),
        Stroke::new(1.0, Color::OVERLAY0),
    );

    // Circle indicator
    ui.painter().add(circle_shape);

    response
}

// Heavily inspired by https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_animation
// THANK YOU :D <3
pub fn animate_value(
    ctx: &Context,
    id: impl std::hash::Hash + Sized,
    value: f32,
    duration: f32,
) -> f32 {
    // Cubic-In-Out
    fn ease_fn(t: f32) -> f32 {
        if t < 0.5 {
            // f(t) = 4 * (t ^ 3)

            4.0 * t * t * t
        } else {
            // f(t) = 0.5 * ((2t − 2)) ^ 3 + 1

            let f = 2.0 * t - 2.0;

            0.5 * f * f * f + 1.0
        }
    }

    let id = Id::new(id).with("animate_eased");

    let (source, target) = ctx.memory_mut(|mem| {
        let state = mem.data.get_temp_mut_or_insert_with(id, || (value, value));

        if state.1 != value {
            state.0 = state.1;
            state.1 = value;
        }
        (state.0, state.1)
    });

    let progress = ctx.animate_value_with_time(id, value, duration);

    if target == source {
        return target;
    }

    let progress = (progress - source) / (target - source);

    ease_fn(progress) * (target - source) + source
}
