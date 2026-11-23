use std::fs;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreset {
    #[default]
    Indigo,
    Ocean,
    Forest,
    Terracotta,
    Rose,
    Lavender,
}

impl ThemePreset {
    pub const ALL: [Self; 6] = [
        Self::Indigo,
        Self::Ocean,
        Self::Forest,
        Self::Terracotta,
        Self::Rose,
        Self::Lavender,
    ];

    pub const fn label_es(self) -> &'static str {
        match self {
            Self::Indigo => "Índigo",
            Self::Ocean => "Océano",
            Self::Forest => "Bosque",
            Self::Terracotta => "Terracota",
            Self::Rose => "Rosa",
            Self::Lavender => "Lavanda",
        }
    }

    pub const fn accent_color(self, dark: bool) -> Color32 {
        match (self, dark) {
            (Self::Indigo, false) => Color32::from_rgb(67, 56, 202),
            (Self::Indigo, true) => Color32::from_rgb(129, 140, 248),
            (Self::Ocean, false) => Color32::from_rgb(3, 105, 161),
            (Self::Ocean, true) => Color32::from_rgb(56, 189, 248),
            (Self::Forest, false) => Color32::from_rgb(4, 120, 87),
            (Self::Forest, true) => Color32::from_rgb(52, 211, 153),
            (Self::Terracotta, false) => Color32::from_rgb(194, 65, 12),
            (Self::Terracotta, true) => Color32::from_rgb(251, 146, 60),
            (Self::Rose, false) => Color32::from_rgb(190, 24, 93),
            (Self::Rose, true) => Color32::from_rgb(244, 114, 182),
            (Self::Lavender, false) => Color32::from_rgb(109, 40, 217),
            (Self::Lavender, true) => Color32::from_rgb(167, 139, 250),
        }
    }
}

/// Centralized spacing tokens (4px base scale).
pub struct Spacing;

#[allow(dead_code)]
impl Spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Centralized corner radius tokens.
pub struct Radius;

#[allow(dead_code)]
impl Radius {
    pub const SM: f32 = 6.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const PILL: f32 = 14.0;
}

/// Unified semantic category token (solid accent, 12-15% tinted background, border, and WCAG AA text).
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct CategoryToken {
    pub solid: Color32,
    pub bg: Color32,
    pub border: Color32,
    pub text: Color32,
}

/// Centralized design tokens for Light and Dark modes (WCAG AA compliant).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
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
    pub fn for_theme(preset: ThemePreset, dark: bool) -> Self {
        let mut p = if dark { Self::DARK } else { Self::LIGHT };
        if preset == ThemePreset::Indigo {
            return p;
        }
        let accent = preset.accent_color(dark);
        p.accent = accent;
        if dark {
            p.accent_soft = blend_rgb(accent, p.bg_surface, 0.24);
            p.bg_cell_selected = blend_rgb(accent, p.bg_surface, 0.28);
            p.bg_cell_hover = blend_rgb(accent, p.bg_surface, 0.14);
            p.bg_elevated = blend_rgb(accent, p.bg_surface, 0.12);
        } else {
            p.accent_soft = blend_rgb(accent, p.bg_surface, 0.14);
            p.bg_cell_selected = blend_rgb(accent, p.bg_surface, 0.18);
            p.bg_cell_hover = blend_rgb(accent, p.bg_surface, 0.08);
            p.bg_elevated = blend_rgb(accent, p.bg_canvas, 0.08);
            p.bg_canvas = blend_rgb(accent, Color32::from_rgb(250, 250, 252), 0.03);
        }
        p
    }

    /// Clarity Suite editorial light theme (`#F8F9FC` canvas, `#4338CA` indigo primary).
    pub const LIGHT: Self = Self {
        dark: false,
        bg_canvas: Color32::from_rgb(248, 249, 252),     // #F8F9FC
        bg_surface: Color32::from_rgb(255, 255, 255),    // #FFFFFF
        bg_elevated: Color32::from_rgb(238, 237, 247),   // #EEEDF7
        bg_cell_hover: Color32::from_rgb(244, 242, 253), // #F4F2FD
        bg_cell_selected: Color32::from_rgb(227, 223, 255),
        bg_cell_muted: Color32::from_rgb(244, 242, 250),
        border: Color32::from_rgb(199, 196, 215),        // #C7C4D7
        border_subtle: Color32::from_rgb(228, 228, 231), // #E4E4E7
        ink_primary: Color32::from_rgb(9, 9, 11),        // #09090B (>18:1 on white)
        ink_secondary: Color32::from_rgb(63, 63, 76),    // #3F3F4C (>9:1 on white)
        ink_muted: Color32::from_rgb(96, 96, 108),       // #60606C (>5.8:1 WCAG AA)
        accent: Color32::from_rgb(67, 56, 202),          // #4338CA (>7.5:1 on white)
        accent_soft: Color32::from_rgb(227, 223, 255),   // #E3DFFF
        priority_high: Color32::from_rgb(185, 5, 56),    // #B90538
        priority_high_bg: Color32::from_rgb(255, 224, 228),
        priority_normal: Color32::from_rgb(161, 72, 6),  // #A14806 (WCAG AA)
        priority_normal_bg: Color32::from_rgb(254, 243, 199),
        priority_low: Color32::from_rgb(4, 108, 78),     // #046C4E (WCAG AA)
        priority_low_bg: Color32::from_rgb(209, 250, 229),
        done_green: Color32::from_rgb(4, 108, 78),
        done_bg: Color32::from_rgb(238, 237, 247),
    };

    /// Accessible high-contrast dark theme (WCAG AA compliant).
    pub const DARK: Self = Self {
        dark: true,
        bg_canvas: Color32::from_rgb(13, 17, 23),        // #0D1117
        bg_surface: Color32::from_rgb(27, 34, 45),       // #1B222D
        bg_elevated: Color32::from_rgb(36, 45, 60),      // #242D3C
        bg_cell_hover: Color32::from_rgb(38, 49, 66),    // #263142
        bg_cell_selected: Color32::from_rgb(36, 49, 96), // #243160
        bg_cell_muted: Color32::from_rgb(16, 20, 27),    // #10141B
        border: Color32::from_rgb(76, 91, 114),          // #4C5B72
        border_subtle: Color32::from_rgb(48, 59, 77),    // #303B4D
        ink_primary: Color32::from_rgb(248, 250, 252),   // #F8FAFC (>14:1 on surface)
        ink_secondary: Color32::from_rgb(203, 213, 225), // #CBD5E1 (>9:1 on surface)
        ink_muted: Color32::from_rgb(148, 163, 184),     // #94A3B8 (>5.5:1 WCAG AA)
        accent: Color32::from_rgb(129, 140, 248),        // #818CF8 (>5.1:1 on surface)
        accent_soft: Color32::from_rgb(40, 48, 92),
        priority_high: Color32::from_rgb(253, 130, 149),
        priority_high_bg: Color32::from_rgb(66, 30, 40),
        priority_normal: Color32::from_rgb(252, 201, 68),
        priority_normal_bg: Color32::from_rgb(64, 48, 22),
        priority_low: Color32::from_rgb(72, 220, 168),
        priority_low_bg: Color32::from_rgb(21, 60, 48),
        done_green: Color32::from_rgb(72, 220, 168),
        done_bg: Color32::from_rgb(32, 41, 54),
    };

    /// Returns unified semantic tokens (solid, 14% tinted bg, border, WCAG AA text) for a category.
    pub fn category_token(&self, tag: &str) -> CategoryToken {
        let solid = if self.dark {
            match tag {
                "Trabajo" => Color32::from_rgb(56, 189, 248),
                "Personal" => Color32::from_rgb(167, 139, 250),
                "Estudio" => Color32::from_rgb(251, 191, 36),
                "Salud" => Color32::from_rgb(52, 211, 153),
                _ => self.ink_secondary,
            }
        } else {
            match tag {
                "Trabajo" => Color32::from_rgb(2, 132, 199),
                "Personal" => Color32::from_rgb(109, 40, 217),
                "Estudio" => Color32::from_rgb(180, 83, 9),
                "Salud" => Color32::from_rgb(5, 150, 105),
                _ => self.ink_secondary,
            }
        };

        let text = if self.dark {
            solid
        } else {
            match tag {
                "Trabajo" => Color32::from_rgb(3, 105, 161),
                "Personal" => Color32::from_rgb(91, 33, 182),
                "Estudio" => Color32::from_rgb(146, 64, 14),
                "Salud" => Color32::from_rgb(4, 108, 78),
                _ => self.ink_secondary,
            }
        };

        let bg_alpha = if self.dark { 0.15_f32 } else { 0.13_f32 };
        let border_alpha = if self.dark { 0.38_f32 } else { 0.30_f32 };

        CategoryToken {
            solid,
            bg: blend_rgb(solid, self.bg_surface, bg_alpha),
            border: blend_rgb(solid, self.bg_surface, border_alpha),
            text,
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
pub fn apply_theme(ctx: &egui::Context, preset: ThemePreset, dark: bool) -> Palette {
    let p = Palette::for_theme(preset, dark);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(Spacing::SM, Spacing::SM);
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
    visuals.widgets.noninteractive.rounding = Rounding::same(Radius::MD);

    visuals.widgets.inactive.bg_fill = p.bg_elevated;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.ink_primary);
    visuals.widgets.inactive.rounding = Rounding::same(Radius::MD);

    visuals.widgets.hovered.bg_fill = p.bg_cell_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.ink_primary);
    visuals.widgets.hovered.rounding = Rounding::same(Radius::MD);

    visuals.widgets.active.bg_fill = p.accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.ink_primary);
    visuals.widgets.active.rounding = Rounding::same(Radius::MD);

    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);

    style.visuals = visuals;
    ctx.set_style(style);
    p
}

/// Blends an arbitrary RGB accent with the surface color for readable cell backgrounds.
pub fn soft_tint(rgb: [u8; 3], surface: Color32, dark: bool) -> Color32 {
    let alpha = if dark { 0.15_f32 } else { 0.14_f32 };
    blend_rgb(Color32::from_rgb(rgb[0], rgb[1], rgb[2]), surface, alpha)
}

fn blend_rgb(fg: Color32, bg: Color32, alpha: f32) -> Color32 {
    let blend = |f: u8, b: u8| -> u8 {
        (f32::from(f) * alpha + f32::from(b) * (1.0 - alpha)).round() as u8
    };
    Color32::from_rgb(
        blend(fg.r(), bg.r()),
        blend(fg.g(), bg.g()),
        blend(fg.b(), bg.b()),
    )
}
