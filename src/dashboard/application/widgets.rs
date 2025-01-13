use std::sync::Arc;

use eframe::{
    egui::{Color32, Context, Id, Pos2, Response, Sense, Stroke, Ui, Vec2},
    epaint,
};

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
