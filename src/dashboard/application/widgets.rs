use std::f32::consts::{PI, TAU};

use eframe::{
    egui::{
        Color32, Context, CursorIcon, FontId, Frame, Id, Margin, Pos2, Rect, Response, Rounding,
        Sense, Shape, Stroke, Ui, Vec2, Widget,
    },
    epaint::PathShape,
};

pub use super::theme::Color;

#[derive(Default)]
pub struct ModalManager {
    pub stack: Vec<Modal>,
}

#[derive(Clone)]
pub struct Modal {
    pub id: &'static str,
    pub can_close: bool,
    pub width: f32,
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

pub struct Button {
    size: (f32, f32),
    pressed: bool,
}

impl Button {
    pub fn new(size: (f32, f32)) -> Self {
        Self {
            size,
            pressed: false,
        }
    }

    pub fn set_pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;

        self
    }

    pub const STYLES_COUNT: u8 = 0;
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::from(self.size);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);

        let rect = rect - Margin::same(ui.style().spacing.item_spacing.x / 2.0);

        let center = rect.center();
        let rounding = ui.style().visuals.menu_rounding;
        let padding = ui.style().spacing.item_spacing.x / 2.5;

        let color = Color::ACCENT;

        // Draw outer glow effect
        for i in (0..4).rev() {
            let alpha = 25 - (i * 5);

            ui.painter().rect_stroke(
                Rect::from_center_size(
                    center,
                    (
                        rect.width() + padding - (i as f32) * 1.25,
                        rect.height() + padding - (i as f32) * 1.25,
                    )
                        .into(),
                ),
                rounding - (i as f32).into(),
                Stroke::new(
                    1.0,
                    Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha),
                ),
            );
        }

        // Draw subtle inner rect
        ui.painter().rect_stroke(
            Rect::from_center_size(
                center,
                (rect.width() - padding * 3.0, rect.height() - padding * 3.0).into(),
            ),
            rounding / 4.0,
            Stroke::new(1.0, Color32::from_gray(40)),
        );

        let inner_color = if self.pressed {
            Color::ACCENT.gamma_multiply(0.5)
        } else if response.hovered() {
            Color32::from_gray(25)
        } else {
            Color32::from_gray(20)
        };

        // Draw inner rect
        ui.painter().rect_filled(
            Rect::from_center_size(
                center,
                (rect.width() - padding * 3.0, rect.height() - padding * 3.0).into(),
            ),
            rounding / 4.0,
            inner_color,
        );

        response
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

    pub const STYLES_COUNT: u8 = 0;
}

impl Widget for LED {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::from(self.size);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);

        let painter = ui.painter();

        let center = rect.center();

        painter.rect_filled(
            Rect::from_center_size(center, desired_size),
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

    fn draw_style_default(&self, ui: &mut Ui, rect: Rect, hovered: bool) {
        let rect = rect - Margin::same(ui.style().spacing.item_spacing.x / 2.0);

        let center = rect.center();
        let radius = (rect.width().min(rect.height()) * 0.5) - 2.0;

        // Define the angle range (270 degrees, leaving 90 degrees gap)
        let start_angle = -225.0 * PI / 180.0;
        let end_angle = 45.0 * PI / 180.0;
        let rotation = start_angle + (end_angle - start_angle) * (self.value / 100.0);

        let color = Color::ACCENT;

        // Draw outer glow effect
        for i in (0..4).rev() {
            let alpha = 25 - (i * 5);

            ui.painter().circle_stroke(
                center,
                radius - (i as f32) / 1.5,
                Stroke::new(
                    1.0,
                    Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha),
                ),
            );
        }

        // Draw background track
        let bg_points = Self::create_arc_points(center, radius - 5.0, start_angle, end_angle, 32);

        ui.painter().add(PathShape::line(
            bg_points,
            Stroke::new(1.5, Color32::from_gray(40)),
        ));

        // Draw progress track
        let filled_points =
            Self::create_arc_points(center, radius - 5.0, start_angle, rotation, 32);

        ui.painter()
            .add(PathShape::line(filled_points, Stroke::new(3.0, color)));

        // Draw subtle inner ring
        ui.painter().circle_stroke(
            center,
            radius - 8.0,
            Stroke::new(1.0, Color32::from_gray(40)),
        );

        let inner_color = if hovered {
            Color32::from_gray(25)
        } else {
            Color32::from_gray(20)
        };

        // Draw inner circle
        ui.painter()
            .circle_filled(center, radius - 6.0, inner_color);

        // Draw indicator dot
        let dot_pos = Pos2::new(
            center.x + (rotation.cos() * (radius - 12.0)),
            center.y + (rotation.sin() * (radius - 12.0)),
        );

        // Draw dot glow
        for i in (0..3).rev() {
            let alpha = 255 - (i * 60);

            ui.painter().circle_filled(
                dot_pos,
                3.0 + i as f32,
                Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha),
            );
        }

        // Draw center dot
        ui.painter().circle_filled(dot_pos, 3.0, Color32::WHITE);
    }

    fn draw_style_1(&self, ui: &mut Ui, rect: Rect) {
        let center = rect.center();
        let radius = (rect.width().min(rect.height()) * 0.5) - 2.0;

        // Define the angle range (270 degrees, leaving 90 degrees gap)
        let start_angle = -225.0 * PI / 180.0;
        let end_angle = 45.0 * PI / 180.0;

        // Calculate knob rotation based on value
        let rotation = start_angle + (end_angle - start_angle) * (self.value / 100.0);

        // Draw background arc
        let bg_points = Self::create_arc_points(center, radius, start_angle, end_angle, 32);

        ui.painter().add(PathShape::line(
            bg_points,
            Stroke::new(2.0, Color32::from_gray(60)),
        ));

        // Draw filled arc
        let filled_points = Self::create_arc_points(center, radius, start_angle, rotation, 32);

        ui.painter().add(PathShape::line(
            filled_points,
            Stroke::new(2.0, Color32::from_rgb(200, 200, 200)),
        ));

        // Draw knob center
        ui.painter()
            .circle_filled(center, radius - 4.0, Color32::from_gray(40));

        // Draw indicator line
        let indicator_length = radius - 6.0;
        let indicator_end = Pos2::new(
            center.x + rotation.cos() * indicator_length,
            center.y + rotation.sin() * indicator_length,
        );

        ui.painter()
            .line_segment([center, indicator_end], Stroke::new(2.0, Color32::WHITE));

        ui.painter().circle_stroke(
            center,
            radius - 2.0,
            Stroke::new(1.0, Color32::from_gray(180)),
        );
    }

    fn draw_style_2(&self, ui: &mut Ui, rect: Rect) {
        let center = rect.center();
        let radius = (rect.width().min(rect.height()) * 0.5) - 2.0;

        let start_angle = -225.0 * PI / 180.0;
        let end_angle = 45.0 * PI / 180.0;
        let rotation = start_angle + (end_angle - start_angle) * (self.value / 100.0);

        let color = Color::ACCENT;

        // Draw outer ring
        for i in 0..8 {
            let angle = (i as f32 * PI / 4.0) + (PI / 8.0);
            let offset = 1.5;
            let start = Pos2::new(
                center.x + (radius + offset) * angle.cos(),
                center.y + (radius + offset) * angle.sin(),
            );
            let end = Pos2::new(
                center.x + (radius + offset) * (angle + PI).cos(),
                center.y + (radius + offset) * (angle + PI).sin(),
            );

            ui.painter()
                .line_segment([start, end], Stroke::new(1.0, Color32::from_gray(180)));
        }

        // Draw base circle with gradient effect
        for i in 0..3 {
            let r = radius - (i as f32 * 2.0);

            ui.painter().circle_stroke(
                center,
                r,
                Stroke::new(1.0, Color32::from_gray(140 + (i * 20))),
            );
        }

        // Draw some marks
        for i in 0..27 {
            let angle = start_angle + (i as f32 * (end_angle - start_angle) / 26.0);
            let inner_point = Pos2::new(
                center.x + (radius - 8.0) * angle.cos(),
                center.y + (radius - 8.0) * angle.sin(),
            );
            let outer_point = Pos2::new(
                center.x + (radius - 4.0) * angle.cos(),
                center.y + (radius - 4.0) * angle.sin(),
            );

            ui.painter().line_segment(
                [inner_point, outer_point],
                Stroke::new(1.0, Color32::from_gray(100)),
            );
        }

        // Draw progress track
        let filled_points =
            Self::create_arc_points(center, radius - 6.0, start_angle, rotation, 64);

        ui.painter()
            .add(PathShape::line(filled_points, Stroke::new(2.5, color)));

        // Draw background track
        let bg_points = Self::create_arc_points(center, radius - 6.0, rotation, end_angle, 64);

        ui.painter().add(PathShape::line(
            bg_points,
            Stroke::new(2.0, Color32::from_gray(60)),
        ));

        // Draw center circle
        ui.painter()
            .circle_filled(center, radius - 12.0, Color32::from_gray(160));

        // Draw radial lines
        for i in 0..12 {
            let angle = i as f32 * PI / 6.0;
            let start = Pos2::new(center.x + 4.0 * angle.cos(), center.y + 4.0 * angle.sin());
            let end = Pos2::new(
                center.x + (radius - 14.0) * angle.cos(),
                center.y + (radius - 14.0) * angle.sin(),
            );

            ui.painter()
                .line_segment([start, end], Stroke::new(1.0, Color32::from_gray(140)));
        }

        // Draw indicator line
        let indicator_start = Pos2::new(
            center.x + 6.0 * rotation.cos(),
            center.y + 6.0 * rotation.sin(),
        );
        let indicator_end = Pos2::new(
            center.x + (radius - 14.0) * rotation.cos(),
            center.y + (radius - 14.0) * rotation.sin(),
        );

        ui.painter().line_segment(
            [indicator_start, indicator_end],
            Stroke::new(2.5, Color32::from_rgb(60, 60, 60)),
        );

        // Draw center cap
        ui.painter()
            .circle_filled(center, 4.0, Color32::from_gray(80));
        ui.painter()
            .circle_stroke(center, 4.0, Stroke::new(1.0, Color32::from_gray(180)));
    }

    fn create_arc_points(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        segments: usize,
    ) -> Vec<Pos2> {
        let mut points = Vec::with_capacity(segments + 1);
        let angle_step = (end_angle - start_angle) / segments as f32;

        for i in 0..=segments {
            let angle = start_angle + angle_step * i as f32;

            points.push(Pos2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            ));
        }

        points
    }

    pub const STYLES_COUNT: u8 = 3;
}

impl Widget for Potentiometer {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let desired_size = self.size.into();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);

        let hovered = response.hovered();

        self.value = ui
            .ctx()
            .animate_value_with_time(self.id.clone().into(), self.value, 0.25);

        // Draw potentiometer based on style
        match self.style {
            1 => self.draw_style_1(ui, rect),
            2 => self.draw_style_2(ui, rect),

            _ => self.draw_style_default(ui, rect, hovered),
        }

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

    pub const STYLES_COUNT: u8 = 0;
}

impl Widget for Joystick {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::from(self.size);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);

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

        let handle_color = if response.hovered() {
            Color32::from_gray(65)
        } else {
            Color32::from_gray(70)
        };

        // Handle main part
        painter.circle_filled(handle_pos, radius * 0.65, handle_color);

        response
    }
}

pub struct RotaryEncoder {
    size: (f32, f32),
}

impl RotaryEncoder {
    pub fn new(size: (f32, f32)) -> Self {
        Self { size }
    }

    pub const STYLES_COUNT: u8 = 0;
}

impl Widget for RotaryEncoder {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = self.size.into();
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);

        let center = rect.center();
        let radius = (rect.width().min(rect.height()) * 0.5) - 2.0;

        let body_color = if response.hovered() {
            Color32::from_gray(38)
        } else {
            Color32::from_gray(40)
        };

        // Main body
        ui.painter().circle_filled(center, radius, body_color);

        // Outer ring
        ui.painter()
            .circle_stroke(center, radius, Stroke::new(1.5, Color32::from_gray(80)));

        // Grip pattern
        for i in 0..12 {
            let angle = i as f32 * PI / 6.0;
            let inner_radius = radius - 12.0;
            let outer_radius = radius - 4.0;

            let start = Pos2::new(
                center.x + inner_radius * angle.cos(),
                center.y + inner_radius * angle.sin(),
            );
            let end = Pos2::new(
                center.x + outer_radius * angle.cos(),
                center.y + outer_radius * angle.sin(),
            );

            ui.painter()
                .line_segment([start, end], Stroke::new(2.0, Color32::from_gray(60)));
        }

        ui.painter().circle_stroke(
            center,
            radius - 1.0,
            Stroke::new(1.0, Color32::from_gray(100)),
        );

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

    pub const STYLES_COUNT: u8 = 0;
}

impl Widget for GLCD {
    fn ui(self, ui: &mut Ui) -> Response {
        let glcd_width = self.glcd_size.0 as f32 * self.pixel_size * self.scale;
        let glcd_height = self.glcd_size.1 as f32 * self.pixel_size * self.scale;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(glcd_width, glcd_height), Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);

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

pub struct ItemList<'a, T, F> {
    items: &'a mut Vec<T>,
    spacing: f32,
    height: f32,
    background_color: Color32,
    text_color: Color32,
    hover_background_color: Color32,
    border_color: Color32,
    on_item_removed: Option<F>,
}

impl<'a, T, F> ItemList<'a, T, F> {
    pub fn new(
        items: &'a mut Vec<T>,
        height: f32,
        background_color: Color32,
        text_color: Color32,
        hover_background_color: Color32,
        border_color: Color32,
    ) -> Self {
        Self {
            items,
            spacing: 6.0,
            height,
            background_color,
            text_color,
            hover_background_color,
            border_color,
            on_item_removed: None,
        }
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;

        self
    }

    pub fn on_item_removed(mut self, callback: F) -> Self
    where
        F: FnMut(usize),
    {
        self.on_item_removed = Some(callback);

        self
    }
}

impl<T: std::fmt::Display, F> Widget for ItemList<'_, T, F>
where
    F: FnMut(usize),
{
    fn ui(self, ui: &mut Ui) -> Response {
        let mut clicked_index = None;

        let response = Frame::default()
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (index, item) in self.items.iter().enumerate() {
                        let text = format!("{} ×", item);
                        let galley = ui.painter().layout_no_wrap(
                            text.clone(),
                            FontId::proportional(self.height * 0.6),
                            self.text_color,
                        );

                        let padding = Vec2::new(self.height * 0.5, 0.0);
                        let desired_size = galley.size() + padding * 2.0;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(desired_size.x, self.height),
                            Sense::click(),
                        );

                        let response = response.on_hover_cursor(CursorIcon::PointingHand);

                        if ui.is_rect_visible(rect) {
                            let is_hovered = response.hovered();
                            let bg_color = if is_hovered {
                                self.hover_background_color
                            } else {
                                self.background_color
                            };

                            // Draw border
                            ui.painter().rect_stroke(
                                rect,
                                self.height * 0.3,
                                (1.0, self.border_color),
                            );

                            // Draw background
                            ui.painter().rect_filled(rect, self.height * 0.3, bg_color);

                            // Center and draw text
                            let text_pos = Pos2::new(
                                rect.min.x + (rect.width() - galley.size().x) * 0.5,
                                rect.min.y + (rect.height() - galley.size().y) * 0.5,
                            );

                            ui.painter().galley(text_pos, galley, Color::RED);

                            if response.clicked() {
                                clicked_index = Some(index);
                            }
                        }

                        ui.add_space(self.spacing);
                    }
                });
            })
            .response;

        // Remove the clicked item if any
        if let Some(index) = clicked_index {
            self.items.remove(index);

            if let Some(mut callback) = self.on_item_removed {
                callback(index);
            }
        }

        response
    }
}

pub struct ToggleSwitch {
    checked: bool,
    size: (f32, f32),
}

impl ToggleSwitch {
    pub fn new(checked: bool, size: (f32, f32)) -> Self {
        Self { checked, size }
    }
}

impl Widget for ToggleSwitch {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::from(self.size);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let response = response.on_hover_cursor(CursorIcon::PointingHand);
        let painter = ui.painter();

        // Animation progress based on time
        let knob_position_value = animate_value(
            ui.ctx(),
            response.id,
            if self.checked { 1.0 } else { 0.0 },
            0.1,
        );

        // Background
        let rounding = rect.height() / 2.0;
        let background_color = if self.checked {
            ui.style().visuals.selection.bg_fill
        } else {
            ui.style().visuals.widgets.inactive.bg_fill
        };

        painter.rect_filled(rect, rounding, background_color);

        // Knob
        let knob_radius = (rect.height() - 4.0) / 2.0;
        let knob_x_range = rect.width() - 2.0 * (knob_radius + 2.0);
        let knob_x = rect.left() + knob_radius + 2.0 + (knob_x_range * knob_position_value);

        // Add a small bounce effect to the knob
        let bounce_offset = (knob_position_value * TAU).sin() * 1.0;
        let knob_y = rect.center().y + bounce_offset;

        painter.circle_filled(
            Pos2::new(knob_x, knob_y),
            knob_radius,
            ui.style().visuals.widgets.active.bg_fill,
        );

        response
    }
}

pub fn status_indicator(id: &'static str, ui: &mut Ui, color: Color32, size: f32) -> Response {
    let desired_size = Vec2::splat(size);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    if !ui.is_rect_visible(rect) {
        return response;
    }

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

// Helper functions for drawing shadows

pub fn draw_rect_shadow(
    ui: &mut Ui,
    rect: Rect,
    rounding: f32,
    shadow_size: f32,
    shadow_offset: (f32 /* x */, f32 /* y */),
) {
    if shadow_size <= 0.0 {
        return;
    }

    // Draw multiple layers of shadow with decreasing opacity
    for i in 1..=5 {
        let shadow_rect = rect.translate(shadow_offset.into());
        let expansion = shadow_size * (i as f32 / 5.0);
        let shadow_rect = shadow_rect.expand(expansion);
        let opacity = 40 - (i * 7); // Decreasing opacity for each layer

        ui.painter().rect_filled(
            shadow_rect,
            Rounding::same(rounding + expansion),
            Color32::from_black_alpha(opacity as u8),
        );
    }
}

pub fn draw_circle_shadow(
    ui: &mut Ui,
    center: Pos2,
    radius: f32,
    shadow_size: f32,
    shadow_offset: (f32 /* x */, f32 /* y */),
) {
    if shadow_size <= 0.0 {
        return;
    }

    // Draw multiple layers of shadow with decreasing opacity
    for i in 1..=5 {
        let shadow_center = center + shadow_offset.into();
        let shadow_radius = radius + (shadow_size * (i as f32 / 5.0));
        let opacity = 40 - (i * 7); // Decreasing opacity for each layer

        ui.painter().circle_filled(
            shadow_center,
            shadow_radius,
            Color32::from_black_alpha(opacity as u8),
        );
    }
}
