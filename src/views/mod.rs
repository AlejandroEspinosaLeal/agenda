pub mod daily;
pub mod mini;
pub mod monthly;
pub mod weekly;

use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

use crate::model::{Priority, Task};
use crate::theme::{category_colors, soft_tint, Palette};

pub const TAG_PRESETS: [&str; 4] = ["Trabajo", "Personal", "Estudio", "Salud"];

pub const COLOR_PRESETS: [[u8; 3]; 7] = [
    [220, 38, 38],  // Rojo
    [217, 119, 6],  // Ámbar
    [16, 185, 129], // Esmeralda
    [67, 56, 202],  // Índigo
    [139, 92, 246], // Violeta
    [236, 72, 153], // Rosa
    [6, 182, 212],  // Turquesa
];

pub fn badge(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).strong().color(fg));
        });
}

pub fn category_filter_pills(
    ui: &mut egui::Ui,
    active_category: &mut Option<String>,
    p: Palette,
    dark: bool,
) {
    ui.horizontal_wrapped(|ui| {
        let all_active = active_category.is_none();
        let all_btn = egui::Button::new(
            RichText::new("Todas")
                .size(11.5)
                .strong()
                .color(if all_active { Color32::WHITE } else { p.ink_secondary }),
        )
        .fill(if all_active { p.accent } else { p.bg_elevated })
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(14.0));
        if ui.add(all_btn).clicked() {
            *active_category = None;
        }

        for cat in TAG_PRESETS {
            let selected = active_category.as_deref() == Some(cat);
            let (fg, bg) = category_colors(cat, dark);
            let btn = egui::Button::new(
                RichText::new(cat)
                    .size(11.5)
                    .strong()
                    .color(if selected { Color32::WHITE } else { fg }),
            )
            .fill(if selected { fg } else { bg })
            .stroke(Stroke::NONE)
            .rounding(Rounding::same(14.0));
            if ui.add(btn).clicked() {
                if selected {
                    *active_category = None;
                } else {
                    *active_category = Some(cat.to_owned());
                }
            }
        }
    });
}

pub fn task_matches_filters(
    task: &Task,
    query: &str,
    category_filter: Option<&str>,
) -> bool {
    if let Some(cat) = category_filter {
        if task.tag != cat {
            return false;
        }
    }
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    task.title.to_lowercase().contains(&q)
        || task.notes.to_lowercase().contains(&q)
        || task.tag.to_lowercase().contains(&q)
        || task.location.to_lowercase().contains(&q)
}

pub fn task_colors(
    custom: Option<[u8; 3]>,
    tag: &str,
    priority: Priority,
    p: Palette,
    dark: bool,
) -> (Color32, Color32) {
    if let Some(rgb) = custom {
        let fg = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        let bg = soft_tint(rgb, p.bg_surface, dark);
        (fg, bg)
    } else if !tag.is_empty() {
        category_colors(tag, dark)
    } else {
        priority_colors(priority, p)
    }
}

pub fn priority_colors(priority: Priority, p: Palette) -> (Color32, Color32) {
    match priority {
        Priority::High => (p.priority_high, p.priority_high_bg),
        Priority::Normal => (p.priority_normal, p.priority_normal_bg),
        Priority::Low => (p.priority_low, p.priority_low_bg),
    }
}

pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_owned();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push('…');
    out
}
