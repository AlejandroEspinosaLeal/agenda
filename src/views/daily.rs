use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{
    self, Align, Color32, Key, Layout, RichText, Rounding, Sense, Stroke, Vec2,
};

use crate::app::AgendaApp;
use crate::calendar::{month_name_es, month_short_es, weekday_name_es, Date};
use crate::model::Priority;
use crate::theme::{category_colors, Palette};
use crate::views::{self, COLOR_PRESETS, TAG_PRESETS};

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
                    .color(p.ink_muted),
                );
                if date == app.today {
                    let (hh, mm) = current_hh_mm();
                    views::badge(
                        ui,
                        &format!("{hh:02}:{mm:02} Actual"),
                        p.ink_secondary,
                        p.bg_surface,
                    );
                }
            });
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let focus_lbl = if app.focus_mode {
                "Salir de Enfoque"
            } else {
                "Modo Enfoque"
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(focus_lbl).size(12.5).strong().color(Color32::WHITE))
                        .fill(p.accent)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(118.0, 32.0)),
                )
                .clicked()
            {
                app.focus_mode = !app.focus_mode;
            }

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

    ui.add_space(12.0);

    let avail = ui.available_size();
    let right_w = if app.focus_mode { 0.0 } else { 355.0 };
    let left_w = (avail.x - if app.focus_mode { 0.0 } else { right_w + 16.0 }).max(420.0);

    ui.horizontal_top(|ui| {
        // Columna izquierda / central: Cronograma del Día + Ritmo del día
        ui.allocate_ui_with_layout(
            Vec2::new(left_w, avail.y),
            Layout::top_down(Align::Min),
            |ui| {
                let rhythm_h = 74.0_f32;
                let timeline_h = (avail.y - rhythm_h - 12.0).max(260.0);

                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_width((left_w - 34.0).max(360.0));
                        ui.set_min_height((timeline_h - 34.0).max(200.0));

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Cronograma del Día")
                                        .size(16.0)
                                        .strong()
                                        .color(p.ink_primary),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    views::category_filter_pills(
                                        ui,
                                        &mut app.category_filter,
                                        p,
                                        app.dark_mode,
                                    );
                                });
                            });

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(8.0);

                            let mut toggle_target = None;
                            let mut move_target = None;
                            let mut color_target = None;
                            let mut delete_target = None;

                            let (now_h, now_m) = current_hh_mm();
                            let now_str = format!("{now_h:02}:{now_m:02}");
                            let mut now_line_drawn = false;

                            egui::ScrollArea::vertical()
                                .max_height((timeline_h - 96.0).max(160.0))
                                .show(ui, |ui| {
                                    if occurrences.is_empty() {
                                        ui.add_space(28.0);
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                RichText::new("Agenda despejada para este día")
                                                    .size(15.0)
                                                    .strong()
                                                    .color(p.ink_secondary),
                                            );
                                            ui.label(
                                                RichText::new("Añade un bloque desde el panel derecho o con '+ Nueva Tarea'.")
                                                    .size(12.5)
                                                    .color(p.ink_muted),
                                            );
                                        });
                                    } else {
                                        let mut sorted = occurrences.clone();
                                        sorted.sort_by(|a, b| a.task.time.cmp(&b.task.time));

                                        for occ in &sorted {
                                            let task = &occ.task;
                                            if date == app.today
                                                && !now_line_drawn
                                                && !task.time.is_empty()
                                                && task.time > now_str
                                            {
                                                render_now_indicator(ui, &now_str, p);
                                                now_line_drawn = true;
                                            }

                                            let (fg, bg) = views::task_colors(
                                                task.color,
                                                &task.tag,
                                                task.priority,
                                                p,
                                                app.dark_mode,
                                            );

                                            ui.horizontal_top(|ui| {
                                                let time_col = if task.time.is_empty() {
                                                    "Sin hora"
                                                } else {
                                                    &task.time
                                                };
                                                ui.allocate_ui_with_layout(
                                                    Vec2::new(62.0, 24.0),
                                                    Layout::top_down(Align::Min),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(time_col)
                                                                .size(12.0)
                                                                .color(p.ink_muted),
                                                        );
                                                    },
                                                );

                                                // Tarjeta con banda lateral de color según categoría
                                                let card_resp = egui::Frame::none()
                                                    .fill(if task.done { p.done_bg } else { p.bg_canvas })
                                                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                                    .rounding(Rounding::same(10.0))
                                                    .inner_margin(egui::Margin {
                                                        left: 14.0,
                                                        right: 12.0,
                                                        top: 10.0,
                                                        bottom: 10.0,
                                                    })
                                                    .show(ui, |ui| {
                                                        ui.set_min_width(ui.available_width());
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                if !task.tag.is_empty() {
                                                                    views::badge(ui, &task.tag, fg, bg);
                                                                }
                                                                let (p_fg, p_bg) =
                                                                    views::priority_colors(task.priority, p);
                                                                views::badge(
                                                                    ui,
                                                                    &format!("• {}", task.priority.label_es()),
                                                                    p_fg,
                                                                    p_bg,
                                                                );
                                                                if !task.time.is_empty() {
                                                                    let range = if task.end_time.is_empty() {
                                                                        task.time.clone()
                                                                    } else {
                                                                        format!("{} - {}", task.time, task.end_time)
                                                                    };
                                                                    ui.label(
                                                                        RichText::new(range)
                                                                            .size(11.5)
                                                                            .color(p.ink_muted),
                                                                    );
                                                                }

                                                                ui.with_layout(
                                                                    Layout::right_to_left(Align::Center),
                                                                    |ui| {
                                                                        if ui
                                                                            .add(
                                                                                egui::Button::new(
                                                                                    RichText::new("×")
                                                                                        .size(14.0)
                                                                                        .color(p.ink_muted),
                                                                                )
                                                                                .frame(false),
                                                                            )
                                                                            .on_hover_text("Eliminar")
                                                                            .clicked()
                                                                        {
                                                                            delete_target =
                                                                                Some((occ.start_date, task.id));
                                                                        }
                                                                        if !task.done
                                                                            && task.span_days == 1
                                                                            && ui
                                                                                .add(
                                                                                    egui::Button::new(
                                                                                        RichText::new("→")
                                                                                            .size(13.0)
                                                                                            .color(p.ink_secondary),
                                                                                    )
                                                                                    .frame(false),
                                                                                )
                                                                                .on_hover_text("Mover a mañana")
                                                                                .clicked()
                                                                        {
                                                                            move_target =
                                                                                Some((occ.start_date, task.id));
                                                                        }
                                                                        let mut rgb = task.color.unwrap_or([
                                                                            fg.r(),
                                                                            fg.g(),
                                                                            fg.b(),
                                                                        ]);
                                                                        if ui
                                                                            .color_edit_button_srgb(&mut rgb)
                                                                            .changed()
                                                                        {
                                                                            color_target = Some((
                                                                                occ.start_date,
                                                                                task.id,
                                                                                Some(rgb),
                                                                            ));
                                                                        }
                                                                        let mut checked = task.done;
                                                                        if ui.checkbox(&mut checked, "").changed()
                                                                        {
                                                                            toggle_target =
                                                                                Some((occ.start_date, task.id));
                                                                        }
                                                                    },
                                                                );
                                                            });

                                                            ui.add_space(4.0);
                                                            let mut title_rt =
                                                                RichText::new(&task.title).size(15.0).strong();
                                                            if task.done {
                                                                title_rt =
                                                                    title_rt.strikethrough().color(p.ink_muted);
                                                            } else {
                                                                title_rt = title_rt.color(p.ink_primary);
                                                            }
                                                            ui.label(title_rt);

                                                            if !task.notes.is_empty() {
                                                                ui.label(
                                                                    RichText::new(&task.notes)
                                                                        .size(12.5)
                                                                        .color(p.ink_secondary),
                                                                );
                                                            }
                                                            if !task.location.is_empty() {
                                                                ui.label(
                                                                    RichText::new(&task.location)
                                                                        .size(11.5)
                                                                        .color(p.ink_muted),
                                                                );
                                                            }
                                                        });
                                                    });

                                                // Banda lateral de color
                                                let strip = egui::Rect::from_min_size(
                                                    card_resp.response.rect.min,
                                                    Vec2::new(4.0, card_resp.response.rect.height()),
                                                );
                                                ui.painter().rect_filled(
                                                    strip,
                                                    Rounding::same(2.0),
                                                    if task.done { p.done_green } else { fg },
                                                );
                                            });
                                            ui.add_space(8.0);
                                        }

                                        if date == app.today && !now_line_drawn {
                                            render_now_indicator(ui, &now_str, p);
                                        }
                                    }
                                });

                            let mut changed = false;
                            if let Some((sd, id)) = toggle_target {
                                changed |= app.store.toggle_task(sd, id);
                            }
                            if let Some((sd, id)) = move_target {
                                changed |= app.store.move_task_to_next_day(sd, id);
                            }
                            if let Some((sd, id, col)) = color_target {
                                changed |= app.store.set_task_color(sd, id, col);
                            }
                            if let Some((sd, id)) = delete_target {
                                changed |= app.store.delete_task(sd, id);
                            }
                            if changed {
                                app.persist();
                            }
                        });
                    });

                ui.add_space(10.0);

                // Tarjeta inferior de bienestar/ritmo
                let (rhythm_title, rhythm_desc) = if occurrences.is_empty() {
                    (
                        "Ritmo del día: Despejado",
                        "Tienes el día libre para descanso o trabajo profundo sin interrupciones.",
                    )
                } else if occurrences.len() <= 5 {
                    (
                        "Ritmo del día: Equilibrado",
                        "Has distribuido de forma saludable tus bloques de enfoque, descanso y bienestar.",
                    )
                } else {
                    (
                        "Ritmo del día: Intenso",
                        "Jornada exigente: recuerda reservar pausas breves entre bloques.",
                    )
                };

                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width((left_w - 26.0).max(360.0));
                        ui.horizontal(|ui| {
                            let (_, bg_ok) = category_colors("Salud", app.dark_mode);
                            views::badge(ui, "BIENESTAR", p.done_green, bg_ok);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(rhythm_title)
                                        .size(13.0)
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

        // Columna derecha: Widgets de Acción y Resumen
        if !app.focus_mode {
            ui.add_space(16.0);
            ui.allocate_ui_with_layout(
                Vec2::new(right_w, avail.y),
                Layout::top_down(Align::Min),
                |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        render_daily_right_column(app, ui, p, date, &occurrences);
                    });
                },
            );
        }
    });
}

fn render_now_indicator(ui: &mut egui::Ui, now_str: &str, p: Palette) {
    ui.horizontal(|ui| {
        views::badge(
            ui,
            &format!("{now_str} ACTUAL"),
            Color32::WHITE,
            p.priority_high,
        );
        let avail_w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, 12.0), Sense::hover());
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            Stroke::new(1.5_f32, p.priority_high),
        );
    });
    ui.add_space(6.0);
}

fn current_hh_mm() -> (u64, u64) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Offset +1h (CET) for local display
    let local_secs = secs + 3600;
    let day_secs = local_secs % 86_400;
    (day_secs / 3600, (day_secs % 3600) / 60)
}

fn render_daily_right_column(
    app: &mut AgendaApp,
    ui: &mut egui::Ui,
    p: Palette,
    date: Date,
    occurrences: &[crate::model::TaskOccurrence],
) {
    // Tarjeta 1: Nueva Tarea para Hoy
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(12.0))
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
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.new_time)
                            .hint_text("09:30")
                            .desired_width(68.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.new_end_time)
                            .hint_text("10:30")
                            .desired_width(68.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.new_notes)
                            .hint_text("Nota corta...")
                            .desired_width(f32::INFINITY),
                    );
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for (days, lbl) in [(1u8, "1 día"), (3u8, "3 días"), (7u8, "1 sem")] {
                        let sel = app.new_span_days == days;
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(lbl).size(11.0).color(if sel {
                                        Color32::WHITE
                                    } else {
                                        p.ink_primary
                                    }),
                                )
                                .fill(if sel { p.accent } else { p.bg_elevated }),
                            )
                            .clicked()
                        {
                            app.new_span_days = days;
                        }
                    }
                    let mut custom_days = i32::from(app.new_span_days);
                    if ui
                        .add(
                            egui::DragValue::new(&mut custom_days)
                                .range(1..=14)
                                .suffix("d"),
                        )
                        .on_hover_text("Duración personalizada (1–14 días)")
                        .changed()
                    {
                        app.new_span_days = custom_days.clamp(1, 14) as u8;
                    }
                });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for cat in TAG_PRESETS {
                        let sel = app.new_tag == cat;
                        let (fg, bg) = category_colors(cat, app.dark_mode);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(cat)
                                        .size(11.0)
                                        .strong()
                                        .color(if sel { Color32::WHITE } else { fg }),
                                )
                                .fill(if sel { fg } else { bg }),
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

    // Tarjeta 2: Progreso de Tareas
    let total = occurrences.len();
    let done = occurrences.iter().filter(|o| o.task.done).count();
    let pct = (done * 100).checked_div(total).unwrap_or(0) as u8;

    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(12.0))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Progreso de Tareas")
                                .size(14.5)
                                .strong()
                                .color(p.ink_primary),
                        );
                        ui.label(
                            RichText::new(format!("{done} de {total} completadas"))
                                .size(11.5)
                                .color(p.ink_muted),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        views::badge(ui, &format!("{pct}%"), p.accent, p.accent_soft);
                        if done > 0
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Limpiar").size(11.0).color(p.ink_muted),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            && app.store.clear_completed(date)
                        {
                            app.persist();
                        }
                    });
                });

                ui.add_space(8.0);
                let mut toggle_target = None;
                for occ in occurrences {
                    let task = &occ.task;
                    ui.horizontal(|ui| {
                        let mut checked = task.done;
                        if ui.checkbox(&mut checked, "").changed() {
                            toggle_target = Some((occ.start_date, task.id));
                        }
                        let mut rt = RichText::new(views::truncate_chars(&task.title, 26)).size(12.0);
                        rt = if task.done {
                            rt.strikethrough().color(p.ink_muted)
                        } else {
                            rt.color(p.ink_primary)
                        };
                        ui.label(rt);
                    });
                }
                if let Some((sd, id)) = toggle_target {
                    if app.store.toggle_task(sd, id) {
                        app.persist();
                    }
                }
            });
        });

    ui.add_space(12.0);

    // Tarjeta 3: Notas Rápidas del Día
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(12.0))
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
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                if resp.changed() {
                    app.persist();
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Guardado automático")
                            .size(10.5)
                            .color(p.ink_muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Sincronizado")
                                .size(10.5)
                                .color(p.done_green),
                        );
                    });
                });
            });
        });
}
