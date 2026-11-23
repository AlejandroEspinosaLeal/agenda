use eframe::egui::{
    self, Align, Color32, FontId, Key, Layout, RichText, Rounding, Sense, Stroke, Vec2,
};

use crate::app::AgendaApp;
use crate::calendar::{
    days_in_month, first_weekday_monday_zero, month_name_es, weekday_name_es, Date,
};
use crate::theme::Palette;
use crate::views;

const WEEKDAYS_MINI: [&str; 7] = ["Lu", "Ma", "Mi", "Ju", "Vi", "Sá", "Do"];

/// Renders the compact Windows-11-style flyout calendar + today's tasks.
pub fn render(app: &mut AgendaApp, ctx: &egui::Context, ui: &mut egui::Ui, p: Palette) {
    let date = app.selected_date;
    let wd_name = weekday_name_es(date.weekday_monday_zero());

    // Header: Selected day + Expand to Full + Theme toggle
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("{wd_name}, {} de {}", date.day, month_name_es(date.month)))
                    .size(15.5)
                    .strong()
                    .color(p.ink_primary),
            );
            ui.label(
                RichText::new(format!("{} {}", month_name_es(app.view_month), app.view_year))
                    .size(11.5)
                    .color(p.ink_muted),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("—").size(12.0).strong().color(p.ink_primary))
                        .fill(p.bg_elevated)
                        .min_size(Vec2::new(26.0, 26.0)),
                )
                .on_hover_text("Ocultar en bandeja del sistema")
                .clicked()
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("⤢").size(13.0).strong().color(Color32::WHITE))
                        .fill(p.accent)
                        .min_size(Vec2::new(28.0, 26.0)),
                )
                .on_hover_text("Abrir Mi Agenda completa")
                .clicked()
            {
                app.set_mini_mode(ctx, false);
            }

            let theme_lbl = if app.dark_mode { "☀" } else { "🌙" };
            if ui
                .add(
                    egui::Button::new(RichText::new(theme_lbl).size(12.0).color(p.ink_primary))
                        .fill(p.bg_elevated)
                        .min_size(Vec2::new(28.0, 26.0)),
                )
                .on_hover_text("Cambiar tema")
                .clicked()
            {
                app.dark_mode = !app.dark_mode;
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("›").size(14.0).color(p.ink_primary))
                        .fill(p.bg_surface)
                        .min_size(Vec2::new(24.0, 26.0)),
                )
                .clicked()
            {
                app.change_month(1);
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("‹").size(14.0).color(p.ink_primary))
                        .fill(p.bg_surface)
                        .min_size(Vec2::new(24.0, 26.0)),
                )
                .clicked()
            {
                app.change_month(-1);
            }
        });
    });

    ui.add_space(6.0);

    // Compact 7x6 Mini-Calendar Grid
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(10.0))
        .inner_margin(8.0)
        .show(ui, |ui| {
            let first_wd = first_weekday_monday_zero(app.view_year, app.view_month) as usize;
            let total_days = days_in_month(app.view_year, app.view_month) as usize;
            let rows = (first_wd + total_days).div_ceil(7).max(5);
            let cell_w = (ui.available_width() / 7.0).floor().max(36.0);
            let cell_h = 28.0_f32;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for wd in WEEKDAYS_MINI {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(cell_w, 18.0), Sense::hover());
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        wd,
                        FontId::proportional(10.5),
                        p.ink_muted,
                    );
                }
            });

            let origin = ui.cursor().min;
            let (_grid_rect, _) =
                ui.allocate_exact_size(Vec2::new(cell_w * 7.0, cell_h * rows as f32), Sense::hover());
            let painter = ui.painter();
            let first_of_month = Date::new(app.view_year, app.view_month, 1);

            for row in 0..rows {
                for col in 0..7 {
                    let idx = row * 7 + col;
                    let min = origin + Vec2::new(col as f32 * cell_w, row as f32 * cell_h);
                    let rect = egui::Rect::from_min_size(min, Vec2::new(cell_w, cell_h));
                    let cell_date = first_of_month.add_days(idx as i64 - first_wd as i64);
                    let in_month = idx >= first_wd && idx < first_wd + total_days;

                    if !in_month {
                        painter.text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            cell_date.day.to_string(),
                            FontId::proportional(11.0),
                            p.ink_muted.gamma_multiply(0.5),
                        );
                        continue;
                    }

                    let resp = ui.interact(rect, ui.id().with(("mini_d", cell_date)), Sense::click());
                    if resp.clicked() {
                        app.selected_date = cell_date;
                    }

                    let is_today = cell_date == app.today;
                    let is_selected = cell_date == app.selected_date;
                    let occs = app.store.tasks_visible_on(cell_date);
                    let has_pending = occs.iter().any(|o| !o.task.done);

                    if is_today {
                        painter.circle_filled(rect.center(), 12.0, p.accent);
                    } else if is_selected {
                        painter.circle_filled(rect.center(), 12.0, p.accent_soft);
                    } else if resp.hovered() {
                        painter.circle_filled(rect.center(), 12.0, p.bg_cell_hover);
                    }

                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        cell_date.day.to_string(),
                        FontId::proportional(11.5),
                        if is_today {
                            Color32::WHITE
                        } else if is_selected {
                            p.accent
                        } else {
                            p.ink_primary
                        },
                    );

                    if has_pending {
                        let dot_color = occs
                            .iter()
                            .filter(|o| !o.task.done)
                            .max_by_key(|o| o.task.priority)
                            .map(|o| {
                                views::task_colors(
                                    o.task.color,
                                    &o.task.tag,
                                    o.task.priority,
                                    p,
                                    app.dark_mode,
                                )
                                .0
                            })
                            .unwrap_or(p.accent);
                        painter.circle_filled(
                            egui::pos2(rect.center().x, rect.max.y - 3.0),
                            2.3,
                            if is_today { Color32::WHITE } else { dot_color },
                        );
                    }
                }
            }
        });

    ui.add_space(8.0);

    // Quick add in Mini Flyout
    ui.horizontal(|ui| {
        let input_w = (ui.available_width() - 38.0).max(120.0);
        let resp = ui.add_sized(
            Vec2::new(input_w, 28.0),
            egui::TextEdit::singleline(&mut app.new_title)
                .hint_text("Añadir tarea rápida..."),
        );
        let submit = (resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
            || ui
                .add(
                    egui::Button::new(RichText::new("+").size(14.0).strong().color(Color32::WHITE))
                        .fill(p.accent)
                        .min_size(Vec2::new(30.0, 28.0)),
                )
                .clicked();
        if submit && app.submit_draft(date) {
            resp.request_focus();
        }
    });

    ui.add_space(6.0);

    // Selected Day's Tasks
    let occurrences = app.store.tasks_visible_on(date);
    let mut toggle_target = None;
    let mut delete_target = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        if occurrences.is_empty() {
            ui.label(
                RichText::new("Sin tareas para este día.")
                    .size(12.0)
                    .color(p.ink_muted),
            );
        } else {
            for occ in &occurrences {
                let task = &occ.task;
                let (fg, _) =
                    views::task_colors(task.color, &task.tag, task.priority, p, app.dark_mode);
                egui::Frame::none()
                    .fill(if task.done { p.done_bg } else { p.bg_surface })
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut checked = task.done;
                            if ui.checkbox(&mut checked, "").changed() {
                                toggle_target = Some((occ.start_date, task.id));
                            }
                            let mut rt = RichText::new(&task.title).size(12.5);
                            rt = if task.done {
                                rt.strikethrough().color(p.ink_muted)
                            } else {
                                rt.color(p.ink_primary)
                            };
                            ui.add(egui::Label::new(rt).wrap());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("×").size(13.0).color(p.ink_muted),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    delete_target = Some((occ.start_date, task.id));
                                }
                                if !task.time.is_empty() {
                                    ui.label(RichText::new(&task.time).size(10.5).color(fg));
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        }
    });

    let mut changed = false;
    if let Some((sd, id)) = toggle_target {
        changed |= app.store.toggle_task(sd, id);
    }
    if let Some((sd, id)) = delete_target {
        changed |= app.store.delete_task(sd, id);
    }
    if changed {
        app.persist();
    }
}
