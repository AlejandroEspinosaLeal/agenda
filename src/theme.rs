use std::fs;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Vec2};

/// Design tokens matching Clarity Suite ("Mi Agenda") in light and accessible dark modes.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_canvas: Color32,
    pub bg_surface: Color32,
    pub bg_elevated: Color32,
    pub bg_cell_hover: Color32,
    pub bg_cell_selected: Color32,
    pub bg_cell_muted: Color32,
    pub border: Color32,
    pub border_subtle: Color32,
    pub ink_primary: Color32,
    pub ink_secondary: Color32,
    pub ink_muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub priority_high: Color32,
    pub priority_high_bg: Color32,
    pub priority_normal: Color32,
    pub priority_normal_bg: Color32,
    pub priority_low: Color32,
    pub priority_low_bg: Color32,
    pub done_green: Color32,
    pub done_bg: Color32,
}

impl Palette {
    /// Clarity Suite editorial light theme (`#F8F9FC` canvas, `#4338CA` indigo primary).
    pub const LIGHT: Self = Self {
        bg_canvas: Color32::from_rgb(248, 249, 252),     // #F8F9FC
        bg_surface: Color32::from_rgb(255, 255, 255),    // #FFFFFF
        bg_elevated: Color32::from_rgb(238, 237, 247),   // #EEEDF7 surface-container
        bg_cell_hover: Color32::from_rgb(244, 242, 253), // #F4F2FD
        bg_cell_selected: Color32::from_rgb(227, 223, 255),
        bg_cell_muted: Color32::from_rgb(244, 242, 250),
        border: Color32::from_rgb(199, 196, 215),        // #C7C4D7
        border_subtle: Color32::from_rgb(228, 228, 231), // #E4E4E7
        ink_primary: Color32::from_rgb(9, 9, 11),        // #09090B
        ink_secondary: Color32::from_rgb(70, 69, 84),    // #464554
        ink_muted: Color32::from_rgb(113, 113, 122),     // #71717A
        accent: Color32::from_rgb(67, 56, 202),          // #4338CA indigo
        accent_soft: Color32::from_rgb(227, 223, 255),   // #E3DFFF
        priority_high: Color32::from_rgb(185, 5, 56),    // #B90538
        priority_high_bg: Color32::from_rgb(255, 218, 219),
        priority_normal: Color32::from_rgb(180, 83, 9),  // #B45309
        priority_normal_bg: Color32::from_rgb(254, 243, 199),
        priority_low: Color32::from_rgb(4, 120, 87),     // #047857
        priority_low_bg: Color32::from_rgb(209, 250, 229),
        done_green: Color32::from_rgb(4, 120, 87),
        done_bg: Color32::from_rgb(238, 237, 247),
    };

    /// Accessible high-contrast dark theme.
    pub const DARK: Self = Self {
        bg_canvas: Color32::from_rgb(13, 17, 23),
        bg_surface: Color32::from_rgb(27, 34, 45),
        bg_elevated: Color32::from_rgb(36, 45, 60),
        bg_cell_hover: Color32::from_rgb(38, 49, 66),
        bg_cell_selected: Color32::from_rgb(36, 49, 96),
        bg_cell_muted: Color32::from_rgb(16, 20, 27),
        border: Color32::from_rgb(68, 82, 104),
        border_subtle: Color32::from_rgb(45, 55, 72),
        ink_primary: Color32::from_rgb(248, 250, 252),
        ink_secondary: Color32::from_rgb(203, 213, 225),
        ink_muted: Color32::from_rgb(148, 163, 184),
        accent: Color32::from_rgb(129, 140, 248), // bright indigo #818CF8
        accent_soft: Color32::from_rgb(40, 48, 92),
        priority_high: Color32::from_rgb(251, 113, 133),
        priority_high_bg: Color32::from_rgb(76, 29, 36),
        priority_normal: Color32::from_rgb(251, 191, 36),
        priority_normal_bg: Color32::from_rgb(74, 52, 20),
        priority_low: Color32::from_rgb(52, 211, 153),
        priority_low_bg: Color32::from_rgb(19, 68, 52),
        done_green: Color32::from_rgb(52, 211, 153),
        done_bg: Color32::from_rgb(32, 41, 54),
    };
}

/// Returns semantic `(fg, bg)` colors for Clarity Suite categories.
pub fn category_colors(tag: &str, dark: bool) -> (Color32, Color32) {
    if dark {
        match tag {
            "Trabajo" => (Color32::from_rgb(56, 189, 248), Color32::from_rgb(17, 50, 74)),
            "Personal" => (Color32::from_rgb(167, 139, 250), Color32::from_rgb(48, 34, 84)),
            "Estudio" => (Color32::from_rgb(251, 191, 36), Color32::from_rgb(74, 52, 20)),
            "Salud" => (Color32::from_rgb(52, 211, 153), Color32::from_rgb(19, 68, 52)),
            _ => (Color32::from_rgb(203, 213, 225), Color32::from_rgb(36, 45, 60)),
        }
    } else {
        match tag {
            "Trabajo" => (Color32::from_rgb(3, 105, 161), Color32::from_rgb(224, 242, 254)),
            "Personal" => (Color32::from_rgb(109, 40, 217), Color32::from_rgb(237, 233, 254)),
            "Estudio" => (Color32::from_rgb(180, 83, 9), Color32::from_rgb(254, 243, 199)),
            "Salud" => (Color32::from_rgb(4, 120, 87), Color32::from_rgb(209, 250, 229)),
            _ => (Color32::from_rgb(70, 69, 84), Color32::from_rgb(238, 237, 247)),
        }
    }
}

/// Loads Windows Segoe UI at runtime for crisp native typography with zero binary bloat.
pub fn setup_fonts(ctx: &egui::Context) {
    let Ok(bytes) = fs::read("C:\\Windows\\Fonts\\segoeui.ttf") else {
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("segoe_ui".to_owned(), FontData::from_owned(bytes));
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "segoe_ui".to_owned());
    }
    ctx.set_fonts(fonts);
}

/// Applies spacing, rounding, and color tokens to `egui::Context`.
pub fn apply_theme(ctx: &egui::Context, dark: bool) -> Palette {
    let p = if dark { Palette::DARK } else { Palette::LIGHT };

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.interact_size = Vec2::new(36.0, 28.0);

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = p.bg_canvas;
    visuals.window_fill = p.bg_surface;
    visuals.extreme_bg_color = if dark { p.bg_canvas } else { p.bg_elevated };
    visuals.faint_bg_color = p.bg_cell_hover;

    visuals.widgets.noninteractive.bg_fill = p.bg_surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.ink_primary);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

    visuals.widgets.inactive.bg_fill = p.bg_elevated;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.ink_primary);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);

    visuals.widgets.hovered.bg_fill = p.bg_cell_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.ink_primary);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);

    visuals.widgets.active.bg_fill = p.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.ink_primary);
    visuals.widgets.active.rounding = Rounding::same(8.0);

    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);

    style.visuals = visuals;
    ctx.set_style(style);
    p
}

/// Blends an arbitrary RGB accent with the surface color for readable cell backgrounds.
pub fn soft_tint(rgb: [u8; 3], surface: Color32, dark: bool) -> Color32 {
    let alpha = if dark { 0.24_f32 } else { 0.16_f32 };
    let blend = |fg: u8, bg: u8| -> u8 {
        (f32::from(fg) * alpha + f32::from(bg) * (1.0 - alpha)).round() as u8
    };
    Color32::from_rgb(
        blend(rgb[0], surface.r()),
        blend(rgb[1], surface.g()),
        blend(rgb[2], surface.b()),
    )
}
