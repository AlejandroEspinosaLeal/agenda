pub mod daily;
pub mod monthly;
pub mod weekly;

use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

use crate::model::{Priority, Recurrence, Task};
use crate::theme::{soft_tint, Palette, Radius, Spacing};

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
        .rounding(Rounding::same(Radius::SM))
        .inner_margin(egui::Margin::symmetric(Spacing::SM, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).strong().color(fg));
        });
}

/// Single reusable category filter bar (`Todas`, `Trabajo`, `Personal`, `Estudio`, `Salud`),
/// consuming `Palette::category_token` exclusively.
pub fn category_filter_pills(
    ui: &mut egui::Ui,
    active_category: &mut Option<String>,
    p: Palette,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(Spacing::XS + 2.0, Spacing::XS);

        let all_active = active_category.is_none();
        let all_btn = egui::Button::new(
            RichText::new("Todas")
                .size(11.5)
                .strong()
                .color(if all_active { Color32::WHITE } else { p.ink_secondary }),
        )
        .fill(if all_active { p.accent } else { p.bg_elevated })
        .stroke(Stroke::new(
            1.0_f32,
            if all_active { p.accent } else { p.border_subtle },
        ))
        .rounding(Rounding::same(Radius::PILL));
        if ui.add(all_btn).clicked() {
            *active_category = None;
        }

        for cat in TAG_PRESETS {
            let selected = active_category.as_deref() == Some(cat);
            let tok = p.category_token(cat);
            let btn = egui::Button::new(
                RichText::new(cat)
                    .size(11.5)
                    .strong()
                    .color(if selected { Color32::WHITE } else { tok.text }),
            )
            .fill(if selected { tok.solid } else { tok.bg })
            .stroke(Stroke::new(1.0_f32, if selected { tok.solid } else { tok.border }))
            .rounding(Rounding::same(Radius::PILL));
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

/// Reusable separated controls for Schedule (`Todo el día` vs `Horario`) and Recurrence.
pub fn schedule_and_recurrence_controls(
    ui: &mut egui::Ui,
    all_day: &mut bool,
    start_time: &mut String,
    end_time: &mut String,
    recurrence: &mut Recurrence,
    p: Palette,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Horario:").size(11.5).color(p.ink_secondary));
        for (is_all_day, lbl) in [(true, "Todo el día"), (false, "Con hora")] {
            let sel = *all_day == is_all_day;
            let btn = egui::Button::new(
                RichText::new(lbl)
                    .size(11.0)
                    .color(if sel { Color32::WHITE } else { p.ink_primary }),
            )
            .fill(if sel { p.accent } else { p.bg_elevated })
            .rounding(Rounding::same(Radius::SM));
            if ui.add(btn).clicked() {
                *all_day = is_all_day;
                if is_all_day {
                    start_time.clear();
                    end_time.clear();
                } else if start_time.trim().is_empty() {
                    *start_time = "09:30".to_owned();
                }
            }
        }
    });

    if !*all_day {
        ui.add_space(Spacing::XS);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(start_time)
                    .hint_text("Inicio 09:30")
                    .desired_width(84.0),
            );
            ui.label(RichText::new("–").color(p.ink_muted));
            ui.add(
                egui::TextEdit::singleline(end_time)
                    .hint_text("Fin 10:30")
                    .desired_width(84.0),
            );
        });
    }

    ui.add_space(Spacing::XS);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Repetir:").size(11.5).color(p.ink_secondary));
        for (rec, lbl) in [
            (Recurrence::None, "No"),
            (Recurrence::Daily, "Diario"),
            (Recurrence::Weekdays, "L–V"),
            (Recurrence::Weekly, "Semanal"),
            (Recurrence::Monthly, "Mensual"),
        ] {
            let sel = *recurrence == rec;
            let btn = egui::Button::new(
                RichText::new(lbl)
                    .size(11.0)
                    .color(if sel { Color32::WHITE } else { p.ink_primary }),
            )
            .fill(if sel { p.accent } else { p.bg_elevated })
            .rounding(Rounding::same(Radius::SM));
            if ui.add(btn).clicked() {
                *recurrence = rec;
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
) -> (Color32, Color32) {
    if let Some(rgb) = custom {
        let fg = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        let bg = soft_tint(rgb, p.bg_surface, p.dark);
        (fg, bg)
    } else if !tag.is_empty() {
        let tok = p.category_token(tag);
        (tok.text, tok.bg)
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
