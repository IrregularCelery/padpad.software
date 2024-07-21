use eframe::egui::{self, Color32};

const TEXT_COLOR: Color32 = Color32::from_rgb(202, 211, 245);
//const ROSEWATER_COLOR: Color32 = Color32::from_rgb(244, 219, 214);
//const FLAMINGO_COLOR: Color32 = Color32::from_rgb(240, 198, 198);
//const PINK_COLOR: Color32 = Color32::from_rgb(245, 189, 230);
//const MAUVE_COLOR: Color32 = Color32::from_rgb(198, 160, 246);
//const RED_COLOR: Color32 = Color32::from_rgb(237, 135, 150);
const MAROON_COLOR: Color32 = Color32::from_rgb(238, 153, 160);
const PEACH_COLOR: Color32 = Color32::from_rgb(245, 169, 127);
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
const SURFACE2_COLOR: Color32 = Color32::from_rgb(91, 96, 120);
const SURFACE1_COLOR: Color32 = Color32::from_rgb(73, 77, 100);
const SURFACE0_COLOR: Color32 = Color32::from_rgb(54, 58, 79);
const BASE_COLOR: Color32 = Color32::from_rgb(36, 39, 58);
const MANTLE_COLOR: Color32 = Color32::from_rgb(30, 32, 48);
const CRUST_COLOR: Color32 = Color32::from_rgb(24, 25, 38);

pub fn get_current_style() -> egui::Style {
    let mut style = egui::Style::default();
    let old = style.visuals;

    style.interaction.selectable_labels = false;
    style.visuals = egui::Visuals {
        override_text_color: Some(TEXT_COLOR),
        faint_bg_color: SURFACE0_COLOR,
        extreme_bg_color: CRUST_COLOR,
        code_bg_color: MANTLE_COLOR,
        warn_fg_color: PEACH_COLOR,
        error_fg_color: MAROON_COLOR,
        window_fill: BASE_COLOR,
        panel_fill: BASE_COLOR,
        window_stroke: egui::Stroke {
            color: OVERLAY1_COLOR,
            ..Default::default()
        },
        widgets: egui::style::Widgets {
            noninteractive: pack_widget_visual(old.widgets.noninteractive, BASE_COLOR),
            inactive: pack_widget_visual(old.widgets.inactive, SURFACE0_COLOR),
            hovered: pack_widget_visual(old.widgets.hovered, SURFACE2_COLOR),
            active: pack_widget_visual(old.widgets.active, SURFACE1_COLOR),
            open: pack_widget_visual(old.widgets.open, SURFACE0_COLOR),
        },
        ..Default::default()
    };

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
