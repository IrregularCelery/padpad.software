use eframe::egui::{
    Align2, Color32, Context, CursorIcon, FontId, Id, Pos2, Rect, Response, Rounding, Sense, Shape,
    Stroke, Ui, Vec2, Widget,
};

pub use super::theme::Color;

#[derive(Default)]
pub struct ModalManager {
    pub stack: Vec<Modal>,
}

#[derive(Clone)]
pub struct Modal {
    pub id: &'static str,
    pub content: std::sync::Arc<dyn Fn(&mut Ui, &mut super::Application) + Send + Sync + 'static>,
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

pub struct LED {
    value: (u8 /* r */, u8 /* g */, u8 /* b */),
    size: (f32, f32),
}

impl LED {
    pub fn new(value: (u8, u8, u8), size: (f32, f32)) -> Self {
        Self { value, size }
    }
}

impl Widget for LED {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::from(self.size);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let painter = ui.painter();

        let center = rect.center();

        painter.rect_filled(
            Rect::from_center_size(center, desired_size * 0.55),
            ui.style().visuals.menu_rounding,
            Color32::from_rgb(self.value.0, self.value.1, self.value.2),
        );

        response
    }
}

pub struct Potentiometer {
    id: String,
    /// Value must be between 0-100
    value: f32,
    size: (f32, f32),
    style: u8,
}

impl Potentiometer {
    pub fn new(id: String, value: f32, size: (f32, f32)) -> Self {
        Self {
            id,
            value: value.clamp(0.0, 100.0),
            size,
            style: 0,
        }
    }

    pub fn style(mut self, style: u8) -> Self {
        self.style = style;

        self
    }
}

// TODO: Update colors
impl Widget for Potentiometer {
    fn ui(self, ui: &mut Ui) -> Response {
        use std::f32::consts::PI;

        let desired_size = self.size.into();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let value = ui
            .ctx()
            .animate_value_with_time(self.id.into(), self.value, 0.25);

        let painter = ui.painter();

        // Draw container
        painter.rect_filled(rect, ui.style().visuals.menu_rounding, Color::CONTAINER);

        let center = rect.center();
        let radius = (desired_size.x.min(desired_size.y) / 2.0) * 0.8;

        let start_angle = 0.75 * PI;
        let end_angle = 2.25 * PI;
        let range = end_angle - start_angle;

        let stroke_width = 16.0;
        let circle_radius = stroke_width / 2.0;
        let offset_y = (stroke_width / 2.0) - 2.0;

        let bg_points = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let angle = start_angle + t * range;
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin() + offset_y;

                Pos2::new(x, y)
            })
            .collect::<Vec<_>>();

        let bg_color = Color::SURFACE2;

        // Draw background line
        painter.add(Shape::line(
            bg_points.clone(),
            Stroke::new(stroke_width, bg_color),
        ));

        // Add line end circles for background
        if let (Some(start), Some(end)) = (bg_points.first(), bg_points.last()) {
            painter.circle_filled(*start, circle_radius, bg_color);
            painter.circle_filled(*end, circle_radius, bg_color);
        }

        // Draw the filled portion based on value
        let value_angle = start_angle + (value / 100.0) * range;
        let filled_points = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let angle = start_angle + t * (value_angle - start_angle);
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin() + offset_y;
                Pos2::new(x, y)
            })
            .collect::<Vec<_>>();

        let filled_color = Color::BLUE;

        // Draw filled line
        painter.add(Shape::line(
            filled_points.clone(),
            Stroke::new(stroke_width, filled_color),
        ));

        // Add circles for filled portion
        if let (Some(start), Some(end)) = (filled_points.first(), filled_points.last()) {
            painter.circle_filled(*start, circle_radius, filled_color);
            painter.circle_filled(*end, circle_radius, filled_color);
        }

        // Draw indicator
        let indicator_position_offset = 0.75;
        let indicator_length = radius * (indicator_position_offset / 2.0);

        // Value indicator
        let value_start = Pos2::new(
            center.x + radius * value_angle.cos(),
            center.y + radius * value_angle.sin() + offset_y,
        );
        let value_end = Pos2::new(
            center.x + (radius - indicator_length) * value_angle.cos(),
            center.y + (radius - indicator_length) * value_angle.sin() + offset_y,
        );
        painter.line_segment([value_start, value_end], Stroke::new(2.0, Color::WHITE));

        // Draw value text
        painter.text(
            center,
            Align2::CENTER_CENTER,
            format!("{:.0}", value),
            FontId::proportional(24.0),
            Color::WHITE,
        );

        response
    }
}

pub struct Joystick {
    value: (f32 /* x */, f32 /* y */),
    size: (f32, f32),
}

impl Joystick {
    pub fn new(value: (f32, f32), size: (f32, f32)) -> Self {
        let x = value.0.clamp(-1.0, 1.0);
        let y = value.1.clamp(-1.0, 1.0);

        Self {
            value: (x, y),
            size,
        }
    }
}

impl Widget for Joystick {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::from(self.size);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let center = rect.center();
        let radius = rect.width().min(rect.height()) * 0.5;

        let painter = ui.painter();

        // Base shadow
        painter.circle_filled(center, radius, Color32::from_gray(40));

        // Base main part
        painter.circle_filled(center, radius * 0.95, Color32::from_gray(60));

        // Calculate handle position
        let handle_pos = Pos2::new(
            center.x + self.value.0 * radius * 0.3, // Increased movement range
            center.y - self.value.1 * radius * 0.3,
        );

        // Handle shadow
        painter.circle_filled(handle_pos, radius * 0.7, Color32::from_gray(50));

        // Handle main part
        painter.circle_filled(handle_pos, radius * 0.65, Color32::from_gray(70));

        response
    }
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
    /// Size multiplier
    scale: f32,
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
            scale: 1.0,
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;

        self
    }
}

impl Widget for GLCD {
    fn ui(self, ui: &mut Ui) -> Response {
        let glcd_width = self.glcd_size.0 as f32 * self.pixel_size * self.scale;
        let glcd_height = self.glcd_size.1 as f32 * self.pixel_size * self.scale;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(glcd_width, glcd_height), Sense::hover());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let painter = ui.painter();

        let pixel_size = self.pixel_size * self.scale;

        let start_x = rect.min.x + self.xbm_position.0 as f32 * pixel_size;
        let start_y = rect.min.y + self.xbm_position.1 as f32 * pixel_size;

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
                        let x = start_x + col as f32 * pixel_size;
                        let y = start_y + row as f32 * pixel_size;

                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(x.floor(), y.floor()),
                                Vec2::new(pixel_size.ceil(), pixel_size.ceil()),
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

    let id = Id::new(id).with("animate-eased");

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
