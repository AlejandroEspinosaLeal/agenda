use eframe::egui::{
    self, Align, Color32, FontId, Key, Layout, RichText, Rounding, Sense, Stroke, Vec2,
};

use crate::app::{ActiveView, AgendaApp};
use crate::calendar::{
    days_in_month, first_weekday_monday_zero, month_name_es, month_short_es, weekday_name_es, Date,
};
use crate::theme::{Palette, Radius, Spacing};
use crate::views::{self, TAG_PRESETS};

const WEEKDAYS: [&str; 7] = ["LUN", "MAR", "MIÉ", "JUE", "VIE", "SÁB", "DOM"];

pub fn render(app: &mut AgendaApp, ui: &mut egui::Ui, p: Palette) {
    let (pending_m, done_m) = app.store.month_counts(app.view_year, app.view_month);
    let total_days_m = usize::from(days_in_month(app.view_year, app.view_month));
    let days_with_tasks = (1..=total_days_m as u8)
        .filter(|&d| !app.store.tasks_visible_on(Date::new(app.view_year, app.view_month, d)).is_empty())
        .count();
    let free_pct = ((total_days_m - days_with_tasks) * 100) / total_days_m.max(1);

    // Top control card (‹ Hoy ›  Noviembre 2026 + Category Filter Pills + Stats)
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(12.0))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                let nav_btn = |text: &str| {
                    egui::Button::new(RichText::new(text).size(15.0).strong().color(p.ink_primary))
                        .fill(p.bg_elevated)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(30.0, 30.0))
                };

                if ui.add(nav_btn("‹")).on_hover_text("Mes anterior (←)").clicked() {
                    app.change_month(-1);
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("Hoy").size(12.5).strong().color(p.ink_primary))
                            .fill(p.bg_elevated)
                            .stroke(Stroke::NONE)
                            .min_size(Vec2::new(48.0, 30.0)),
                    )
                    .clicked()
                {
                    app.today = Date::today();
                    app.select_date(app.today);
                }
                if ui.add(nav_btn("›")).on_hover_text("Mes siguiente (→)").clicked() {
                    app.change_month(1);
                }

                ui.add_space(6.0);
                ui.label(
                    RichText::new(month_name_es(app.view_month))
                        .size(24.0)
                        .strong()
                        .color(p.ink_primary),
                );
                ui.label(
                    RichText::new(app.view_year.to_string())
                        .size(22.0)
                        .color(p.ink_muted),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    views::category_filter_pills(ui, &mut app.category_filter, p);
                });
            });

            ui.add_space(Spacing::SM);
            ui.horizontal(|ui| {
                views::badge(
                    ui,
                    &format!("• {pending_m} Pendientes"),
                    p.priority_high,
                    p.priority_high_bg,
                );
                views::badge(
                    ui,
                    &format!("{done_m} Completadas"),
                    p.ink_secondary,
                    p.bg_elevated,
                );
                views::badge(
                    ui,
                    &format!("{free_pct}% Tiempo Libre"),
                    p.accent,
                    p.accent_soft,
                );

                if !app.focus_mode {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let insp_lbl = if app.monthly_inspector_open {
                            "Ocultar panel"
                        } else {
                            "Ver panel"
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(insp_lbl).size(11.5).color(p.ink_secondary),
                                )
                                .fill(p.bg_elevated)
                                .rounding(Rounding::same(Radius::SM)),
                            )
                            .clicked()
                        {
                            app.monthly_inspector_open = !app.monthly_inspector_open;
                        }
                    });
                }
            });
        });

    ui.add_space(Spacing::MD);

    // Main split: >= 75% width for Calendar Grid + Collapsible Contextual Inspector
    let avail = ui.available_size();
    let show_inspector = !app.focus_mode && app.monthly_inspector_open;
    let inspector_w = if show_inspector {
        (avail.x * 0.25).clamp(250.0, 290.0)
    } else {
        0.0
    };
    let grid_w = (avail.x - if show_inspector { inspector_w + Spacing::MD } else { 0.0 }).max(420.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(grid_w, avail.y),
            Layout::top_down(Align::Min),
            |ui| {
                render_grid(app, ui, p, grid_w, avail.y);
            },
        );

        if show_inspector {
            ui.add_space(Spacing::MD);
            ui.allocate_ui_with_layout(
                Vec2::new(inspector_w, avail.y),
                Layout::top_down(Align::Min),
                |ui| {
                    render_inspector(app, ui, p);
                },
            );
        }
    });
}

fn render_grid(app: &mut AgendaApp, ui: &mut egui::Ui, p: Palette, grid_w: f32, avail_h: f32) {
    let first_wd = first_weekday_monday_zero(app.view_year, app.view_month) as usize;
    let total_days = days_in_month(app.view_year, app.view_month) as usize;
    let total_cells = first_wd + total_days;
    let rows = total_cells.div_ceil(7).max(5);

    let gap = 6.0_f32;
    let header_h = 24.0_f32;
    let col_w = ((grid_w - gap * 6.0) / 7.0).floor().max(64.0);
    let row_h = ((avail_h - header_h - gap * rows as f32) / rows as f32)
        .floor()
        .max(72.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (i, name) in WEEKDAYS.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(col_w, header_h), Sense::hover());
            let color = if i >= 5 { p.ink_muted } else { p.ink_secondary };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                *name,
                FontId::proportional(11.0),
                color,
            );
        }
    });

    let grid_origin = ui.cursor().min;
    let grid_size = Vec2::new(
        col_w * 7.0 + gap * 6.0,
        row_h * rows as f32 + gap * (rows as f32 - 1.0),
    );
    let (_grid_rect, _) = ui.allocate_exact_size(grid_size, Sense::hover());
    let painter = ui.painter();

    let first_of_month = Date::new(app.view_year, app.view_month, 1);
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    let drag_range = app
        .monthly_range_drag
        .map(|(a, b)| if a <= b { (a, b) } else { (b, a) });

    for row in 0..rows {
        for col in 0..7 {
            let idx = row * 7 + col;
            let cell_min = grid_origin
                + Vec2::new(col as f32 * (col_w + gap), row as f32 * (row_h + gap));
            let cell_rect = egui::Rect::from_min_size(cell_min, Vec2::new(col_w, row_h));
            let day_offset = idx as i64 - first_wd as i64;
            let cell_date = first_of_month.add_days(day_offset);
            let in_month = idx >= first_wd && idx < first_wd + total_days;

            if !in_month {
                painter.rect_filled(cell_rect, Rounding::same(8.0), p.bg_cell_muted);
                painter.rect_stroke(
                    cell_rect,
                    Rounding::same(8.0),
                    Stroke::new(1.0_f32, p.border_subtle),
                );
                painter.text(
                    cell_rect.min + Vec2::new(10.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    cell_date.day.to_string(),
                    FontId::proportional(12.0),
                    p.ink_muted,
                );
                continue;
            }

            let is_selected = cell_date == app.selected_date;
            let is_today = cell_date == app.today;

            let response = ui.interact(
                cell_rect,
                ui.id().with(("m_day", cell_date)),
                Sense::click_and_drag(),
            );
            if response.drag_started() {
                app.monthly_range_drag = Some((cell_date, cell_date));
            }
            if let Some((start, _)) = app.monthly_range_drag {
                if pointer_pos.is_some_and(|pos| cell_rect.contains(pos)) {
                    app.monthly_range_drag = Some((start, cell_date));
                }
            }
            let in_drag_range = drag_range.is_some_and(|(s, e)| cell_date >= s && cell_date <= e);
            let plus_rect = egui::Rect::from_min_size(
                egui::pos2(cell_rect.max.x - 24.0, cell_rect.min.y + 6.0),
                Vec2::splat(18.0),
            );
            let plus_resp =
                ui.interact(plus_rect, ui.id().with(("m_add", cell_date)), Sense::click());
            if plus_resp.clicked() {
                app.selected_date = cell_date;
                app.show_new_task_modal = true;
            } else if response.clicked() {
                app.selected_date = cell_date;
            }
            if response.double_clicked() {
                app.selected_date = cell_date;
                app.active_view = ActiveView::Daily;
            }

            let tasks: Vec<_> = app
                .store
                .tasks_visible_on(cell_date)
                .into_iter()
                .filter(|occ| {
                    views::task_matches_filters(
                        &occ.task,
                        app.search_query.trim(),
                        app.category_filter.as_deref(),
                    )
                })
                .collect();

            let dominant_pending = tasks
                .iter()
                .filter(|occ| !occ.task.done)
                .max_by_key(|occ| occ.task.priority);

            let bg = if in_drag_range {
                p.accent_soft
            } else if let Some(occ) = dominant_pending {
                views::task_colors(occ.task.color, &occ.task.tag, occ.task.priority, p).1
            } else if !tasks.is_empty() {
                p.done_bg
            } else if is_selected {
                p.bg_cell_selected
            } else if response.hovered() || plus_resp.hovered() {
                p.bg_cell_hover
            } else {
                p.bg_surface
            };
            painter.rect_filled(cell_rect, Rounding::same(8.0), bg);

            // Day number + HOY pill (tabular monospace)
            let num_pos = cell_rect.min + Vec2::new(10.0, 8.0);
            if is_today {
                let badge_rect = egui::Rect::from_min_size(
                    num_pos - Vec2::new(3.0, 2.0),
                    Vec2::new(22.0, 20.0),
                );
                painter.rect_filled(badge_rect, Rounding::same(10.0), p.accent);
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    cell_date.day.to_string(),
                    FontId::monospace(12.0),
                    Color32::WHITE,
                );
                painter.text(
                    egui::pos2(badge_rect.max.x + 6.0, badge_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "HOY",
                    FontId::proportional(10.0),
                    p.accent,
                );
            } else {
                painter.text(
                    num_pos,
                    egui::Align2::LEFT_TOP,
                    cell_date.day.to_string(),
                    FontId::monospace(13.0),
                    if is_selected { p.accent } else { p.ink_primary },
                );
            }

            // Up to 3 compact blocks per cell (color dot + time + truncated title) + clickable "+N más"
            let chip_h = 18.0_f32;
            let chip_gap = 3.0_f32;
            let max_preview = ((row_h - 36.0) / (chip_h + chip_gap))
                .floor()
                .clamp(1.0, 3.0) as usize;

            for (shown, occ) in tasks.iter().enumerate() {
                if shown >= max_preview {
                    break;
                }
                let task = &occ.task;
                let y = cell_rect.min.y + 32.0 + shown as f32 * (chip_h + chip_gap);
                let chip_rect = egui::Rect::from_min_size(
                    egui::pos2(cell_rect.min.x + 6.0, y),
                    Vec2::new(col_w - 12.0, chip_h),
                );
                let (fg, chip_bg) = if task.done {
                    (p.ink_muted, p.done_bg)
                } else if dominant_pending.is_some() {
                    let (fg, _) = views::task_colors(task.color, &task.tag, task.priority, p);
                    (fg, p.bg_surface)
                } else {
                    views::task_colors(task.color, &task.tag, task.priority, p)
                };

                painter.rect_filled(chip_rect, Rounding::same(4.0), chip_bg);
                painter.circle_filled(
                    egui::pos2(chip_rect.min.x + 7.0, chip_rect.center().y),
                    3.0,
                    if task.done { p.done_green } else { fg },
                );

                let prefix = if task.span_days > 1 && occ.start_date != cell_date {
                    "↔ "
                } else {
                    ""
                };
                let display = if task.time.is_empty() {
                    format!("{prefix}{}", task.title)
                } else {
                    format!("{prefix}{} {}", task.time, task.title)
                };
                let max_chars = ((col_w - 28.0) / 6.2).floor().max(4.0) as usize;
                painter.text(
                    egui::pos2(chip_rect.min.x + 14.0, chip_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    views::truncate_chars(&display, max_chars),
                    FontId::proportional(10.5),
                    if task.done { p.ink_muted } else { p.ink_primary },
                );
            }

            if tasks.len() > max_preview {
                let more_rect = egui::Rect::from_min_size(
                    egui::pos2(cell_rect.max.x - 54.0, cell_rect.max.y - 18.0),
                    Vec2::new(48.0, 15.0),
                );
                let more_resp =
                    ui.interact(more_rect, ui.id().with(("m_more", cell_date)), Sense::click());
                if more_resp.clicked() {
                    app.selected_date = cell_date;
                    app.monthly_inspector_open = true;
                }
                painter.text(
                    more_rect.right_center(),
                    egui::Align2::RIGHT_CENTER,
                    format!("+{} más", tasks.len() - max_preview),
                    FontId::proportional(10.0),
                    if more_resp.hovered() { p.accent } else { p.ink_muted },
                );
            }

            let stroke = if in_drag_range || is_selected {
                Stroke::new(2.0_f32, p.accent)
            } else if response.hovered() || plus_resp.hovered() {
                Stroke::new(1.0_f32, p.accent)
            } else {
                Stroke::new(1.0_f32, p.border_subtle)
            };
            painter.rect_stroke(cell_rect, Rounding::same(8.0), stroke);

            if response.hovered() || plus_resp.hovered() {
                painter.circle_filled(
                    plus_rect.center(),
                    8.5,
                    if plus_resp.hovered() { p.accent } else { p.accent_soft },
                );
                painter.text(
                    plus_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    FontId::proportional(12.0),
                    if plus_resp.hovered() { Color32::WHITE } else { p.accent },
                );
            }
        }
    }

    if let Some((a, b)) = app.monthly_range_drag {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        let span = (start.days_until(end) + 1).clamp(1, 31) as u8;
        if ui.input(|i| i.pointer.primary_down()) {
            if span > 1 {
                if let Some(pos) = pointer_pos {
                    let pill_rect =
                        egui::Rect::from_min_size(pos + Vec2::new(12.0, 12.0), Vec2::new(66.0, 22.0));
                    painter.rect_filled(pill_rect, Rounding::same(6.0), p.accent);
                    painter.text(
                        pill_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{span} días"),
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                }
            }
        } else {
            app.monthly_range_drag = None;
            if span > 1 {
                app.open_span_draft(start, span);
            }
        }
    }
}

fn render_inspector(app: &mut AgendaApp, ui: &mut egui::Ui, p: Palette) {
    let date = app.selected_date;
    let wd_name = weekday_name_es(date.weekday_monday_zero());
    let occurrences: Vec<_> = app
        .store
        .tasks_visible_on(date)
        .into_iter()
        .filter(|occ| {
            views::task_matches_filters(
                &occ.task,
                app.search_query.trim(),
                app.category_filter.as_deref(),
            )
        })
        .collect();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // DETALLE DEL DÍA card
        egui::Frame::none()
            .fill(p.bg_surface)
            .stroke(Stroke::new(1.0_f32, p.border_subtle))
            .rounding(Rounding::same(12.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("DETALLE DEL DÍA")
                        .size(10.5)
                        .strong()
                        .color(p.ink_muted),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{wd_name}, {} {}",
                            date.day,
                            month_short_es(date.month)
                        ))
                        .size(18.0)
                        .strong()
                        .color(p.ink_primary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        views::badge(
                            ui,
                            &format!("{} Tareas", occurrences.len()),
                            p.accent,
                            p.accent_soft,
                        );
                    });
                });

                ui.add_space(8.0);
                let mut toggle_target = None;
                let mut delete_target = None;

                if occurrences.is_empty() {
                    ui.label(
                        RichText::new("Sin tareas para este día.")
                            .size(12.5)
                            .color(p.ink_muted),
                    );
                } else {
                    for occ in &occurrences {
                        let task = &occ.task;
                        let (fg, _) = views::task_colors(
                            task.color,
                            &task.tag,
                            task.priority,
                            p,
                        );
                        let card_resp = egui::Frame::none()
                            .fill(p.bg_canvas)
                            .stroke(Stroke::new(1.0_f32, p.border_subtle))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin {
                                left: 13.0,
                                right: 10.0,
                                top: 10.0,
                                bottom: 10.0,
                            })
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        let cat_label = if task.tag.is_empty() {
                                            "GENERAL".to_owned()
                                        } else {
                                            task.tag.to_uppercase()
                                        };
                                        ui.label(RichText::new(cat_label).size(10.5).strong().color(fg));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if !task.time.is_empty() {
                                                let time_str = if task.end_time.is_empty() {
                                                    task.time.clone()
                                                } else {
                                                    format!("{} - {}", task.time, task.end_time)
                                                };
                                                ui.label(
                                                    RichText::new(time_str)
                                                        .size(11.0)
                                                        .color(p.ink_muted),
                                                );
                                            }
                                        });
                                    });

                                    ui.add_space(3.0);
                                    let mut title_rt = RichText::new(&task.title).size(13.5).strong();
                                    if task.done {
                                        title_rt = title_rt.strikethrough().color(p.ink_muted);
                                    } else {
                                        title_rt = title_rt.color(p.ink_primary);
                                    }
                                    ui.add(egui::Label::new(title_rt).wrap());

                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "Prioridad {}",
                                                task.priority.label_es()
                                            ))
                                            .size(11.0)
                                            .color(p.ink_muted),
                                        );
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
                                            let mut checked = task.done;
                                            if ui.checkbox(&mut checked, "").changed() {
                                                toggle_target = Some((occ.start_date, task.id));
                                            }
                                        });
                                    });
                                });
                            });
                        let strip = egui::Rect::from_min_size(
                            card_resp.response.rect.min,
                            Vec2::new(3.5, card_resp.response.rect.height()),
                        );
                        ui.painter().rect_filled(
                            strip,
                            Rounding::same(2.0),
                            if task.done { p.done_green } else { fg },
                        );
                        ui.add_space(6.0);
                    }
                }

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
            });

        ui.add_space(12.0);

        // Agregar Rápido card (fixed sizing so nothing clips!)
        egui::Frame::none()
            .fill(p.bg_surface)
            .stroke(Stroke::new(1.0_f32, p.border_subtle))
            .rounding(Rounding::same(12.0))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Agregar Rápido")
                        .size(15.0)
                        .strong()
                        .color(p.ink_primary),
                );
                ui.add_space(6.0);

                let title_resp = ui.add(
                    egui::TextEdit::singleline(&mut app.new_title)
                        .hint_text("¿Qué tienes planeado?")
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(6.0);
                egui::ComboBox::from_id_salt("quick_tag")
                    .selected_text(if app.new_tag.is_empty() {
                        "Categoría"
                    } else {
                        &app.new_tag
                    })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for cat in TAG_PRESETS {
                            ui.selectable_value(&mut app.new_tag, cat.to_owned(), cat);
                        }
                    });

                ui.add_space(6.0);
                views::schedule_and_recurrence_controls(
                    ui,
                    &mut app.new_all_day,
                    &mut app.new_time,
                    &mut app.new_end_time,
                    &mut app.new_recurrence,
                    p,
                );

                ui.add_space(8.0);
                let submit = (title_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    || ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!(
                                    "+ Guardar en {}",
                                    month_name_es(date.month)
                                ))
                                .size(12.5)
                                .strong()
                                .color(Color32::WHITE),
                            )
                            .fill(p.accent)
                            .stroke(Stroke::NONE)
                            .min_size(Vec2::new(ui.available_width(), 34.0)),
                        )
                        .clicked();

                if submit && app.submit_draft(date) {
                    title_resp.request_focus();
                }
            });

        ui.add_space(12.0);

        // Notas Rápidas del Día (autoguardado con debounce)
        egui::Frame::none()
            .fill(p.bg_surface)
            .stroke(Stroke::new(1.0_f32, p.border_subtle))
            .rounding(Rounding::same(Radius::LG))
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Notas del Día")
                                .size(13.5)
                                .strong()
                                .color(p.ink_primary),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let st = if app.notes_dirty_at.is_some() {
                                "Guardando..."
                            } else {
                                "Sincronizado"
                            };
                            ui.label(
                                RichText::new(st)
                                    .size(10.5)
                                    .color(if app.notes_dirty_at.is_some() {
                                        p.accent
                                    } else {
                                        p.done_green
                                    }),
                            );
                        });
                    });
                    ui.add_space(Spacing::XS);
                    let key = date.to_key();
                    let note = app.store.daily_notes.entry(key).or_default();
                    let resp = ui.add(
                        egui::TextEdit::multiline(note)
                            .hint_text("Apuntes rápidos de este día...")
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );
                    if resp.changed() {
                        app.mark_notes_dirty();
                    }
                });
            });
    });
}
