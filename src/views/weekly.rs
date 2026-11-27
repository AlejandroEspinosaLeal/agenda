use eframe::egui::{self, Align, FontId, Layout, RichText, Rounding, Sense, Stroke, Vec2};

use crate::app::{ActiveView, AgendaApp, GridCreateDrag};
use crate::calendar::{month_name_es, month_short_es, Date};
use crate::theme::{Palette, Radius, Spacing};
use crate::timeblock::{self, TimeInterval};
use crate::views::{self, TAG_PRESETS};

const WEEKDAY_SHORT: [&str; 7] = ["LUN", "MAR", "MIÉ", "JUE", "VIE", "SÁB", "DOM"];

pub fn render(app: &mut AgendaApp, ui: &mut egui::Ui, p: Palette) {
    let week_start = app.selected_date.week_monday();
    let week_end = week_start.add_days(6);
    let week_num = week_start.iso_week();

    let mut week_total = 0usize;
    let mut week_done = 0usize;
    for offset in 0..7 {
        let day = week_start.add_days(offset);
        for occ in app.store.tasks_visible_on(day) {
            if views::task_matches_filters(
                &occ.task,
                app.search_query.trim(),
                app.category_filter.as_deref(),
            ) {
                week_total += 1;
                if occ.task.done {
                    week_done += 1;
                }
            }
        }
    }
    let free_pct = 100usize.saturating_sub((week_total * 4).min(95));

    // Cabecera del planificador semanal + Toggle Modo Dual
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            views::badge(
                ui,
                &format!("PLANIFICADOR ACTIVO  ·  Ciclo {}", week_start.year),
                p.accent,
                p.accent_soft,
            );
            ui.add_space(Spacing::XS);
            let range_title = if week_start.month == week_end.month {
                format!(
                    "Semana {week_num} · {} - {} de {} {}",
                    week_start.day,
                    week_end.day,
                    month_name_es(week_end.month),
                    week_end.year
                )
            } else {
                format!(
                    "Semana {week_num} · {} {} - {} {} {}",
                    week_start.day,
                    month_short_es(week_start.month),
                    week_end.day,
                    month_short_es(week_end.month),
                    week_end.year
                )
            };
            ui.label(
                RichText::new(range_title)
                    .size(22.0)
                    .strong()
                    .color(p.ink_primary),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("›").size(15.0).strong().color(p.ink_primary))
                        .fill(p.bg_surface)
                        .min_size(Vec2::new(30.0, 32.0)),
                )
                .on_hover_text("Semana siguiente (→)")
                .clicked()
            {
                app.select_date(app.selected_date.add_days(7));
            }
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Esta Semana")
                            .size(12.5)
                            .strong()
                            .color(p.ink_primary),
                    )
                    .fill(p.bg_surface)
                    .min_size(Vec2::new(96.0, 32.0)),
                )
                .clicked()
            {
                app.select_date(app.today);
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("‹").size(15.0).strong().color(p.ink_primary))
                        .fill(p.bg_surface)
                        .min_size(Vec2::new(30.0, 32.0)),
                )
                .on_hover_text("Semana anterior (←)")
                .clicked()
            {
                app.select_date(app.selected_date.add_days(-7));
            }
        });
    });

    ui.add_space(Spacing::SM);

    // Filtros por categoría + conmutador de modo dual
    ui.horizontal_wrapped(|ui| {
        views::category_filter_pills(ui, &mut app.category_filter, p);
        ui.add_space(Spacing::SM);
        for (is_sched, lbl) in [(true, "Modo Horario"), (false, "Modo Tablero")] {
            let sel = app.weekly_schedule_mode == is_sched;
            if ui
                .add(
                    egui::Button::new(RichText::new(lbl).size(12.0).strong().color(if sel {
                        p.on_accent
                    } else {
                        p.ink_primary
                    }))
                    .fill(if sel { p.accent } else { p.bg_surface })
                    .min_size(Vec2::new(102.0, 32.0)),
                )
                .clicked()
            {
                app.weekly_schedule_mode = is_sched;
            }
        }
    });

    ui.add_space(Spacing::SM);

    // Métricas clave y resumen semanal
    ui.horizontal_wrapped(|ui| {
        let footer_lbl = if app.weekly_footer_open {
            "v Ocultar resumen"
        } else {
            "^ Ver resumen"
        };
        if ui
            .add(
                egui::Button::new(RichText::new(footer_lbl).size(11.5).color(p.ink_secondary))
                    .fill(p.bg_elevated)
                    .rounding(Rounding::same(Radius::SM)),
            )
            .clicked()
        {
            app.weekly_footer_open = !app.weekly_footer_open;
        }

        views::badge(ui, &format!("LIBRE: {free_pct}%"), p.accent, p.accent_soft);
        views::badge(
            ui,
            &format!("{week_done} Completadas"),
            p.done_green,
            p.done_bg,
        );
        views::badge(
            ui,
            &format!("{week_total} Bloques"),
            p.ink_primary,
            p.bg_surface,
        );
    });

    ui.add_space(Spacing::SM);

    let avail = ui.available_size();
    let bottom_h = if app.weekly_footer_open {
        174.0_f32
    } else {
        0.0
    };
    let board_h = if app.weekly_footer_open {
        (avail.y - bottom_h - 18.0).clamp(240.0, 440.0)
    } else {
        avail.y.max(340.0)
    };

    if app.weekly_schedule_mode {
        render_schedule_mode(app, ui, p, week_start, avail.x, board_h);
    } else {
        render_board_mode(app, ui, p, week_start, avail.x, board_h);
    }

    if app.weekly_footer_open {
        ui.add_space(Spacing::MD);
        render_analytical_footer(
            app, ui, p, week_start, week_num, week_total, week_done, avail.x, bottom_h,
        );
    }
}

fn render_schedule_mode(
    app: &mut AgendaApp,
    ui: &mut egui::Ui,
    p: Palette,
    week_start: Date,
    avail_x: f32,
    board_h: f32,
) {
    let axis_w = 50.0_f32;
    let col_gap = 4.0_f32;
    let col_w = ((avail_x - axis_w - col_gap * 6.0 - 16.0) / 7.0)
        .floor()
        .max(76.0);
    let start_hour: u32 = 6;
    let end_hour: u32 = 23;
    let hour_h = 36.0_f32;
    let px_per_min = hour_h / 60.0;
    let grid_h = (end_hour - start_hour) as f32 * hour_h;
    let now_min = timeblock::current_local_minutes();

    let mut toggle_target = None;
    let mut persist_needed = false;
    let mut open_draft: Option<(Date, u32, u32)> = None;
    let mut move_update: Option<(u64, Date, u32, u32)> = None;

    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(Radius::LG))
        .inner_margin(Spacing::SM)
        .show(ui, |ui| {
            ui.set_width((avail_x - 18.0).max(400.0));
            ui.vertical(|ui| {
                // Cabecera de los 7 días
                ui.horizontal(|ui| {
                    ui.add_space(axis_w);
                    ui.spacing_mut().item_spacing.x = col_gap;
                    for (i, wd) in WEEKDAY_SHORT.iter().enumerate() {
                        let day = week_start.add_days(i as i64);
                        let is_today = day == app.today;
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::new(col_w, 26.0), Sense::click());
                        if resp.clicked() {
                            app.select_date(day);
                        }
                        if is_today {
                            ui.painter().rect_filled(r, Rounding::same(Radius::SM), p.accent_soft);
                        }
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{wd} {}", day.day),
                            FontId::monospace(11.5),
                            if is_today { p.accent } else { p.ink_primary },
                        );
                    }
                });

                ui.add_space(Spacing::XS);

                // Grid horario compartido de 06:00 a 23:00
                egui::ScrollArea::vertical()
                    .id_salt("weekly_schedule_grid")
                    .max_height((board_h - 48.0).max(200.0))
                    .show(ui, |ui| {
                        let total_w = axis_w + col_w * 7.0 + col_gap * 6.0;
                        let (canvas, _) =
                            ui.allocate_exact_size(Vec2::new(total_w, grid_h), Sense::hover());
                        let painter = ui.painter();

                        // Líneas de hora horizontales
                        for h in start_hour..=end_hour {
                            let y = canvas.min.y + (h - start_hour) as f32 * hour_h;
                            painter.text(
                                egui::pos2(canvas.min.x + 4.0, y),
                                egui::Align2::LEFT_CENTER,
                                format!("{h:02}:00"),
                                FontId::monospace(10.0),
                                p.ink_muted,
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(canvas.min.x + axis_w, y),
                                    egui::pos2(canvas.max.x, y),
                                ],
                                Stroke::new(1.0_f32, p.border_subtle),
                            );
                        }

                        // Columnas de los 7 días y bloques con particionado de solapamientos + Drag & Resize
                        let min_vis = start_hour * 60;
                        let max_vis = end_hour * 60;
                        let y_to_snap_min = |y: f32| -> u32 {
                            let rel = ((y - canvas.min.y) / px_per_min).max(0.0);
                            let raw = min_vis + rel.round() as u32;
                            timeblock::snap_to_15(raw).clamp(min_vis, max_vis)
                        };
                        let x_to_day = |x: f32| -> Date {
                            let rel_x = (x - (canvas.min.x + axis_w)).max(0.0);
                            let col_idx = (rel_x / (col_w + col_gap)).floor() as i64;
                            week_start.add_days(col_idx.clamp(0, 6))
                        };

                        for i in 0..7 {
                            let day = week_start.add_days(i as i64);
                            let col_x = canvas.min.x + axis_w + i as f32 * (col_w + col_gap);
                            let col_rect = egui::Rect::from_min_size(
                                egui::pos2(col_x, canvas.min.y),
                                Vec2::new(col_w, grid_h),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(col_x, canvas.min.y),
                                    egui::pos2(col_x, canvas.max.y),
                                ],
                                Stroke::new(1.0_f32, p.border_subtle),
                            );

                            // Drop desde Backlog de la barra lateral
                            if let Some(backlog_id) = app.drag_backlog_id {
                                if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                    if col_rect.contains(ptr) {
                                        let s_min = y_to_snap_min(ptr.y).min(1380);
                                        let py = canvas.min.y
                                            + (s_min - min_vis) as f32 * px_per_min;
                                        let pr = egui::Rect::from_min_size(
                                            egui::pos2(col_x + 2.0, py),
                                            Vec2::new(col_w - 4.0, hour_h),
                                        );
                                        painter.rect_filled(pr, Rounding::same(4.0), p.accent_soft);
                                        painter.rect_stroke(
                                            pr,
                                            Rounding::same(4.0),
                                            Stroke::new(1.5_f32, p.accent),
                                        );
                                        if !ui.input(|inp| inp.pointer.primary_down()) {
                                            if app.store.update_item_schedule(
                                                backlog_id,
                                                Some(day),
                                                s_min,
                                                s_min + 60,
                                            ) {
                                                persist_needed = true;
                                            }
                                            app.drag_backlog_id = None;
                                        }
                                    }
                                }
                            }

                            // Creación directa por clic o arrastre sobre franja vacía
                            let bg_resp = ui.interact(
                                col_rect,
                                ui.id().with(("wk_bg_create", day)),
                                Sense::click_and_drag(),
                            );
                            if bg_resp.drag_started() {
                                if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                    let m = y_to_snap_min(ptr.y);
                                    app.grid_create_drag = Some(GridCreateDrag {
                                        day,
                                        anchor_min: m,
                                        current_min: (m + 60).min(max_vis),
                                    });
                                }
                            }
                            if bg_resp.dragged() {
                                if let (Some(drag), Some(ptr)) =
                                    (&mut app.grid_create_drag, ui.ctx().pointer_interact_pos())
                                {
                                    if drag.day == day {
                                        drag.current_min = y_to_snap_min(ptr.y);
                                    }
                                }
                            }
                            if bg_resp.drag_stopped() {
                                if let Some(drag) = app.grid_create_drag.take() {
                                    let (s, e) = if drag.current_min >= drag.anchor_min {
                                        (drag.anchor_min, drag.current_min.max(drag.anchor_min + 30))
                                    } else {
                                        (drag.current_min, drag.anchor_min.max(drag.current_min + 30))
                                    };
                                    open_draft = Some((drag.day, s, e));
                                }
                            } else if bg_resp.clicked() {
                                if let Some(ptr) = bg_resp.interact_pointer_pos() {
                                    let s = y_to_snap_min(ptr.y).min(max_vis - 30);
                                    open_draft = Some((day, s, (s + 60).min(max_vis)));
                                }
                            }

                            if let Some(drag) = app.grid_create_drag {
                                if drag.day == day {
                                    let s = drag.anchor_min.min(drag.current_min);
                                    let e = drag.anchor_min.max(drag.current_min).max(s + 15);
                                    let top = canvas.min.y + (s - min_vis) as f32 * px_per_min;
                                    let h = ((e - s) as f32 * px_per_min).max(14.0);
                                    let pr = egui::Rect::from_min_size(
                                        egui::pos2(col_x + 2.0, top),
                                        Vec2::new(col_w - 4.0, h),
                                    );
                                    painter.rect_filled(pr, Rounding::same(4.0), p.accent_soft);
                                    painter.rect_stroke(
                                        pr,
                                        Rounding::same(4.0),
                                        Stroke::new(1.5_f32, p.accent),
                                    );
                                }
                            }

                            let timed: Vec<_> = app
                                .store
                                .tasks_visible_on(day)
                                .into_iter()
                                .filter(|o| {
                                    o.task.time_bounds_minutes().is_some()
                                        && views::task_matches_filters(
                                            &o.task,
                                            app.search_query.trim(),
                                            app.category_filter.as_deref(),
                                        )
                                })
                                .collect();

                            let ivs: Vec<TimeInterval> = timed
                                .iter()
                                .filter_map(|o| {
                                    o.task.time_bounds_minutes().map(|(s, e)| TimeInterval {
                                        id: o.task.id,
                                        start_min: s,
                                        end_min: e,
                                    })
                                })
                                .collect();

                            for blk in timeblock::layout_day_blocks(ivs) {
                                let Some(occ) = timed.iter().find(|o| o.task.id == blk.id) else {
                                    continue;
                                };
                                if blk.end_min <= min_vis || blk.start_min >= max_vis {
                                    continue;
                                }
                                let s = blk.start_min.max(min_vis);
                                let e = blk.end_min.min(max_vis);
                                let top_y = canvas.min.y + (s - min_vis) as f32 * px_per_min;
                                let h = ((e - s) as f32 * px_per_min).max(20.0);
                                let sub_w = col_w / blk.total_cols as f32;
                                let x = col_x + blk.col as f32 * sub_w + 1.5;
                                let w = (sub_w - 3.0).max(20.0);
                                let rect =
                                    egui::Rect::from_min_size(egui::pos2(x, top_y), Vec2::new(w, h));

                                let (fg, bg) = views::task_colors(
                                    occ.task.color,
                                    &occ.task.tag,
                                    occ.task.priority,
                                    p,
                                );
                                painter.rect_filled(
                                    rect,
                                    Rounding::same(4.0),
                                    if occ.task.done { p.done_bg } else { bg },
                                );
                                painter.rect_filled(
                                    egui::Rect::from_min_size(rect.min, Vec2::new(3.0, h)),
                                    Rounding::same(2.0),
                                    if occ.task.done { p.done_green } else { fg },
                                );

                                let max_c = ((w - 8.0) / 6.0).floor().max(3.0) as usize;
                                painter.text(
                                    rect.min + Vec2::new(6.0, 3.0),
                                    egui::Align2::LEFT_TOP,
                                    views::truncate_chars(&occ.task.title, max_c),
                                    FontId::proportional(10.5),
                                    if occ.task.done {
                                        p.ink_muted
                                    } else {
                                        p.ink_primary
                                    },
                                );

                                let body_rect = egui::Rect::from_min_max(
                                    rect.min,
                                    egui::pos2(rect.max.x, (rect.max.y - 5.0).max(rect.min.y + 12.0)),
                                );
                                let resp = ui.interact(
                                    body_rect,
                                    ui.id().with(("wk_blk", day, occ.task.id)),
                                    Sense::click_and_drag(),
                                );
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                                if resp.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                        let dur = (blk.end_min - blk.start_min).max(15);
                                        let new_s = y_to_snap_min(ptr.y).min(1440 - dur);
                                        let new_day = x_to_day(ptr.x);
                                        move_update = Some((occ.task.id, new_day, new_s, new_s + dur));
                                    }
                                }
                                if resp.drag_stopped() {
                                    persist_needed = true;
                                } else if resp.clicked() {
                                    toggle_target = Some((occ.start_date, occ.task.id));
                                }

                                let res_rect = egui::Rect::from_min_max(
                                    egui::pos2(rect.min.x, body_rect.max.y),
                                    rect.max,
                                );
                                let res_resp = ui.interact(
                                    res_rect,
                                    ui.id().with(("wk_res", day, occ.task.id)),
                                    Sense::drag(),
                                );
                                if res_resp.hovered() || res_resp.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                                }
                                if res_resp.dragged() {
                                    if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                        let new_e = y_to_snap_min(ptr.y).max(blk.start_min + 15);
                                        move_update = Some((occ.task.id, day, blk.start_min, new_e));
                                    }
                                }
                                if res_resp.drag_stopped() {
                                    persist_needed = true;
                                }

                                let _ = resp.on_hover_text(format!(
                                    "{} ({})\nArrastra para mover de hora o día · Clic para completar",
                                    occ.task.title, occ.task.time
                                ));
                            }

                            // Línea de hora actual sobre la columna de hoy
                            if day == app.today && now_min >= min_vis && now_min <= max_vis {
                                let y = canvas.min.y + (now_min - min_vis) as f32 * px_per_min;
                                painter.line_segment(
                                    [egui::pos2(col_x, y), egui::pos2(col_x + col_w, y)],
                                    Stroke::new(2.0_f32, p.priority_high),
                                );
                                painter.circle_filled(
                                    egui::pos2(col_x, y),
                                    3.5,
                                    p.priority_high,
                                );
                            }
                        }
                    });
            });
        });

    if let Some((id, nd, ns, ne)) = move_update {
        app.store.update_item_schedule(id, Some(nd), ns, ne);
    }
    if let Some((sd, id)) = toggle_target {
        persist_needed |= app.store.toggle_task(sd, id);
    }
    if persist_needed {
        app.persist();
    }
    if let Some((d, s, e)) = open_draft {
        app.open_timed_draft(d, s, e);
    }
}

fn render_board_mode(
    app: &mut AgendaApp,
    ui: &mut egui::Ui,
    p: Palette,
    week_start: Date,
    avail_x: f32,
    board_h: f32,
) {
    let col_gap = 8.0_f32;
    let col_w = ((avail_x - col_gap * 6.0) / 7.0).floor().max(80.0);

    let mut toggle_target = None;
    let mut open_modal_for = None;
    let mut open_daily_for = None;

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = col_gap;
        for (i, wd_label) in WEEKDAY_SHORT.iter().enumerate() {
            let day = week_start.add_days(i as i64);
            let is_today = day == app.today;
            let is_selected = day == app.selected_date;

            let occurrences: Vec<_> = app
                .store
                .tasks_visible_on(day)
                .into_iter()
                .filter(|occ| {
                    views::task_matches_filters(
                        &occ.task,
                        app.search_query.trim(),
                        app.category_filter.as_deref(),
                    )
                })
                .collect();

            ui.allocate_ui_with_layout(
                Vec2::new(col_w, board_h),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::Frame::none()
                        .fill(p.bg_surface)
                        .stroke(Stroke::new(
                            if is_today || is_selected {
                                2.0_f32
                            } else {
                                1.0_f32
                            },
                            if is_today || is_selected {
                                p.accent
                            } else {
                                p.border_subtle
                            },
                        ))
                        .rounding(Rounding::same(Radius::LG))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.set_width((col_w - 22.0).max(60.0));
                            ui.set_min_height((board_h - 22.0).max(160.0));

                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(*wd_label)
                                            .size(11.0)
                                            .strong()
                                            .color(if is_today { p.accent } else { p.ink_muted }),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if is_today {
                                            views::badge(
                                                ui,
                                                &day.day.to_string(),
                                                p.on_accent,
                                                p.accent,
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(day.day.to_string())
                                                    .size(14.5)
                                                    .monospace()
                                                    .strong()
                                                    .color(p.ink_primary),
                                            );
                                        }
                                    });
                                });

                                ui.horizontal(|ui| {
                                    let cnt_lbl = if occurrences.len() == 1 {
                                        "1 tarea".to_owned()
                                    } else {
                                        format!("{} tareas", occurrences.len())
                                    };
                                    ui.label(
                                        RichText::new(cnt_lbl)
                                            .size(10.5)
                                            .monospace()
                                            .color(p.ink_muted),
                                    );
                                    if is_today {
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                views::badge(ui, "HOY", p.accent, p.accent_soft);
                                            },
                                        );
                                    }
                                });

                                ui.add_space(6.0);

                                let list_h = (board_h - 118.0).max(90.0);
                                egui::ScrollArea::vertical()
                                    .id_salt(("week_col", day))
                                    .max_height(list_h)
                                    .show(ui, |ui| {
                                        for occ in &occurrences {
                                            let task = &occ.task;
                                            let (fg, bg) = views::task_colors(
                                                task.color,
                                                &task.tag,
                                                task.priority,
                                                p,
                                            );
                                            egui::Frame::none()
                                                .fill(if task.done {
                                                    p.done_bg
                                                } else {
                                                    p.bg_canvas
                                                })
                                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                                .rounding(Rounding::same(Radius::MD))
                                                .inner_margin(8.0)
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            if !task.time.is_empty() {
                                                                let t_lbl =
                                                                    if task.end_time.is_empty() {
                                                                        task.time.clone()
                                                                    } else {
                                                                        format!(
                                                                            "{}-{}",
                                                                            task.time,
                                                                            task.end_time
                                                                        )
                                                                    };
                                                                ui.label(
                                                                    RichText::new(t_lbl)
                                                                        .size(10.0)
                                                                        .monospace()
                                                                        .color(p.ink_muted),
                                                                );
                                                            }
                                                            if !task.tag.is_empty() {
                                                                ui.with_layout(
                                                                    Layout::right_to_left(
                                                                        Align::Center,
                                                                    ),
                                                                    |ui| {
                                                                        views::badge(
                                                                            ui, &task.tag, fg, bg,
                                                                        );
                                                                    },
                                                                );
                                                            }
                                                        });

                                                        ui.add_space(2.0);
                                                        let mut title_rt =
                                                            RichText::new(&task.title)
                                                                .size(12.0)
                                                                .strong();
                                                        if task.done {
                                                            title_rt = title_rt
                                                                .strikethrough()
                                                                .color(p.ink_muted);
                                                        } else {
                                                            title_rt =
                                                                title_rt.color(p.ink_primary);
                                                        }
                                                        ui.add(egui::Label::new(title_rt).wrap());

                                                        ui.horizontal(|ui| {
                                                            if !task.location.is_empty() {
                                                                ui.label(
                                                                    RichText::new(&task.location)
                                                                        .size(10.0)
                                                                        .color(p.ink_muted),
                                                                );
                                                            }
                                                            ui.with_layout(
                                                                Layout::right_to_left(
                                                                    Align::Center,
                                                                ),
                                                                |ui| {
                                                                    let mut checked = task.done;
                                                                    if ui
                                                                        .checkbox(&mut checked, "")
                                                                        .changed()
                                                                    {
                                                                        toggle_target = Some((
                                                                            occ.start_date,
                                                                            task.id,
                                                                        ));
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    });
                                                });
                                            ui.add_space(5.0);
                                        }
                                    });

                                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                                    ui.horizontal(|ui| {
                                        let add_w = (ui.available_width() - 32.0).max(24.0);
                                        let add_label =
                                            if col_w < 115.0 { "+" } else { "+ Añadir" };
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(add_label)
                                                        .size(11.0)
                                                        .color(p.ink_secondary),
                                                )
                                                .fill(p.bg_elevated)
                                                .min_size(Vec2::new(add_w, 26.0)),
                                            )
                                            .clicked()
                                        {
                                            open_modal_for = Some(day);
                                        }
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(">").size(12.0).color(p.accent),
                                                )
                                                .fill(p.accent_soft)
                                                .min_size(Vec2::new(26.0, 26.0)),
                                            )
                                            .on_hover_text("Abrir Agenda Diaria")
                                            .clicked()
                                        {
                                            open_daily_for = Some(day);
                                        }
                                    });
                                });
                            });
                        });
                },
            );
        }
    });

    if let Some((sd, id)) = toggle_target {
        if app.store.toggle_task(sd, id) {
            app.persist();
        }
    }
    if let Some(day) = open_modal_for {
        app.select_date(day);
        app.show_new_task_modal = true;
    }
    if let Some(day) = open_daily_for {
        app.select_date(day);
        app.active_view = ActiveView::Daily;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_analytical_footer(
    app: &mut AgendaApp,
    ui: &mut egui::Ui,
    p: Palette,
    week_start: Date,
    week_num: u32,
    week_total: usize,
    week_done: usize,
    avail_x: f32,
    bottom_h: f32,
) {
    let card_gap = 12.0_f32;
    let card_w = ((avail_x - card_gap * 2.0) / 3.0).floor().max(200.0);
    let (total_mins, cat_stats) = app.store.week_category_stats(week_start);
    let total_hours = total_mins as f32 / 60.0;

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = card_gap;

        // Tarjeta 1: Distribución por Categorías (en horas reales y conteo)
        ui.allocate_ui_with_layout(
            Vec2::new(card_w, bottom_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(Radius::LG))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width((card_w - 30.0).max(160.0));
                        ui.set_min_height((bottom_h - 30.0).max(110.0));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Distribución por Categorías ({total_hours:.1}h)"
                                ))
                                .size(13.5)
                                .monospace()
                                .strong()
                                .color(p.ink_primary),
                            );
                            ui.add_space(6.0);
                            for cat in TAG_PRESETS {
                                let (cnt, mins) = cat_stats.get(cat).copied().unwrap_or((0, 0));
                                let pct = (mins * 100)
                                    .checked_div(total_mins)
                                    .unwrap_or(0);
                                let hrs = mins as f32 / 60.0;
                                let fg = p.category_token(cat).solid;
                                ui.horizontal(|ui| {
                                    let (dot_r, _) =
                                        ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                    ui.painter().circle_filled(dot_r.center(), 4.0, fg);
                                    ui.label(RichText::new(cat).size(12.0).color(p.ink_primary));

                                    let bar_w = (ui.available_width() - 90.0).max(20.0);
                                    let (bar_r, _) = ui.allocate_exact_size(
                                        Vec2::new(bar_w, 5.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        bar_r,
                                        Rounding::same(Radius::SM / 2.0),
                                        p.bg_elevated,
                                    );
                                    if pct > 0 {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                bar_r.min,
                                                Vec2::new(bar_w * (pct as f32 / 100.0), 5.0),
                                            ),
                                            Rounding::same(Radius::SM / 2.0),
                                            fg,
                                        );
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!("{hrs:.1}h ({cnt}) · {pct}%"))
                                                .size(10.5)
                                                .monospace()
                                                .strong()
                                                .color(p.ink_secondary),
                                        );
                                    });
                                });
                            }
                        });
                    });
            },
        );

        // Tarjeta 2: Intención de la Semana (autoguardado con debounce)
        ui.allocate_ui_with_layout(
            Vec2::new(card_w, bottom_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(Radius::LG))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width((card_w - 30.0).max(160.0));
                        ui.set_min_height((bottom_h - 30.0).max(110.0));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("INTENCIÓN DE LA SEMANA")
                                    .size(11.0)
                                    .strong()
                                    .color(p.accent),
                            );
                            ui.add_space(6.0);
                            let w_key = week_start.to_key();
                            let intention = app.store.weekly_intentions.entry(w_key).or_default();
                            let resp = ui.add(
                                egui::TextEdit::multiline(intention)
                                    .hint_text("Priorizar la profundidad en la ejecución...")
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            );
                            if resp.changed() {
                                app.mark_notes_dirty();
                            }
                        });
                    });
            },
        );

        // Tarjeta 3: Balance de Carga
        ui.allocate_ui_with_layout(
            Vec2::new(card_w, bottom_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(Radius::LG))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width((card_w - 30.0).max(160.0));
                        ui.set_min_height((bottom_h - 30.0).max(110.0));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("BALANCE DE CARGA")
                                    .size(11.0)
                                    .strong()
                                    .color(p.ink_muted),
                            );
                            let balance_title = if total_hours <= 20.0 {
                                "Sostenible & Fluida"
                            } else if total_hours <= 36.0 {
                                "Ritmo Equilibrado"
                            } else {
                                "Semana Intensa"
                            };
                            ui.label(
                                RichText::new(balance_title)
                                    .size(15.5)
                                    .strong()
                                    .color(p.ink_primary),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(format!(
                                    "{week_total} bloques · {total_hours:.1}h ({week_done} completados)"
                                ))
                                .size(11.5)
                                .monospace()
                                .color(p.ink_secondary),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let half_w = ((ui.available_width() - 8.0) / 2.0).max(80.0);
                                ui.allocate_ui_with_layout(
                                    Vec2::new(half_w, 30.0),
                                    Layout::left_to_right(Align::Center),
                                    |ui| {
                                        ui.menu_button(
                                            RichText::new("Exportar v")
                                                .size(11.5)
                                                .color(p.ink_primary),
                                            |ui| {
                                                if ui.button("Resumen Markdown (.md)").clicked() {
                                                    let md = crate::export::export_week_markdown(
                                                        &app.store,
                                                        week_start,
                                                        week_num,
                                                    );
                                                    app.export_to_desktop(
                                                        &format!("Semana_{week_num}_Resumen.md"),
                                                        &md,
                                                    );
                                                    ui.close_menu();
                                                }
                                                if ui.button("Calendario iCal (.ics)").clicked() {
                                                    let ics = crate::export::export_ics(&app.store);
                                                    app.export_to_desktop("Mi_Agenda.ics", &ics);
                                                    ui.close_menu();
                                                }
                                                if ui.button("Texto plano (.txt)").clicked() {
                                                    export_week_summary(app, week_start, week_num);
                                                    ui.close_menu();
                                                }
                                            },
                                        );
                                    },
                                );
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Agenda de Hoy")
                                                .size(11.5)
                                                .strong()
                                                .color(p.on_accent),
                                        )
                                        .fill(p.accent)
                                        .min_size(Vec2::new(half_w, 30.0)),
                                    )
                                    .clicked()
                                {
                                    app.select_date(app.today);
                                    app.active_view = ActiveView::Daily;
                                }
                            });
                        });
                    });
            },
        );
    });
}

fn export_week_summary(app: &mut AgendaApp, week_start: Date, week_num: u32) {
    let mut text = format!("Mi Agenda - Resumen Semana {week_num}\n\n");
    for offset in 0..7 {
        let day = week_start.add_days(offset);
        let occs = app.store.tasks_visible_on(day);
        text.push_str(&format!(
            "## {} {} {}\n",
            WEEKDAY_SHORT[offset as usize],
            day.day,
            month_short_es(day.month)
        ));
        if occs.is_empty() {
            text.push_str("  (Sin tareas)\n");
        } else {
            for occ in occs {
                let mark = if occ.task.done { "[x]" } else { "[ ]" };
                text.push_str(&format!("  {mark} {}\n", occ.task.title));
            }
        }
        text.push('\n');
    }
    let path = std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Desktop")
        .join(format!("Semana_{week_num}_Resumen.txt"));
    if let Err(err) = std::fs::write(&path, text) {
        app.status_error = Some(format!("No se pudo exportar resumen: {err}"));
    } else {
        app.status_error = None;
    }
}
