use eframe::{
    egui::{Color32, Pos2, Response, Sense, Stroke, Ui, Vec2},
    epaint,
};

pub enum Modal {
    None,
    Message {
        title: String,
        message: String,
    },
    YesNo {
        title: String,
        question: String,
        on_yes: Option<Box<dyn FnMut()>>,
        on_no: Option<Box<dyn FnMut()>>,
    },
    Custom {
        content: Box<dyn FnMut(&mut Ui)>,
    },
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
