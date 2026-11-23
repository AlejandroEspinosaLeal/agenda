use eframe::egui::{
    self, Align, Color32, FontId, Key, Layout, RichText, Rounding, Sense, Stroke, Vec2,
};

use crate::app::{AgendaApp, GridCreateDrag};
use crate::calendar::{month_name_es, month_short_es, weekday_name_es, Date};
use crate::model::Priority;
use crate::theme::{Palette, Radius, Spacing};
use crate::timeblock::{self, TimeInterval};
use crate::views::{self, COLOR_PRESETS, TAG_PRESETS};

const HOUR_HEIGHT: f32 = 52.0;

pub fn render(app: &mut AgendaApp, ui: &mut egui::Ui, p: Palette) {
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

    let now_min = timeblock::current_local_minutes();
    let (now_h, now_m) = (now_min / 60, now_min % 60);

    // Cabecera principal del día
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{wd_name}, {} de {}",
                        date.day,
                        month_name_es(date.month)
                    ))
                    .size(26.0)
                    .strong()
                    .color(p.ink_primary),
                );
                if date == app.today {
                    views::badge(ui, "HOY", p.accent, p.accent_soft);
                }
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Año {} · Semana {} · {} bloques programados",
                        date.year,
                        date.iso_week(),
                        occurrences.len()
                    ))
                    .size(12.5)
                    .monospace()
                    .color(p.ink_muted),
                );
                if date == app.today {
                    views::badge(
                        ui,
                        &format!("{now_h:02}:{now_m:02} Actual"),
                        p.ink_secondary,
                        p.bg_surface,
                    );
                }
            });
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("›").size(15.0).strong().color(p.ink_primary))
                        .fill(p.bg_surface)
                        .min_size(Vec2::new(30.0, 32.0)),
                )
                .on_hover_text("Día siguiente (→)")
                .clicked()
            {
                app.select_date(date.add_days(1));
            }
            ui.label(
                RichText::new(format!(
                    "{} {} {}",
                    date.day,
                    month_short_es(date.month),
                    date.year
                ))
                .size(12.5)
                .monospace()
                .strong()
                .color(p.ink_primary),
            );
            if ui
                .add(
                    egui::Button::new(RichText::new("‹").size(15.0).strong().color(p.ink_primary))
                        .fill(p.bg_surface)
                        .min_size(Vec2::new(30.0, 32.0)),
                )
                .on_hover_text("Día anterior (←)")
                .clicked()
            {
                app.select_date(date.add_days(-1));
            }
        });
    });

    ui.add_space(Spacing::MD);

    let avail = ui.available_size();
    let right_w = if app.focus_mode { 0.0 } else { 340.0 };
    let left_w = (avail.x - if app.focus_mode { 0.0 } else { right_w + Spacing::LG }).max(420.0);

    ui.horizontal_top(|ui| {
        // Columna izquierda / central: Grid Horario Real + Ritmo del día
        ui.allocate_ui_with_layout(
            Vec2::new(left_w, avail.y),
            Layout::top_down(Align::Min),
            |ui| {
                let rhythm_h = 72.0_f32;
                let timeline_h = (avail.y - rhythm_h - Spacing::MD).max(280.0);

                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(Radius::LG))
                    .inner_margin(Spacing::LG)
                    .show(ui, |ui| {
                        ui.set_width((left_w - 34.0).max(360.0));
                        ui.set_min_height((timeline_h - 34.0).max(220.0));

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Cronograma del Día")
                                        .size(16.0)
                                        .strong()
                                        .color(p.ink_primary),
                                );
                                ui.add_space(Spacing::SM);
                                let range_lbl = if app.daily_24h { "24h" } else { "06–23h" };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(range_lbl)
                                                .size(11.0)
                                                .monospace()
                                                .color(p.accent),
                                        )
                                        .fill(p.accent_soft)
                                        .rounding(Rounding::same(Radius::SM)),
                                    )
                                    .on_hover_text("Alternar entre 06:00–23:00 y 24h")
                                    .clicked()
                                {
                                    app.daily_24h = !app.daily_24h;
                                }

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    views::category_filter_pills(ui, &mut app.category_filter, p);
                                });
                            });

                            ui.add_space(Spacing::SM);

                            // Franja superior: Bloques "Todo el día"
                            let all_day_items: Vec<_> = occurrences
                                .iter()
                                .filter(|o| o.task.time_bounds_minutes().is_none())
                                .cloned()
                                .collect();
                            let timed_items: Vec<_> = occurrences
                                .iter()
                                .filter(|o| o.task.time_bounds_minutes().is_some())
                                .cloned()
                                .collect();

                            let mut toggle_target = None;
                            let mut delete_target = None;
                            let mut persist_needed = false;
                            let mut open_draft: Option<(Date, u32, u32)> = None;

                            if !all_day_items.is_empty() {
                                egui::Frame::none()
                                    .fill(p.bg_canvas)
                                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                    .rounding(Rounding::same(Radius::MD))
                                    .inner_margin(Spacing::SM)
                                    .show(ui, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                RichText::new("TODO EL DÍA")
                                                    .size(10.0)
                                                    .strong()
                                                    .color(p.ink_muted),
                                            );
                                            for occ in &all_day_items {
                                                let task = &occ.task;
                                                let (fg, bg) = views::task_colors(
                                                    task.color,
                                                    &task.tag,
                                                    task.priority,
                                                    p,
                                                );
                                                egui::Frame::none()
                                                    .fill(if task.done { p.done_bg } else { bg })
                                                    .rounding(Rounding::same(Radius::SM))
                                                    .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                                                    .show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            let mut c = task.done;
                                                            if ui.checkbox(&mut c, "").changed() {
                                                                toggle_target =
                                                                    Some((occ.start_date, task.id));
                                                            }
                                                            let mut rt = RichText::new(&task.title)
                                                                .size(11.5)
                                                                .strong();
                                                            rt = if task.done {
                                                                rt.strikethrough().color(p.ink_muted)
                                                            } else {
                                                                rt.color(fg)
                                                            };
                                                            ui.label(rt);
                                                            if ui
                                                                .add(
                                                                    egui::Button::new(
                                                                        RichText::new("×")
                                                                            .size(12.0)
                                                                            .color(p.ink_muted),
                                                                    )
                                                                    .frame(false),
                                                                )
                                                                .clicked()
                                                            {
                                                                delete_target =
                                                                    Some((occ.start_date, task.id));
                                                            }
                                                        });
                                                    });
                                            }
                                        });
                                    });
                                ui.add_space(Spacing::SM);
                            }

                            // Cuadrícula horaria real (06:00..23:00 o 00:00..24:00)
                            let start_hour: u32 = if app.daily_24h { 0 } else { 6 };
                            let end_hour: u32 = if app.daily_24h { 24 } else { 23 };
                            let total_hours = end_hour - start_hour;
                            let grid_h = total_hours as f32 * HOUR_HEIGHT;
                            let axis_w = 54.0_f32;
                            let px_per_min = HOUR_HEIGHT / 60.0;

                            let intervals: Vec<TimeInterval> = timed_items
                                .iter()
                                .filter_map(|o| {
                                    o.task.time_bounds_minutes().map(|(s, e)| TimeInterval {
                                        id: o.task.id,
                                        start_min: s,
                                        end_min: e,
                                    })
                                })
                                .collect();
                            let placed = timeblock::layout_day_blocks(intervals);

                            let scroll_h = (timeline_h - 110.0).max(180.0);
                            egui::ScrollArea::vertical()
                                .id_salt("daily_time_grid")
                                .max_height(scroll_h)
                                .show(ui, |ui| {
                                    let avail_w = ui.available_width();
                                    let (canvas_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(avail_w, grid_h),
                                        Sense::hover(),
                                    );
                                    let painter = ui.painter();
                                    let track_x = canvas_rect.min.x + axis_w;
                                    let track_w = (canvas_rect.width() - axis_w - 6.0).max(120.0);
                                    let track_rect = egui::Rect::from_min_size(
                                        egui::pos2(track_x, canvas_rect.min.y),
                                        Vec2::new(track_w, grid_h),
                                    );
                                    let min_visible = start_hour * 60;
                                    let max_visible = end_hour * 60;

                                    let y_to_snap_min = |y: f32| -> u32 {
                                        let rel = ((y - canvas_rect.min.y) / px_per_min).max(0.0);
                                        let raw = min_visible + rel.round() as u32;
                                        timeblock::snap_to_15(raw).clamp(min_visible, max_visible)
                                    };

                                    // Drop desde Backlog de la barra lateral
                                    if let Some(backlog_id) = app.drag_backlog_id {
                                        if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                            if track_rect.contains(ptr) {
                                                let s_min = y_to_snap_min(ptr.y).min(1380);
                                                let preview_y = canvas_rect.min.y
                                                    + (s_min - min_visible) as f32 * px_per_min;
                                                let preview_r = egui::Rect::from_min_size(
                                                    egui::pos2(track_x + 2.0, preview_y),
                                                    Vec2::new(track_w - 4.0, HOUR_HEIGHT),
                                                );
                                                painter.rect_filled(
                                                    preview_r,
                                                    Rounding::same(Radius::MD),
                                                    p.accent_soft,
                                                );
                                                painter.rect_stroke(
                                                    preview_r,
                                                    Rounding::same(Radius::MD),
                                                    Stroke::new(1.5_f32, p.accent),
                                                );
                                                if !ui.input(|i| i.pointer.primary_down()) {
                                                    if app.store.update_item_schedule(
                                                        backlog_id,
                                                        Some(date),
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

                                    // Interacción directa sobre franja vacía (clic o arrastre)
                                    let bg_resp = ui.interact(
                                        track_rect,
                                        ui.id().with(("daily_bg_create", date)),
                                        Sense::click_and_drag(),
                                    );
                                    if bg_resp.drag_started() {
                                        if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                            let m = y_to_snap_min(ptr.y);
                                            app.grid_create_drag = Some(GridCreateDrag {
                                                day: date,
                                                anchor_min: m,
                                                current_min: (m + 60).min(max_visible),
                                            });
                                        }
                                    }
                                    if bg_resp.dragged() {
                                        if let (Some(drag), Some(ptr)) =
                                            (&mut app.grid_create_drag, ui.ctx().pointer_interact_pos())
                                        {
                                            drag.current_min = y_to_snap_min(ptr.y);
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
                                            let s = y_to_snap_min(ptr.y).min(max_visible - 30);
                                            open_draft = Some((date, s, (s + 60).min(max_visible)));
                                        }
                                    }

                                    // Líneas horizontales de cada hora
                                    for h in start_hour..=end_hour {
                                        let y = canvas_rect.min.y
                                            + (h - start_hour) as f32 * HOUR_HEIGHT;
                                        painter.text(
                                            egui::pos2(canvas_rect.min.x + 4.0, y),
                                            egui::Align2::LEFT_CENTER,
                                            format!("{h:02}:00"),
                                            FontId::monospace(10.5),
                                            p.ink_muted,
                                        );
                                        painter.line_segment(
                                            [
                                                egui::pos2(track_x, y),
                                                egui::pos2(track_x + track_w, y),
                                            ],
                                            Stroke::new(1.0_f32, p.border_subtle),
                                        );
                                    }

                                    // Preview de creación directa por arrastre
                                    if let Some(drag) = app.grid_create_drag {
                                        if drag.day == date {
                                            let s = drag.anchor_min.min(drag.current_min);
                                            let e = drag.anchor_min.max(drag.current_min).max(s + 15);
                                            let top = canvas_rect.min.y
                                                + (s - min_visible) as f32 * px_per_min;
                                            let h = ((e - s) as f32 * px_per_min).max(16.0);
                                            let pr = egui::Rect::from_min_size(
                                                egui::pos2(track_x + 2.0, top),
                                                Vec2::new(track_w - 4.0, h),
                                            );
                                            painter.rect_filled(
                                                pr,
                                                Rounding::same(Radius::MD),
                                                p.accent_soft,
                                            );
                                            painter.rect_stroke(
                                                pr,
                                                Rounding::same(Radius::MD),
                                                Stroke::new(1.5_f32, p.accent),
                                            );
                                            painter.text(
                                                pr.min + Vec2::new(8.0, 4.0),
                                                egui::Align2::LEFT_TOP,
                                                format!(
                                                    "{:02}:{:02}–{:02}:{:02}",
                                                    s / 60,
                                                    s % 60,
                                                    e / 60,
                                                    e % 60
                                                ),
                                                FontId::monospace(11.0),
                                                p.accent,
                                            );
                                        }
                                    }

                                    // Bloques horarios particionados en columnas paralelas + Drag & Resize
                                    let mut move_update: Option<(u64, u32, u32)> = None;
                                    for blk in &placed {
                                        let Some(occ) =
                                            timed_items.iter().find(|o| o.task.id == blk.id)
                                        else {
                                            continue;
                                        };
                                        if blk.end_min <= min_visible
                                            || blk.start_min >= max_visible
                                        {
                                            continue;
                                        }
                                        let s = blk.start_min.max(min_visible);
                                        let e = blk.end_min.min(max_visible);
                                        let top_y = canvas_rect.min.y
                                            + (s - min_visible) as f32 * px_per_min;
                                        let height =
                                            ((e - s) as f32 * px_per_min).max(26.0);

                                        let col_w = track_w / blk.total_cols as f32;
                                        let x = track_x + blk.col as f32 * col_w + 2.0;
                                        let w = (col_w - 4.0).max(40.0);
                                        let block_rect = egui::Rect::from_min_size(
                                            egui::pos2(x, top_y),
                                            Vec2::new(w, height),
                                        );

                                        let task = &occ.task;
                                        let (fg, bg) = views::task_colors(
                                            task.color,
                                            &task.tag,
                                            task.priority,
                                            p,
                                        );
                                        let fill = if task.done { p.done_bg } else { bg };
                                        painter.rect_filled(
                                            block_rect,
                                            Rounding::same(Radius::MD),
                                            fill,
                                        );
                                        painter.rect_stroke(
                                            block_rect,
                                            Rounding::same(Radius::MD),
                                            Stroke::new(1.0_f32, fg.gamma_multiply(0.45)),
                                        );
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                block_rect.min,
                                                Vec2::new(4.0, height),
                                            ),
                                            Rounding::same(2.0),
                                            if task.done { p.done_green } else { fg },
                                        );

                                        // Cuerpo arrastrable y clicable
                                        let body_rect = egui::Rect::from_min_max(
                                            block_rect.min,
                                            egui::pos2(block_rect.max.x, (block_rect.max.y - 6.0).max(block_rect.min.y + 14.0)),
                                        );
                                        let resp = ui.interact(
                                            body_rect,
                                            ui.id().with(("tb", task.id)),
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
                                                move_update = Some((task.id, new_s, new_s + dur));
                                            }
                                        }
                                        if resp.drag_stopped() {
                                            persist_needed = true;
                                        } else if resp.clicked() {
                                            toggle_target = Some((occ.start_date, task.id));
                                        }

                                        // Manejador inferior de resize (incrementos de 15 min)
                                        let resize_rect = egui::Rect::from_min_max(
                                            egui::pos2(block_rect.min.x, body_rect.max.y),
                                            block_rect.max,
                                        );
                                        let res_resp = ui.interact(
                                            resize_rect,
                                            ui.id().with(("tb_res", task.id)),
                                            Sense::drag(),
                                        );
                                        if res_resp.hovered() || res_resp.dragged() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                                        }
                                        if res_resp.dragged() {
                                            if let Some(ptr) = ui.ctx().pointer_interact_pos() {
                                                let new_e = y_to_snap_min(ptr.y).max(blk.start_min + 15);
                                                move_update = Some((task.id, blk.start_min, new_e));
                                            }
                                        }
                                        if res_resp.drag_stopped() {
                                            persist_needed = true;
                                        }

                                        let range_txt = if task.end_time.is_empty() {
                                            task.time.clone()
                                        } else {
                                            format!("{}–{}", task.time, task.end_time)
                                        };
                                        let _ = resp.on_hover_text(format!(
                                            "{}\n{} · Arrastra para mover · Clic para completar",
                                            task.title, range_txt
                                        ));

                                        let title_color =
                                            if task.done { p.ink_muted } else { p.ink_primary };
                                        let max_chars =
                                            ((w - 18.0) / 6.5).floor().max(5.0) as usize;
                                        painter.text(
                                            block_rect.min + Vec2::new(10.0, 5.0),
                                            egui::Align2::LEFT_TOP,
                                            views::truncate_chars(&task.title, max_chars),
                                            FontId::proportional(12.5),
                                            title_color,
                                        );
                                        if height >= 38.0 {
                                            painter.text(
                                                block_rect.min + Vec2::new(10.0, 22.0),
                                                egui::Align2::LEFT_TOP,
                                                range_txt,
                                                FontId::monospace(10.5),
                                                fg,
                                            );
                                        }
                                    }

                                    if let Some((id, ns, ne)) = move_update {
                                        app.store.update_item_schedule(id, None, ns, ne);
                                    }

                                    // Línea horizontal en vivo de la hora actual
                                    if date == app.today
                                        && now_min >= min_visible
                                        && now_min <= max_visible
                                    {
                                        let y = canvas_rect.min.y
                                            + (now_min - min_visible) as f32 * px_per_min;
                                        let pill = egui::Rect::from_min_size(
                                            egui::pos2(canvas_rect.min.x + 2.0, y - 9.0),
                                            Vec2::new(46.0, 18.0),
                                        );
                                        painter.rect_filled(
                                            pill,
                                            Rounding::same(Radius::SM),
                                            p.priority_high,
                                        );
                                        painter.text(
                                            pill.center(),
                                            egui::Align2::CENTER_CENTER,
                                            format!("{now_h:02}:{now_m:02}"),
                                            FontId::monospace(10.0),
                                            Color32::WHITE,
                                        );
                                        painter.line_segment(
                                            [
                                                egui::pos2(track_x, y),
                                                egui::pos2(track_x + track_w, y),
                                            ],
                                            Stroke::new(2.0_f32, p.priority_high),
                                        );
                                    }
                                });

                            if let Some((sd, id)) = toggle_target {
                                persist_needed |= app.store.toggle_task(sd, id);
                            }
                            if let Some((sd, id)) = delete_target {
                                persist_needed |= app.store.delete_task(sd, id);
                            }
                            if persist_needed {
                                app.persist();
                            }
                            if let Some((d, s, e)) = open_draft {
                                app.open_timed_draft(d, s, e);
                            }
                        });
                    });

                ui.add_space(Spacing::SM);

                // Tarjeta inferior de bienestar calculada con horas reales ocupadas
                let occupied_mins: u32 = occurrences
                    .iter()
                    .filter_map(|o| o.task.time_bounds_minutes().map(|(s, e)| e - s))
                    .sum();
                let occupied_hours = occupied_mins as f32 / 60.0;
                let (rhythm_title, rhythm_desc) = if occupied_mins == 0 {
                    (
                        "Ritmo del día: Despejado (0.0h / 8.0h)".to_owned(),
                        "Jornada libre para trabajo profundo o descanso.",
                    )
                } else if occupied_hours <= 5.5 {
                    (
                        format!("Ritmo del día: Equilibrado ({occupied_hours:.1}h / 8.0h)"),
                        "Has reservado margen saludable entre bloques.",
                    )
                } else {
                    (
                        format!("Ritmo del día: Intenso ({occupied_hours:.1}h / 8.0h)"),
                        "Alta carga horaria: procura incluir pausas breves.",
                    )
                };

                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(Radius::LG))
                    .inner_margin(Spacing::MD)
                    .show(ui, |ui| {
                        ui.set_width((left_w - 26.0).max(360.0));
                        ui.horizontal(|ui| {
                            let tok = p.category_token("Salud");
                            views::badge(ui, "BIENESTAR", tok.text, tok.bg);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(rhythm_title)
                                        .size(13.0)
                                        .monospace()
                                        .strong()
                                        .color(p.ink_primary),
                                );
                                ui.label(
                                    RichText::new(rhythm_desc)
                                        .size(11.5)
                                        .color(p.ink_secondary),
                                );
                            });
                        });
                    });
            },
        );

        // Columna derecha: Nueva Tarea para Hoy + Notas Rápidas del Día
        if !app.focus_mode {
            ui.add_space(Spacing::LG);
            ui.allocate_ui_with_layout(
                Vec2::new(right_w, avail.y),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        render_daily_right_column(app, ui, p, date);
                    });
                },
            );
        }
    });
}

fn render_daily_right_column(app: &mut AgendaApp, ui: &mut egui::Ui, p: Palette, date: Date) {
    // Tarjeta 1: Nueva Tarea para Hoy
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(Radius::LG))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Nueva Tarea para Hoy")
                            .size(15.0)
                            .strong()
                            .color(p.ink_primary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                date.day,
                                month_short_es(date.month).to_uppercase()
                            ))
                            .size(11.0)
                            .monospace()
                            .color(p.ink_muted),
                        );
                    });
                });

                ui.add_space(6.0);
                let title_resp = ui.add(
                    egui::TextEdit::singleline(&mut app.new_title)
                        .hint_text("ej. Comprar billetes de tren")
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_notes)
                        .hint_text("Nota corta (opcional)")
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(6.0);
                views::schedule_and_recurrence_controls(
                    ui,
                    &mut app.new_all_day,
                    &mut app.new_time,
                    &mut app.new_end_time,
                    &mut app.new_recurrence,
                    p,
                );

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for cat in TAG_PRESETS {
                        let sel = app.new_tag == cat;
                        let tok = p.category_token(cat);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(cat)
                                        .size(11.0)
                                        .strong()
                                        .color(if sel { Color32::WHITE } else { tok.text }),
                                )
                                .fill(if sel { tok.solid } else { tok.bg })
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if sel { tok.solid } else { tok.border },
                                )),
                            )
                            .clicked()
                        {
                            app.new_tag = if sel { String::new() } else { cat.to_owned() };
                        }
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for prio in [Priority::Low, Priority::Normal, Priority::High] {
                        let sel = app.new_priority == prio;
                        let (fg, bg) = views::priority_colors(prio, p);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(prio.label_es())
                                        .size(11.0)
                                        .color(if sel { fg } else { p.ink_secondary }),
                                )
                                .fill(if sel { bg } else { p.bg_elevated }),
                            )
                            .clicked()
                        {
                            app.new_priority = prio;
                        }
                    }
                    for preset in COLOR_PRESETS.iter().take(4) {
                        let c = Color32::from_rgb(preset[0], preset[1], preset[2]);
                        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::click());
                        ui.painter().circle_filled(rect.center(), 6.0, c);
                        if app.new_color == Some(*preset) {
                            ui.painter().circle_stroke(
                                rect.center(),
                                7.5,
                                Stroke::new(1.5_f32, p.ink_primary),
                            );
                        }
                        if resp.clicked() {
                            app.new_color = Some(*preset);
                        }
                    }
                });

                ui.add_space(8.0);
                let submit = (title_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    || ui
                        .add(
                            egui::Button::new(
                                RichText::new("+ Guardar en mi Agenda")
                                    .size(13.0)
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
        });

    ui.add_space(12.0);

    // Tarjeta 2: Notas Rápidas del Día (autoguardado con debounce de 600ms)
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(Radius::LG))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Notas Rápidas del Día")
                        .size(14.5)
                        .strong()
                        .color(p.ink_primary),
                );
                ui.add_space(6.0);
                let key = date.to_key();
                let note = app.store.daily_notes.entry(key).or_default();
                let resp = ui.add(
                    egui::TextEdit::multiline(note)
                        .hint_text("Escribe notas rápidas para este día...")
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
                if resp.changed() {
                    app.mark_notes_dirty();
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let status_txt = if app.notes_dirty_at.is_some() {
                        "Guardando..."
                    } else {
                        "Sincronizado"
                    };
                    ui.label(
                        RichText::new("Guardado automático")
                            .size(10.5)
                            .color(p.ink_muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(status_txt)
                                .size(10.5)
                                .color(if app.notes_dirty_at.is_some() {
                                    p.accent
                                } else {
                                    p.done_green
                                }),
                        );
                    });
                });
            });
        });
}
