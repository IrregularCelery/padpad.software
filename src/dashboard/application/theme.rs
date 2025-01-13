use eframe::egui::{self, Color32};

pub struct Color {}

impl Color {
    pub const BASE: Color32 = Color32::from_rgb(51, 51, 51);
    pub const TEXT: Color32 = Color32::from_rgb(200, 200, 200);
    pub const SURFACE0: Color32 = Color32::from_rgb(6, 6, 6);
    pub const SURFACE1: Color32 = Color32::from_rgb(15, 15, 15);
    pub const SURFACE2: Color32 = Color32::from_rgb(25, 25, 25);
    pub const OVERLAY0: Color32 = Color32::from_rgb(36, 36, 36);
    pub const OVERLAY1: Color32 = Color32::from_rgb(40, 40, 40);

    pub const YELLOW: Color32 = Color32::from_rgb(255, 203, 77);
    pub const GREEN: Color32 = Color32::from_rgb(77, 255, 83);
    pub const RED: Color32 = Color32::from_rgb(255, 77, 77);
    pub const BLUE: Color32 = Color32::from_rgb(77, 116, 255);
    pub const PURPLE: Color32 = Color32::from_rgb(133, 77, 255);
    pub const PINK: Color32 = Color32::from_rgb(255, 77, 208);
}

pub fn get_current_style() -> egui::Style {
    let mut style = egui::Style::default();

    // ---------- Text styles ----------

    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(20.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(20.0, egui::FontFamily::Proportional),
    );

    // ---------- Visual styles ----------

    style.visuals = egui::Visuals {
        dark_mode: true,
        override_text_color: Some(Color::TEXT),
        faint_bg_color: Color::SURFACE0,
        extreme_bg_color: Color::SURFACE0,
        window_fill: Color::SURFACE2,
        panel_fill: Color::BASE,
        window_rounding: 16.0.into(),
        menu_rounding: 8.0.into(),
        window_stroke: egui::Stroke {
            color: Color::OVERLAY1,
            ..Default::default()
        },
        window_shadow: egui::Shadow::NONE,
        popup_shadow: egui::Shadow::NONE,
        widgets: egui::style::Widgets {
            noninteractive: pack_widget_visual(Color::BASE),
            inactive: pack_widget_visual(Color::SURFACE0),
            hovered: pack_widget_visual(Color::SURFACE1),
            active: pack_widget_visual(Color::SURFACE2),
            open: pack_widget_visual(Color::SURFACE0),
        },
        ..Default::default()
    };

    // ---------- Spacing styles ----------

    let padding = egui::Vec2::new(8.0, 8.0);

    style.spacing.button_padding = padding;
    style.spacing.item_spacing = padding;

    // ---------- Interaction styles ----------

    style.interaction.selectable_labels = false;

    style
}

fn pack_widget_visual(fill: Color32) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: egui::Stroke {
            color: Color::OVERLAY0,
            width: 1.0,
        },
        fg_stroke: egui::Stroke {
            color: Color::TEXT,
            width: 1.0,
        },
        rounding: 6.0.into(),
        expansion: 0.0,
    }
}
