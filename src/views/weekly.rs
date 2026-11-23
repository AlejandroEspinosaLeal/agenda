use eframe::egui::{self, Align, Color32, Layout, RichText, Rounding, Sense, Stroke, Vec2};

use crate::app::{ActiveView, AgendaApp};
use crate::calendar::{month_name_es, month_short_es, Date};
use crate::theme::{category_colors, Palette};
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

    // Cabecera del planificador semanal
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            views::badge(
                ui,
                &format!("PLANIFICADOR ACTIVO  ·  Ciclo {}", week_start.year),
                p.accent,
                p.accent_soft,
            );
            ui.add_space(4.0);
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

    ui.add_space(8.0);

    // Filtros por categoría + métricas clave a la derecha
    ui.horizontal(|ui| {
        views::category_filter_pills(ui, &mut app.category_filter, p, app.dark_mode);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            views::badge(
                ui,
                &format!("LIBRE: {free_pct}% Estimado"),
                p.accent,
                p.accent_soft,
            );
            views::badge(
                ui,
                &format!("{week_done} Completadas"),
                p.done_green,
                p.done_bg,
            );
            views::badge(
                ui,
                &format!("{week_total} Tareas programadas"),
                p.ink_primary,
                p.bg_surface,
            );
        });
    });

    ui.add_space(10.0);

    let avail = ui.available_size();
    let bottom_h = 174.0_f32;
    let board_h = (avail.y - bottom_h - 18.0).clamp(240.0, 440.0);
    let col_gap = 8.0_f32;
    let col_w = ((avail.x - col_gap * 6.0) / 7.0).floor().max(80.0);

    let mut toggle_target = None;
    let mut open_modal_for = None;
    let mut open_daily_for = None;

    // Tablero central de 7 columnas (Lunes a Domingo, todas visibles sin desbordamiento)
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
                            if is_today || is_selected { 2.0_f32 } else { 1.0_f32 },
                            if is_today || is_selected { p.accent } else { p.border_subtle },
                        ))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.set_width((col_w - 22.0).max(60.0));
                            ui.set_min_height((board_h - 22.0).max(160.0));

                            ui.vertical(|ui| {
                                // Cabecera de columna en 2 filas limpias (cero solapamientos)
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
                                                Color32::WHITE,
                                                p.accent,
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(day.day.to_string())
                                                    .size(14.5)
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
                                    ui.label(RichText::new(cnt_lbl).size(10.5).color(p.ink_muted));
                                    if is_today {
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            views::badge(ui, "HOY", p.accent, p.accent_soft);
                                        });
                                    }
                                });

                                ui.add_space(6.0);

                                // Tarjetas de tareas dentro de la columna
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
                                                app.dark_mode,
                                            );
                                            egui::Frame::none()
                                                .fill(if task.done { p.done_bg } else { p.bg_canvas })
                                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(8.0)
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            if !task.time.is_empty() {
                                                                let t_lbl = if task.end_time.is_empty() {
                                                                    task.time.clone()
                                                                } else {
                                                                    format!(
                                                                        "{}-{}",
                                                                        task.time, task.end_time
                                                                    )
                                                                };
                                                                ui.label(
                                                                    RichText::new(t_lbl)
                                                                        .size(10.0)
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
                                                            title_rt = title_rt.color(p.ink_primary);
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
                                                                Layout::right_to_left(Align::Center),
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
                                        let add_w = (ui.available_width() - 32.0).max(40.0);
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("+ Añadir")
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
                                                    RichText::new("↗").size(12.0).color(p.accent),
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

    ui.add_space(12.0);

    // Panel inferior de balance y bienestar (3 tarjetas horizontales sin solapamientos)
    let card_gap = 12.0_f32;
    let card_w = ((avail.x - card_gap * 2.0) / 3.0).floor().max(200.0);
    let (cat_total, cat_counts) = app.store.week_category_counts(week_start);

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = card_gap;

        // Tarjeta 1: Distribución por Categorías
        ui.allocate_ui_with_layout(
            Vec2::new(card_w, bottom_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width((card_w - 30.0).max(160.0));
                        ui.set_min_height((bottom_h - 30.0).max(110.0));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Distribución por Categorías")
                                    .size(14.0)
                                    .strong()
                                    .color(p.ink_primary),
                            );
                            ui.add_space(6.0);
                            for cat in TAG_PRESETS {
                                let cnt = cat_counts.get(cat).copied().unwrap_or(0);
                                let pct = (cnt * 100).checked_div(cat_total).unwrap_or(0);
                                let (fg, _) = category_colors(cat, app.dark_mode);
                                ui.horizontal(|ui| {
                                    let (dot_r, _) =
                                        ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                    ui.painter().circle_filled(dot_r.center(), 4.0, fg);
                                    ui.label(RichText::new(cat).size(12.0).color(p.ink_primary));

                                    // Mini barra de progreso proporcional
                                    let bar_w = (ui.available_width() - 64.0).max(24.0);
                                    let (bar_r, _) = ui.allocate_exact_size(
                                        Vec2::new(bar_w, 5.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        bar_r,
                                        Rounding::same(2.5),
                                        p.bg_elevated,
                                    );
                                    if pct > 0 {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                bar_r.min,
                                                Vec2::new(bar_w * (pct as f32 / 100.0), 5.0),
                                            ),
                                            Rounding::same(2.5),
                                            fg,
                                        );
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!("{pct}% ({cnt})"))
                                                .size(11.0)
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

        // Tarjeta 2: Intención de la Semana
        ui.allocate_ui_with_layout(
            Vec2::new(card_w, bottom_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(12.0))
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
                                app.persist();
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
                    .rounding(Rounding::same(12.0))
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
                            let balance_title = if week_total <= 12 {
                                "Sostenible & Fluida"
                            } else if week_total <= 24 {
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
                                    "{week_total} bloques en 7 días ({week_done} completados)."
                                ))
                                .size(12.0)
                                .color(p.ink_secondary),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let half_w = ((ui.available_width() - 8.0) / 2.0).max(80.0);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Exportar Resumen")
                                                .size(11.5)
                                                .color(p.ink_primary),
                                        )
                                        .fill(p.bg_elevated)
                                        .min_size(Vec2::new(half_w, 30.0)),
                                    )
                                    .clicked()
                                {
                                    export_week_summary(app, week_start, week_num);
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Agenda de Hoy")
                                                .size(11.5)
                                                .strong()
                                                .color(Color32::WHITE),
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
