use eframe::egui::{self, Color32};

const TEXT_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
//const ROSEWATER_COLOR: Color32 = Color32::from_rgb(244, 219, 214);
//const FLAMINGO_COLOR: Color32 = Color32::from_rgb(240, 198, 198);
//const PINK_COLOR: Color32 = Color32::from_rgb(245, 189, 230);
//const MAUVE_COLOR: Color32 = Color32::from_rgb(198, 160, 246);
//const RED_COLOR: Color32 = Color32::from_rgb(237, 135, 150);
//const MAROON_COLOR: Color32 = Color32::from_rgb(238, 153, 160);
//const PEACH_COLOR: Color32 = Color32::from_rgb(245, 169, 127);
//const YELLOW_COLOR: Color32 = Color32::from_rgb(238, 212, 159);
//const GREEN_COLOR: Color32 = Color32::from_rgb(166, 218, 149);
//const TEAL_COLOR: Color32 = Color32::from_rgb(139, 213, 202);
//const SKY_COLOR: Color32 = Color32::from_rgb(145, 215, 227);
//const SAPPHIRE_COLOR: Color32 = Color32::from_rgb(125, 196, 228);
//const BLUE_COLOR: Color32 = Color32::from_rgb(138, 173, 244);
//const LAVENDER_COLOR: Color32 = Color32::from_rgb(183, 189, 248);
//const OVERLAY0_COLOR: Color32 = Color32::from_rgb(110, 115, 141);
//const OVERLAY2_COLOR: Color32 = Color32::from_rgb(147, 154, 183);
const OVERLAY1_COLOR: Color32 = Color32::from_rgb(128, 135, 162);
const SURFACE2_COLOR: Color32 = Color32::from_rgb(15, 15, 15);
const SURFACE1_COLOR: Color32 = Color32::from_rgb(25, 25, 25);
const SURFACE0_COLOR: Color32 = Color32::from_rgb(6, 6, 6);
const BASE_COLOR: Color32 = Color32::from_rgb(51, 51, 51);

pub fn get_current_style() -> egui::Style {
    let mut style = egui::Style::default();
    let old = style.visuals;

    // ---------- Text styles ----------

    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );

    // ---------- Visual styles ----------

    style.visuals = egui::Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_COLOR),
        faint_bg_color: SURFACE0_COLOR,
        extreme_bg_color: SURFACE0_COLOR,
        window_fill: SURFACE1_COLOR,
        panel_fill: BASE_COLOR,
        window_rounding: 16.0.into(),
        menu_rounding: 8.0.into(),
        window_stroke: egui::Stroke {
            color: OVERLAY1_COLOR,
            ..Default::default()
        },
        window_shadow: egui::Shadow::NONE,
        popup_shadow: egui::Shadow::NONE,
        widgets: egui::style::Widgets {
            noninteractive: pack_widget_visual(old.widgets.noninteractive, BASE_COLOR),
            inactive: pack_widget_visual(old.widgets.inactive, SURFACE0_COLOR),
            hovered: pack_widget_visual(old.widgets.hovered, SURFACE2_COLOR),
            active: pack_widget_visual(old.widgets.active, SURFACE1_COLOR),
            open: pack_widget_visual(old.widgets.open, SURFACE0_COLOR),
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

fn pack_widget_visual(
    old: egui::style::WidgetVisuals,
    fill: Color32,
) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: egui::Stroke {
            color: OVERLAY1_COLOR,
            ..Default::default()
        },
        fg_stroke: egui::Stroke {
            color: TEXT_COLOR,
            ..Default::default()
        },
        ..old
    }
}
