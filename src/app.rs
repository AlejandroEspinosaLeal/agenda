use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use eframe::egui::{self, Align, Color32, Key, Layout, RichText, Rounding, Sense, Stroke, Vec2};

use crate::calendar::{days_in_month, first_weekday_monday_zero, month_name_es, shift_month, Date};
use crate::export;
use crate::focus::{FocusPreset, FocusTimer};
use crate::model::{Priority, Recurrence, TaskDraft, TaskStore};
use crate::nlp;
use crate::theme::{apply_theme, Palette, Radius, Spacing, ThemePreset};
use crate::tour::{apply_step_view, render_tour_overlay, TourAnchors, TourOutcome};
use crate::tray::{build_daily_notification_body, show_windows_toast, TrayAction, TrayController};
use crate::updater::{self, UpdateStatus, Updater};
use crate::views::{self, COLOR_PRESETS, TAG_PRESETS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Monthly,
    Daily,
    Weekly,
}

/// Zero-allocation drag state for creating blocks directly on the time grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridCreateDrag {
    pub day: Date,
    pub anchor_min: u32,
    pub current_min: u32,
}

pub struct AgendaApp {
    pub today: Date,
    pub view_year: i32,
    pub view_month: u8,
    pub selected_date: Date,
    pub active_view: ActiveView,
    pub focus_mode: bool,
    pub daily_24h: bool,
    pub weekly_schedule_mode: bool,
    pub weekly_footer_open: bool,
    pub monthly_inspector_open: bool,
    pub notes_dirty_at: Option<Instant>,
    pub grid_create_drag: Option<GridCreateDrag>,
    pub monthly_range_drag: Option<(Date, Date)>,
    pub drag_backlog_id: Option<u64>,
    pub show_new_task_modal: bool,
    pub show_command_palette: bool,
    pub palette_query: String,
    pub palette_selection: Option<usize>,
    pub show_tour: bool,
    pub tour_step: usize,
    pub tour_anchors: TourAnchors,
    pub focus_timer: FocusTimer,
    pub tray: Option<TrayController>,
    pub updater: Updater,
    pub last_update_check: Instant,
    pub store: TaskStore,
    pub storage_path: PathBuf,
    // Draft fields for creating tasks
    pub new_title: String,
    pub new_notes: String,
    pub new_all_day: bool,
    pub new_time: String,
    pub new_end_time: String,
    pub new_location: String,
    pub new_tag: String,
    pub new_span_days: u8,
    pub new_recurrence: Recurrence,
    pub new_color: Option<[u8; 3]>,
    pub custom_rgb: [u8; 3],
    pub new_priority: Priority,
    // Filters & theme
    pub search_query: String,
    pub category_filter: Option<String>,
    pub dark_mode: bool,
    /// Cached `(preset, dark, Palette)`; `apply_theme` runs only when this key changes.
    pub palette_cache: Option<(ThemePreset, bool, Palette)>,
    pub status_error: Option<String>,
    new_task_modal_was_open: bool,
}

impl AgendaApp {
    pub fn new() -> Self {
        let today = Date::today();
        let storage_path = TaskStore::default_storage_path();
        let (store, status_error) = match TaskStore::load(&storage_path) {
            Ok(s) => (s, None),
            Err(err) => (
                TaskStore::default(),
                Some(format!("No se pudo leer tareas.dat: {err}")),
            ),
        };

        let mut tray = TrayController::new().ok();
        let mut updater = Updater::new();
        updater.check_for_updates();

        let show_tour = !store.onboarding_completed;
        let dark_mode = store.dark_mode;
        let app = Self {
            today,
            view_year: today.year,
            view_month: today.month,
            selected_date: today,
            active_view: ActiveView::Monthly,
            focus_mode: false,
            daily_24h: false,
            weekly_schedule_mode: false,
            weekly_footer_open: true,
            monthly_inspector_open: true,
            notes_dirty_at: None,
            grid_create_drag: None,
            monthly_range_drag: None,
            drag_backlog_id: None,
            show_new_task_modal: false,
            show_command_palette: false,
            palette_query: String::new(),
            palette_selection: None,
            show_tour,
            tour_step: 0,
            tour_anchors: TourAnchors::default(),
            focus_timer: FocusTimer::new(),
            tray: None,
            updater,
            last_update_check: Instant::now(),
            store,
            storage_path,
            new_title: String::new(),
            new_notes: String::new(),
            new_all_day: true,
            new_time: String::new(),
            new_end_time: String::new(),
            new_location: String::new(),
            new_tag: "Trabajo".to_owned(),
            new_span_days: 1,
            new_recurrence: Recurrence::None,
            new_color: None,
            custom_rgb: [67, 56, 202],
            new_priority: Priority::Normal,
            search_query: String::new(),
            category_filter: None,
            dark_mode,
            palette_cache: None,
            status_error,
            new_task_modal_was_open: false,
        };
        if let Some(t) = tray.as_mut() {
            let pending = app.pending_titles_today();
            t.sync_today(&pending);
        }
        let mut app = Self { tray, ..app };
        app.maybe_send_daily_notification();
        app
    }

    pub fn send_daily_notification_now(&self) {
        let pending = self.pending_titles_today();
        let title = format!(
            "Mi Agenda · Hoy ({:02}/{:02})",
            self.today.day, self.today.month
        );
        let body = build_daily_notification_body(&pending);
        show_windows_toast(&title, &body);
    }

    pub fn maybe_send_daily_notification(&mut self) {
        let today_key = self.today.to_key();
        if self.store.last_notified_date.as_deref() == Some(today_key.as_str()) {
            return;
        }
        self.send_daily_notification_now();
        self.store.last_notified_date = Some(today_key);
        self.persist();
    }

    fn pending_titles_today(&self) -> Vec<String> {
        self.store
            .tasks_visible_on(self.today)
            .into_iter()
            .filter(|occ| !occ.task.done)
            .map(|occ| {
                if occ.task.time.is_empty() {
                    occ.task.title
                } else {
                    format!("{} {}", occ.task.time, occ.task.title)
                }
            })
            .collect()
    }

    pub fn persist(&mut self) {
        self.notes_dirty_at = None;
        if let Err(err) = self.store.save(&self.storage_path) {
            self.status_error = Some(format!("Error al guardar: {err}"));
        } else {
            self.status_error = None;
        }
        let pending = self.pending_titles_today();
        if let Some(t) = self.tray.as_mut() {
            t.sync_today(&pending);
        }
    }

    /// Marks notes as modified so they auto-save after a 600ms debounce without disk thrashing.
    pub fn mark_notes_dirty(&mut self) {
        self.notes_dirty_at = Some(Instant::now());
    }

    pub fn export_to_desktop(&mut self, filename: &str, content: &str) {
        let path = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Desktop")
            .join(filename);
        if let Err(err) = std::fs::write(&path, content) {
            self.status_error = Some(format!("No se pudo exportar {filename}: {err}"));
        } else {
            self.status_error = None;
        }
    }

    /// Opens the new-task modal pre-filled with a specific date and 15-min snapped time range.
    pub fn open_timed_draft(&mut self, day: Date, start_min: u32, end_min: u32) {
        self.select_date(day);
        self.new_all_day = false;
        self.new_time = format!("{:02}:{:02}", start_min / 60, start_min % 60);
        let safe_end = end_min.max(start_min + 15).min(1439);
        self.new_end_time = format!("{:02}:{:02}", safe_end / 60, safe_end % 60);
        self.show_new_task_modal = true;
    }

    /// Opens the new-task modal pre-filled for a multi-day range dragged in Monthly view.
    pub fn open_span_draft(&mut self, start_date: Date, span_days: u8) {
        self.select_date(start_date);
        self.new_all_day = true;
        self.new_time.clear();
        self.new_end_time.clear();
        self.new_span_days = span_days.clamp(1, 31);
        self.new_recurrence = Recurrence::None;
        self.show_new_task_modal = true;
    }

    pub fn select_date(&mut self, date: Date) {
        self.view_year = date.year;
        self.view_month = date.month;
        self.selected_date = date;
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        let Some(action) = self.tray.as_ref().and_then(TrayController::poll_action) else {
            return;
        };
        match action {
            TrayAction::OpenFull => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            TrayAction::NotifyToday => {
                self.today = Date::today();
                self.send_daily_notification_now();
            }
            TrayAction::Quit => {
                crate::tray::QUIT_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    pub fn change_month(&mut self, delta: i32) {
        let (y, m) = shift_month(self.view_year, self.view_month, delta);
        self.view_year = y;
        self.view_month = m;
        let max_day = days_in_month(y, m);
        let day = self.selected_date.day.min(max_day);
        self.selected_date = Date::new(y, m, day);
    }

    pub fn submit_draft(&mut self, date: Date) -> bool {
        let (t_start, t_end) = if self.new_all_day {
            ("", "")
        } else {
            (self.new_time.as_str(), self.new_end_time.as_str())
        };
        let added = self.store.add_task(
            date,
            TaskDraft {
                title: &self.new_title,
                notes: &self.new_notes,
                time: t_start,
                end_time: t_end,
                location: &self.new_location,
                tag: &self.new_tag,
                span_days: self.new_span_days,
                recurrence: self.new_recurrence,
                color: self.new_color,
                priority: self.new_priority,
            },
        );
        if added {
            self.new_title.clear();
            self.new_notes.clear();
            self.new_time.clear();
            self.new_end_time.clear();
            self.new_location.clear();
            self.persist();
        }
        added
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if self.show_command_palette {
                self.show_command_palette = false;
                self.palette_selection = None;
                return;
            }
            if self.show_new_task_modal {
                self.show_new_task_modal = false;
                return;
            }
            if self.show_tour {
                // The tour overlay consumes Escape itself so it never collapses the inspector.
                return;
            }
            if !self.search_query.is_empty() {
                self.search_query.clear();
                return;
            }
            if self.focus_mode {
                self.focus_mode = false;
                return;
            }
            if self.active_view == ActiveView::Monthly && self.monthly_inspector_open {
                self.monthly_inspector_open = false;
                return;
            }
        }
        // Never stack the palette on top of the modal or the onboarding tour.
        if self.show_tour || self.show_new_task_modal {
            return;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::K)) {
            self.show_command_palette = !self.show_command_palette;
            self.palette_selection = None;
            if self.show_command_palette {
                self.palette_query.clear();
            }
            return;
        }
        if ctx.wants_keyboard_input() || self.show_command_palette {
            return;
        }

        ctx.input(|i| {
            if i.key_pressed(Key::M) {
                self.active_view = ActiveView::Monthly;
            } else if i.key_pressed(Key::W) {
                self.active_view = ActiveView::Weekly;
            } else if i.key_pressed(Key::D) {
                self.active_view = ActiveView::Daily;
            } else if i.key_pressed(Key::T) {
                self.today = Date::today();
                self.select_date(self.today);
            } else if i.key_pressed(Key::C) {
                self.show_new_task_modal = true;
            } else if i.key_pressed(Key::F) {
                self.focus_mode = !self.focus_mode;
            } else if i.key_pressed(Key::ArrowLeft) {
                match self.active_view {
                    ActiveView::Monthly => self.change_month(-1),
                    ActiveView::Daily => self.select_date(self.selected_date.add_days(-1)),
                    ActiveView::Weekly => self.select_date(self.selected_date.add_days(-7)),
                }
            } else if i.key_pressed(Key::ArrowRight) {
                match self.active_view {
                    ActiveView::Monthly => self.change_month(1),
                    ActiveView::Daily => self.select_date(self.selected_date.add_days(1)),
                    ActiveView::Weekly => self.select_date(self.selected_date.add_days(7)),
                }
            }
        });
    }

    fn render_header(&mut self, ui: &mut egui::Ui, p: Palette) {
        let focus_search = ui.input(|i| i.modifiers.ctrl && i.key_pressed(Key::F));
        let compact = ui.available_width() < 1080.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;

            // Brand mark
            egui::Frame::none()
                .fill(p.accent)
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("MA").size(13.0).strong().color(p.on_accent));
                });
            ui.label(
                RichText::new("Mi Agenda")
                    .size(18.0)
                    .strong()
                    .color(p.ink_primary),
            );

            ui.add_space(if compact { 8.0 } else { 16.0 });

            // Global Search Bar
            let search_w = if compact {
                (ui.available_width() - 470.0).clamp(110.0, 240.0)
            } else {
                (ui.available_width() - 520.0).clamp(160.0, 380.0)
            };
            let search_resp = ui.add_sized(
                Vec2::new(search_w, 34.0),
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Filtrar vista... (Ctrl+F)")
                    .vertical_align(Align::Center),
            );
            if focus_search {
                search_resp.request_focus();
            }

            let cmd_resp = ui
                .add(
                    egui::Button::new(
                        RichText::new("Ctrl+K")
                            .size(11.5)
                            .monospace()
                            .color(p.ink_secondary),
                    )
                    .fill(p.bg_elevated)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .min_size(Vec2::new(62.0, 34.0)),
                )
                .on_hover_text("Abrir paleta de comandos y entrada rápida (Ctrl+K)");
            self.tour_anchors.command_palette = Some(cmd_resp.rect);
            if cmd_resp.clicked() {
                self.show_command_palette = !self.show_command_palette;
                self.palette_query.clear();
                self.palette_selection = None;
            }

            if !self.search_query.is_empty()
                && ui
                    .add(
                        egui::Button::new(RichText::new("×").size(15.0).color(p.ink_primary))
                            .fill(p.bg_elevated)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .min_size(Vec2::new(30.0, 34.0)),
                    )
                    .on_hover_text("Limpiar búsqueda (Esc)")
                    .clicked()
            {
                self.search_query.clear();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let help_resp = ui
                    .add(
                        egui::Button::new(
                            RichText::new("?").size(13.0).strong().color(p.ink_primary),
                        )
                        .fill(p.bg_elevated)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .min_size(Vec2::new(32.0, 34.0)),
                    )
                    .on_hover_text("Ver guía interactiva de bienvenida");
                if help_resp.clicked() {
                    self.tour_step = 0;
                    self.show_tour = true;
                }

                let theme_label = if self.dark_mode { "Claro" } else { "Oscuro" };
                let theme_resp = ui
                    .add(
                        egui::Button::new(
                            RichText::new(theme_label)
                                .size(12.5)
                                .strong()
                                .color(p.ink_primary),
                        )
                        .fill(p.bg_elevated)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .min_size(Vec2::new(if compact { 62.0 } else { 72.0 }, 34.0)),
                    )
                    .on_hover_text("Cambiar tema claro / oscuro");
                if theme_resp.clicked() {
                    self.dark_mode = !self.dark_mode;
                    self.store.dark_mode = self.dark_mode;
                    self.persist();
                }

                let palette_label = format!("● {} v", self.store.theme_preset.label_es());
                let palette_resp = ui.menu_button(
                    RichText::new(palette_label)
                        .size(12.0)
                        .strong()
                        .color(p.accent),
                    |ui| {
                        for preset in ThemePreset::ALL {
                            let selected = self.store.theme_preset == preset;
                            let dot_col = preset.accent_color(self.dark_mode);
                            ui.horizontal(|ui| {
                                let (dot_r, _) =
                                    ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                ui.painter().circle_filled(dot_r.center(), 4.5, dot_col);
                                let lbl = if selected {
                                    format!("{}  •", preset.label_es())
                                } else {
                                    preset.label_es().to_owned()
                                };
                                if ui
                                    .selectable_label(
                                        selected,
                                        RichText::new(lbl).size(12.5).color(p.ink_primary),
                                    )
                                    .clicked()
                                {
                                    self.store.theme_preset = preset;
                                    self.persist();
                                    ui.close_menu();
                                }
                            });
                        }
                    },
                );
                self.tour_anchors.theme_and_help = Some(
                    help_resp
                        .rect
                        .union(theme_resp.rect)
                        .union(palette_resp.response.rect),
                );

                let focus_label = if self.focus_mode {
                    "Salir de foco"
                } else {
                    "En foco"
                };
                let focus_resp = ui
                    .add(
                        egui::Button::new(RichText::new(focus_label).size(12.5).strong().color(
                            if self.focus_mode {
                                p.accent
                            } else {
                                p.ink_primary
                            },
                        ))
                        .fill(if self.focus_mode {
                            p.accent_soft
                        } else {
                            p.bg_elevated
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if self.focus_mode { p.accent } else { p.border },
                        ))
                        .min_size(Vec2::new(if compact { 78.0 } else { 86.0 }, 34.0)),
                    )
                    .on_hover_text("Oculta barras laterales para concentrarte");
                self.tour_anchors.focus_mode = Some(focus_resp.rect);
                if focus_resp.clicked() {
                    self.focus_mode = !self.focus_mode;
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(if compact { "+ Tarea" } else { "+ Nueva Tarea" })
                                .size(13.0)
                                .strong()
                                .color(p.on_accent),
                        )
                        .fill(p.accent)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(if compact { 88.0 } else { 124.0 }, 34.0)),
                    )
                    .clicked()
                {
                    self.show_new_task_modal = true;
                }
            });
        });

        if let Some(err) = &self.status_error {
            ui.add_space(4.0);
            ui.label(RichText::new(err).color(p.priority_high).size(12.0));
        }
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.vertical(|ui| {
            // a) Navegación de vistas
            let nav_resp = ui.vertical(|ui| {
                ui.label(
                    RichText::new("PLANIFICACIÓN")
                        .size(10.5)
                        .strong()
                        .color(p.ink_muted),
                );
                ui.add_space(Spacing::XS);

                let items = [
                    (ActiveView::Monthly, "Calendario Mensual"),
                    (ActiveView::Weekly, "Vista Semanal"),
                    (ActiveView::Daily, "Agenda Diaria"),
                ];

                for (view, label) in items {
                    let active = self.active_view == view;
                    let btn = egui::Button::new(
                        RichText::new(label).size(13.0).strong().color(if active {
                            p.on_accent
                        } else {
                            p.ink_primary
                        }),
                    )
                    .fill(if active {
                        p.accent
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(Radius::MD))
                    .min_size(Vec2::new(ui.available_width(), 32.0));

                    if ui.add(btn).clicked() {
                        self.active_view = view;
                    }
                }
            });
            self.tour_anchors.views_nav = Some(nav_resp.response.rect);

            ui.add_space(Spacing::MD);

            // b) Mini-Calendario Mensual interactivo
            self.render_sidebar_mini_calendar(ui, p);

            ui.add_space(Spacing::MD);

            // c) Barra compacta de progreso diario + Backlog sin programar
            let (pending, done) = self.store.counts_for(self.today);
            let total = pending + done;
            let pct = (done * 100).checked_div(total).unwrap_or(0);
            let pct_f = if total == 0 {
                0.0
            } else {
                done as f32 / total as f32
            };

            egui::Frame::none()
                .fill(p.bg_surface)
                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Hoy")
                                    .size(11.5)
                                    .strong()
                                    .color(p.ink_primary),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{pct}% ({done}/{total})"))
                                        .size(11.0)
                                        .monospace()
                                        .strong()
                                        .color(p.accent),
                                );
                            });
                        });
                        ui.add_space(Spacing::XS);
                        let w = ui.available_width();
                        let (bar_rect, _) =
                            ui.allocate_exact_size(Vec2::new(w, 5.0), Sense::hover());
                        ui.painter()
                            .rect_filled(bar_rect, Rounding::same(3.0), p.bg_elevated);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(bar_rect.min, Vec2::new(w * pct_f, 5.0)),
                            Rounding::same(3.0),
                            p.accent,
                        );
                    });
                });

            ui.add_space(Spacing::SM);

            let backlog = self.store.unscheduled_backlog();
            let backlog_hdr_resp = ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SIN PROGRAMAR")
                        .size(10.5)
                        .strong()
                        .color(p.ink_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(backlog.len().to_string())
                            .size(10.5)
                            .monospace()
                            .color(p.ink_muted),
                    );
                });
            });
            self.tour_anchors.backlog = Some(backlog_hdr_resp.response.rect);
            ui.add_space(Spacing::XS);

            let mut jump_to = None;
            let list_h = (ui.available_height() - 32.0).max(60.0);
            egui::ScrollArea::vertical()
                .id_salt("sidebar_backlog")
                .max_height(list_h)
                .show(ui, |ui| {
                    if backlog.is_empty() {
                        ui.label(
                            RichText::new("Sin tareas pendientes sin hora")
                                .size(11.0)
                                .color(p.ink_muted),
                        );
                    } else {
                        for occ in backlog.iter().take(12) {
                            let tok = p.category_token(&occ.task.tag);
                            let btn = egui::Button::new(
                                RichText::new(views::truncate_chars(&occ.task.title, 22))
                                    .size(11.5)
                                    .color(p.ink_primary),
                            )
                            .fill(p.bg_surface)
                            .stroke(Stroke::new(1.0_f32, tok.border))
                            .rounding(Rounding::same(Radius::SM))
                            .sense(Sense::click_and_drag())
                            .min_size(Vec2::new(ui.available_width(), 24.0));
                            let resp = ui
                                .add(btn)
                                .on_hover_text("Arrastra al horario o haz clic para abrir");
                            if resp.drag_started() {
                                self.drag_backlog_id = Some(occ.task.id);
                            } else if resp.clicked() {
                                jump_to = Some(occ.start_date);
                            }
                        }
                    }
                });
            if let Some(d) = jump_to {
                self.select_date(d);
                self.active_view = ActiveView::Daily;
            }

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                // Updater status & button at the very bottom of sidebar
                match &self.updater.status {
                    UpdateStatus::Checking => {
                        ui.label(
                            RichText::new("Buscando actualizaciones...")
                                .size(11.0)
                                .color(p.ink_muted),
                        );
                    }
                    UpdateStatus::Downloading(ver) => {
                        ui.label(
                            RichText::new(format!("Descargando {ver}..."))
                                .size(11.0)
                                .color(p.accent),
                        );
                    }
                    UpdateStatus::UpToDate => {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!(
                                        "v{} · Al día (revisar)",
                                        updater::CURRENT_VERSION
                                    ))
                                    .size(11.0)
                                    .color(p.ink_muted),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.updater.check_for_updates();
                        }
                    }
                    UpdateStatus::Error(err) => {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Reintentar actualización")
                                        .size(11.0)
                                        .color(p.priority_high),
                                )
                                .frame(false),
                            )
                            .on_hover_text(err)
                            .clicked()
                        {
                            self.updater.check_for_updates();
                        }
                    }
                    UpdateStatus::Idle => {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!(
                                        "v{} · Buscar actualización",
                                        updater::CURRENT_VERSION
                                    ))
                                    .size(11.0)
                                    .color(p.ink_muted),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.updater.check_for_updates();
                        }
                    }
                }

                ui.add_space(6.0);
            });
        });
    }

    fn render_sidebar_mini_calendar(&mut self, ui: &mut egui::Ui, p: Palette) {
        egui::Frame::none()
            .fill(p.bg_surface)
            .stroke(Stroke::new(1.0_f32, p.border_subtle))
            .rounding(Rounding::same(Radius::LG))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                month_name_es(self.view_month),
                                self.view_year
                            ))
                            .size(11.5)
                            .strong()
                            .color(p.ink_primary),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("›").size(12.0).color(p.ink_secondary),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.change_month(1);
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("‹").size(12.0).color(p.ink_secondary),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.change_month(-1);
                            }
                        });
                    });

                    let first_wd =
                        first_weekday_monday_zero(self.view_year, self.view_month) as usize;
                    let total_days = days_in_month(self.view_year, self.view_month) as usize;
                    let rows = (first_wd + total_days).div_ceil(7).max(5);
                    let cell_w = (ui.available_width() / 7.0).floor().max(20.0);
                    let cell_h = 22.0_f32;

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for wd in ["L", "M", "X", "J", "V", "S", "D"] {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(cell_w, 14.0), Sense::hover());
                            ui.painter().text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                wd,
                                egui::FontId::proportional(9.5),
                                p.ink_muted,
                            );
                        }
                    });

                    let origin = ui.cursor().min;
                    let (_grid_r, _) = ui.allocate_exact_size(
                        Vec2::new(cell_w * 7.0, cell_h * rows as f32),
                        Sense::hover(),
                    );
                    let painter = ui.painter();
                    let first_of_month = Date::new(self.view_year, self.view_month, 1);

                    for row in 0..rows {
                        for col in 0..7 {
                            let idx = row * 7 + col;
                            let in_month = idx >= first_wd && idx < first_wd + total_days;
                            if !in_month {
                                continue;
                            }
                            let cell_date = first_of_month.add_days(idx as i64 - first_wd as i64);
                            let min = origin + Vec2::new(col as f32 * cell_w, row as f32 * cell_h);
                            let rect = egui::Rect::from_min_size(min, Vec2::new(cell_w, cell_h));
                            let resp = ui.interact(
                                rect,
                                ui.id().with(("sb_mini", cell_date)),
                                Sense::click(),
                            );
                            if resp.clicked() {
                                self.select_date(cell_date);
                            }

                            let is_today = cell_date == self.today;
                            let is_sel = cell_date == self.selected_date;
                            if is_today {
                                painter.circle_filled(rect.center(), 9.0, p.accent);
                            } else if is_sel {
                                painter.circle_filled(rect.center(), 9.0, p.accent_soft);
                            } else if resp.hovered() {
                                painter.circle_filled(rect.center(), 9.0, p.bg_cell_hover);
                            }

                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                cell_date.day.to_string(),
                                egui::FontId::monospace(10.0),
                                if is_today {
                                    p.on_accent
                                } else if is_sel {
                                    p.accent
                                } else {
                                    p.ink_primary
                                },
                            );

                            let (pend, _) = self.store.counts_for(cell_date);
                            if pend > 0 {
                                painter.circle_filled(
                                    egui::pos2(rect.center().x, rect.max.y - 2.0),
                                    1.8,
                                    if is_today { p.on_accent } else { p.accent },
                                );
                            }
                        }
                    }
                });
            });
    }

    fn render_new_task_modal(&mut self, ctx: &egui::Context, p: Palette) {
        if !self.show_new_task_modal {
            self.new_task_modal_was_open = false;
            return;
        }
        let just_opened = !self.new_task_modal_was_open;
        self.new_task_modal_was_open = true;
        let mut open = self.show_new_task_modal;
        let mut close_on_submit = false;
        let date = self.selected_date;
        let end_date = date.add_days(i64::from(self.new_span_days.max(1) - 1));
        let date_heading = if self.new_span_days > 1 {
            if date.month == end_date.month {
                format!(
                    "{} – {} de {}",
                    date.day,
                    end_date.day,
                    month_name_es(date.month),
                )
            } else {
                format!(
                    "{} {} – {} {}",
                    date.day,
                    month_name_es(date.month),
                    end_date.day,
                    month_name_es(end_date.month),
                )
            }
        } else {
            format!("{} de {}", date.day, month_name_es(date.month))
        };

        // Subtle dim backdrop for modern modal depth (blocks click-through to the calendar)
        let screen = ctx.screen_rect();
        let mut backdrop_clicked = false;
        egui::Area::new(egui::Id::new("new_task_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let backdrop = ui.allocate_rect(screen, Sense::click_and_drag());
                if backdrop.clicked() {
                    backdrop_clicked = true;
                }
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    Color32::from_black_alpha(if p.dark { 135 } else { 85 }),
                );
            });
        if backdrop_clicked {
            self.show_new_task_modal = false;
            self.new_task_modal_was_open = false;
            return;
        }

        egui::Window::new("new_task_modal_win")
            .id(egui::Id::new("new_task_modal"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(450.0)
            .max_width(450.0)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .rounding(Rounding::same(14.0))
                    .inner_margin(20.0),
            )
            .show(ctx, |ui| {
                // Modern header: badge + date + close button
                ui.horizontal(|ui| {
                    let badge_txt = if self.new_span_days > 1 {
                        format!("{} DÍAS", self.new_span_days)
                    } else {
                        "NUEVA TAREA".to_owned()
                    };
                    views::badge(ui, &badge_txt, p.accent, p.accent_soft);
                    ui.label(
                        RichText::new(&date_heading)
                            .size(16.0)
                            .strong()
                            .color(p.ink_primary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("×").size(12.5).color(p.ink_secondary),
                                )
                                .fill(p.bg_elevated)
                                .stroke(Stroke::NONE)
                                .rounding(Rounding::same(Radius::SM))
                                .min_size(Vec2::splat(28.0)),
                            )
                            .clicked()
                        {
                            open = false;
                        }
                        if self.new_span_days > 1
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Solo 1 día")
                                            .size(11.0)
                                            .color(p.ink_secondary),
                                    )
                                    .fill(p.bg_elevated)
                                    .rounding(Rounding::same(Radius::SM)),
                                )
                                .clicked()
                        {
                            self.new_span_days = 1;
                        }
                    });
                });

                ui.add_space(Spacing::MD);

                // Hero title input
                let title_resp = ui.add_sized(
                    Vec2::new(ui.available_width(), 38.0),
                    egui::TextEdit::singleline(&mut self.new_title)
                        .hint_text("¿Qué quieres planificar? (ej. Comprar billetes)")
                        .font(egui::FontId::proportional(15.0))
                        .margin(egui::Margin::symmetric(12.0, 8.0)),
                );
                if just_opened {
                    title_resp.request_focus();
                }

                ui.add_space(Spacing::MD);

                // Grouped schedule & recurrence card
                egui::Frame::none()
                    .fill(p.bg_canvas)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(Radius::MD))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        views::schedule_and_recurrence_controls(
                            ui,
                            &mut self.new_all_day,
                            &mut self.new_time,
                            &mut self.new_end_time,
                            &mut self.new_recurrence,
                            p,
                        );
                    });

                ui.add_space(Spacing::SM);

                // Location & Notes side by side
                ui.horizontal(|ui| {
                    let half = ((ui.available_width() - Spacing::SM) * 0.5).max(120.0);
                    ui.add_sized(
                        Vec2::new(half, 32.0),
                        egui::TextEdit::singleline(&mut self.new_location)
                            .hint_text("Lugar (opcional)")
                            .margin(egui::Margin::symmetric(10.0, 6.0)),
                    );
                    ui.add_sized(
                        Vec2::new(half, 32.0),
                        egui::TextEdit::singleline(&mut self.new_notes)
                            .hint_text("Nota breve (opcional)")
                            .margin(egui::Margin::symmetric(10.0, 6.0)),
                    );
                });

                ui.add_space(Spacing::MD);

                // Category & Priority cleanly organized
                ui.label(
                    RichText::new("CATEGORÍA")
                        .size(10.5)
                        .strong()
                        .color(p.ink_muted),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for cat in TAG_PRESETS {
                        let selected = self.new_tag == cat;
                        let tok = p.category_token(cat);
                        let btn = egui::Button::new(
                            RichText::new(cat).size(11.5).strong().color(if selected {
                                tok.on_solid
                            } else {
                                tok.text
                            }),
                        )
                        .fill(if selected { tok.solid } else { tok.bg })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if selected { tok.solid } else { tok.border },
                        ))
                        .rounding(Rounding::same(Radius::PILL));
                        if ui.add(btn).clicked() {
                            self.new_tag = if selected {
                                String::new()
                            } else {
                                cat.to_owned()
                            };
                        }
                    }
                });

                ui.add_space(Spacing::SM);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("PRIORIDAD")
                            .size(10.5)
                            .strong()
                            .color(p.ink_muted),
                    );
                    ui.add_space(4.0);
                    for prio in [Priority::Low, Priority::Normal, Priority::High] {
                        let selected = self.new_priority == prio;
                        let (fg, bg) = views::priority_colors(prio, p);
                        let btn = egui::Button::new(
                            RichText::new(prio.label_es())
                                .size(11.5)
                                .strong()
                                .color(if selected { fg } else { p.ink_secondary }),
                        )
                        .fill(if selected { bg } else { p.bg_elevated })
                        .stroke(Stroke::new(
                            if selected { 1.5_f32 } else { 1.0_f32 },
                            if selected { fg } else { p.border_subtle },
                        ))
                        .rounding(Rounding::same(Radius::PILL));
                        if ui.add(btn).clicked() {
                            self.new_priority = prio;
                        }
                    }
                });

                ui.add_space(Spacing::SM);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("COLOR")
                            .size(10.5)
                            .strong()
                            .color(p.ink_muted),
                    );
                    ui.add_space(4.0);
                    let auto = self.new_color.is_none();
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Auto").size(11.0).color(if auto {
                                p.accent
                            } else {
                                p.ink_secondary
                            }))
                            .fill(if auto { p.accent_soft } else { p.bg_elevated })
                            .rounding(Rounding::same(Radius::PILL)),
                        )
                        .clicked()
                    {
                        self.new_color = None;
                    }
                    for preset in COLOR_PRESETS {
                        let selected = self.new_color == Some(preset);
                        let c = Color32::from_rgb(preset[0], preset[1], preset[2]);
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                        ui.painter().circle_filled(rect.center(), 7.5, c);
                        if selected {
                            ui.painter().circle_stroke(
                                rect.center(),
                                9.0,
                                Stroke::new(1.5_f32, p.ink_primary),
                            );
                        }
                        if resp.clicked() {
                            self.new_color = Some(preset);
                            self.custom_rgb = preset;
                        }
                    }
                    if ui.color_edit_button_srgb(&mut self.custom_rgb).changed() {
                        self.new_color = Some(self.custom_rgb);
                    }
                });

                ui.add_space(Spacing::MD);
                ui.separator();
                ui.add_space(Spacing::SM);

                let enter_submit =
                    title_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                let mut save_clicked = false;
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        save_clicked = ui
                            .add(
                                egui::Button::new(
                                    RichText::new("+ Guardar en Agenda")
                                        .size(13.0)
                                        .strong()
                                        .color(p.on_accent),
                                )
                                .fill(p.accent)
                                .stroke(Stroke::NONE)
                                .rounding(Rounding::same(Radius::MD))
                                .min_size(Vec2::new(165.0, 36.0)),
                            )
                            .clicked();

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Cancelar").size(12.5).color(p.ink_secondary),
                                )
                                .fill(p.bg_elevated)
                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                .rounding(Rounding::same(Radius::MD))
                                .min_size(Vec2::new(90.0, 36.0)),
                            )
                            .clicked()
                        {
                            open = false;
                        }
                    });
                });

                if (enter_submit || save_clicked) && self.submit_draft(date) {
                    close_on_submit = true;
                }
            });

        self.show_new_task_modal = open && !close_on_submit;
        if !self.show_new_task_modal {
            self.new_span_days = 1;
            self.new_task_modal_was_open = false;
        }
    }

    fn render_command_palette(&mut self, ctx: &egui::Context, p: Palette) {
        if !self.show_command_palette {
            return;
        }
        let mut close_palette = false;
        egui::Window::new("command_palette")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 88.0))
            .fixed_size(Vec2::new(560.0, 380.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.5_f32, p.accent))
                    .rounding(Rounding::same(Radius::LG))
                    .inner_margin(16.0),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Ctrl+K")
                            .size(11.0)
                            .monospace()
                            .strong()
                            .color(p.accent),
                    );
                    let resp = ui.add_sized(
                        Vec2::new(ui.available_width() - 36.0, 34.0),
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text(
                                "Buscar, comando o crear (ej. \"Estudiar Rust 18:00-19:30 #Estudio\")",
                            )
                            .vertical_align(Align::Center),
                    );
                    resp.request_focus();
                    if resp.changed() {
                        self.palette_selection = None;
                    }
                    if ui.button("×").clicked() {
                        close_palette = true;
                        self.palette_selection = None;
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                let q = self.palette_query.trim().to_owned();
                const COMMANDS: [(&str, &str); 8] = [
                    ("hoy", "Ir a hoy (T)"),
                    ("guia", "Ver guía interactiva de bienvenida"),
                    ("notificacion", "Mostrar notificación de hoy"),
                    ("tema", "Cambiar tema claro / oscuro"),
                    ("paleta", "Siguiente paleta de colores"),
                    ("enfoque", "Iniciar / Salir de Modo Enfoque (F)"),
                    ("markdown", "Exportar semana a Markdown (.md)"),
                    ("ics", "Exportar agenda a iCalendar (.ics)"),
                ];
                let cmd_filter = q.trim_start_matches('>').trim().to_lowercase();
                let quick = if !q.is_empty() && !q.starts_with('>') {
                    nlp::parse_quick_entry(&q)
                } else {
                    None
                };
                let command_hits: Vec<usize> = COMMANDS
                    .iter()
                    .enumerate()
                    .filter(|(_, (key, label))| {
                        cmd_filter.is_empty()
                            || label.to_lowercase().contains(&cmd_filter)
                            || key.contains(&cmd_filter)
                    })
                    .map(|(idx, _)| idx)
                    .collect();
                let has_query = cmd_filter.chars().count() >= 2;
                // Borrow by index: the task store is never cloned per keystroke.
                let task_hits: Vec<usize> = if has_query {
                    self.store
                        .items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| {
                            item.title.to_lowercase().contains(&cmd_filter)
                                || item.notes.to_lowercase().contains(&cmd_filter)
                                || item.tag.to_lowercase().contains(&cmd_filter)
                        })
                        .take(6)
                        .map(|(idx, _)| idx)
                        .collect()
                } else {
                    Vec::new()
                };
                let note_hits: Vec<String> = if has_query {
                    self.store
                        .daily_notes
                        .iter()
                        .filter(|(_, note)| note.to_lowercase().contains(&cmd_filter))
                        .take(4)
                        .map(|(key, _)| key.clone())
                        .collect()
                } else {
                    Vec::new()
                };

                let quick_row = quick.as_ref().map(|_| 0usize);
                let command_start = usize::from(quick_row.is_some());
                let command_end = command_start + command_hits.len();
                let task_start = command_end;
                let task_end = task_start + task_hits.len();
                let note_start = task_end;
                let note_end = note_start + note_hits.len();
                let row_count = note_end;

                if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                    self.palette_selection = Some(match self.palette_selection {
                        Some(i) if row_count > 0 => (i + 1).min(row_count - 1),
                        _ if row_count > 0 => 0,
                        _ => 0,
                    });
                }
                if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                    self.palette_selection = Some(match self.palette_selection {
                        Some(i) => i.saturating_sub(1),
                        None if row_count > 0 => row_count - 1,
                        None => 0,
                    });
                }
                if self.palette_selection.is_some_and(|i| i >= row_count) {
                    self.palette_selection = if row_count == 0 {
                        None
                    } else {
                        Some(row_count - 1)
                    };
                }

                let mut activated: Option<usize> = None;
                let mut row_cursor = 0usize;

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        // 1. Entrada rápida en lenguaje natural
                        if let Some(parsed) = quick.as_ref() {
                            let target_date = self.today.add_days(parsed.date_offset_days);
                            let time_str = match (
                                parsed.start_time.is_empty(),
                                parsed.end_time.is_empty(),
                            ) {
                                (false, false) => {
                                    format!("{}-{}", parsed.start_time, parsed.end_time)
                                }
                                (false, true) => parsed.start_time.clone(),
                                _ => "Todo el día".to_owned(),
                            };
                            let label = format!(
                                "+ Crear \"{}\" · {:02}/{} · {} · #{}",
                                parsed.title,
                                target_date.day,
                                target_date.month,
                                time_str,
                                parsed.tag
                            );
                            let row = row_cursor;
                            row_cursor += 1;
                            let is_selected = self.palette_selection == Some(row);
                            let btn = ui.add(
                                egui::Button::new(
                                    RichText::new(label).size(12.5).strong().color(p.accent),
                                )
                                .fill(if is_selected {
                                    p.bg_cell_selected
                                } else {
                                    p.accent_soft
                                })
                                .stroke(Stroke::new(1.0_f32, p.accent))
                                .min_size(Vec2::new(ui.available_width(), 34.0)),
                            );
                            if btn.clicked() {
                                activated = Some(row);
                            }
                            if is_selected {
                                btn.scroll_to_me(None);
                            }
                            ui.add_space(6.0);
                        }

                        // 2. Comandos
                        if !command_hits.is_empty() {
                            ui.label(
                                RichText::new("COMANDOS")
                                    .size(10.5)
                                    .strong()
                                    .color(p.ink_muted),
                            );
                            for &cmd_idx in &command_hits {
                                let Some(&(_, label)) = COMMANDS.get(cmd_idx) else {
                                    continue;
                                };
                                let row = row_cursor;
                                row_cursor += 1;
                                let is_selected = self.palette_selection == Some(row);
                                let resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(label).size(12.5).color(p.ink_primary),
                                    )
                                    .fill(if is_selected {
                                        p.accent_soft
                                    } else {
                                        p.bg_elevated
                                    })
                                    .min_size(Vec2::new(ui.available_width(), 28.0)),
                                );
                                if resp.clicked() {
                                    activated = Some(row);
                                }
                                if is_selected {
                                    resp.scroll_to_me(None);
                                }
                            }
                        }

                        // 3. Búsqueda en tareas y notas (solo índices, sin clonar)
                        if !task_hits.is_empty() {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("TAREAS Y EVENTOS")
                                    .size(10.5)
                                    .strong()
                                    .color(p.ink_muted),
                            );
                            for &item_idx in &task_hits {
                                let Some(item) = self.store.items.get(item_idx) else {
                                    continue;
                                };
                                let d = Date::new(
                                    item.start_date.year(),
                                    item.start_date.month() as u8,
                                    item.start_date.day() as u8,
                                );
                                let text = format!(
                                    "{} · {:02}/{} ({})",
                                    views::truncate_chars(&item.title, 36),
                                    d.day,
                                    d.month,
                                    item.tag
                                );
                                let row = row_cursor;
                                row_cursor += 1;
                                let is_selected = self.palette_selection == Some(row);
                                let resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(text).size(12.0).color(p.ink_primary),
                                    )
                                    .fill(if is_selected {
                                        p.accent_soft
                                    } else {
                                        p.bg_canvas
                                    })
                                    .min_size(Vec2::new(ui.available_width(), 26.0)),
                                );
                                if resp.clicked() {
                                    activated = Some(row);
                                }
                                if is_selected {
                                    resp.scroll_to_me(None);
                                }
                            }
                        }
                        if !note_hits.is_empty() {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("NOTAS")
                                    .size(10.5)
                                    .strong()
                                    .color(p.ink_muted),
                            );
                            for date_key in &note_hits {
                                let Ok(nd) =
                                    NaiveDate::parse_from_str(date_key, "%Y-%m-%d")
                                else {
                                    continue;
                                };
                                let d =
                                    Date::new(nd.year(), nd.month() as u8, nd.day() as u8);
                                let preview = self
                                    .store
                                    .daily_notes
                                    .get(date_key)
                                    .map(|note| views::truncate_chars(note.trim(), 42))
                                    .unwrap_or_default();
                                let row = row_cursor;
                                row_cursor += 1;
                                let is_selected = self.palette_selection == Some(row);
                                let resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(format!(
                                            "{:02}/{}: {}",
                                            d.day, d.month, preview
                                        ))
                                        .size(12.0)
                                        .color(p.ink_secondary),
                                    )
                                    .fill(if is_selected {
                                        p.accent_soft
                                    } else {
                                        p.bg_canvas
                                    })
                                    .min_size(Vec2::new(ui.available_width(), 26.0)),
                                );
                                if resp.clicked() {
                                    activated = Some(row);
                                }
                                if is_selected {
                                    resp.scroll_to_me(None);
                                }
                            }
                        }
                    });

                let execute_row = activated.or_else(|| {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        match self.palette_selection {
                            Some(row) => Some(row),
                            // Creating with Enter is only allowed with explicit text and no
                            // active task/note result; otherwise the user must select a row.
                            None if quick.is_some()
                                && task_hits.is_empty()
                                && note_hits.is_empty() =>
                            {
                                Some(0)
                            }
                            None => None,
                        }
                    } else {
                        None
                    }
                });

                if let Some(row) = execute_row {
                    if quick_row == Some(row) {
                        if let Some(parsed) = quick.as_ref() {
                            let target_date = self.today.add_days(parsed.date_offset_days);
                            if self.store.add_task(
                                target_date,
                                TaskDraft {
                                    title: &parsed.title,
                                    notes: "",
                                    time: &parsed.start_time,
                                    end_time: &parsed.end_time,
                                    location: "",
                                    tag: &parsed.tag,
                                    span_days: 1,
                                    recurrence: Recurrence::None,
                                    color: None,
                                    priority: parsed.priority,
                                },
                            ) {
                                self.select_date(target_date);
                                self.persist();
                            }
                            close_palette = true;
                        }
                    } else if row >= command_start && row < command_end {
                        if let Some(&cmd_idx) = command_hits.get(row - command_start) {
                            if let Some(&(key, _)) = COMMANDS.get(cmd_idx) {
                                match key {
                                    "hoy" => {
                                        self.today = Date::today();
                                        self.select_date(self.today);
                                    }
                                    "guia" => {
                                        self.tour_step = 0;
                                        self.show_tour = true;
                                    }
                                    "notificacion" => {
                                        self.today = Date::today();
                                        self.send_daily_notification_now();
                                    }
                                    "tema" => {
                                        self.dark_mode = !self.dark_mode;
                                        self.store.dark_mode = self.dark_mode;
                                        self.persist();
                                    }
                                    "paleta" => {
                                        let idx = ThemePreset::ALL
                                            .iter()
                                            .position(|p| *p == self.store.theme_preset)
                                            .unwrap_or(0);
                                        self.store.theme_preset = ThemePreset::ALL
                                            .get((idx + 1) % ThemePreset::ALL.len())
                                            .copied()
                                            .unwrap_or(ThemePreset::Indigo);
                                        self.persist();
                                    }
                                    "enfoque" => self.focus_mode = !self.focus_mode,
                                    "markdown" => {
                                        let ws = self.selected_date.week_monday();
                                        let wn = ws.iso_week();
                                        let md =
                                            export::export_week_markdown(&self.store, ws, wn);
                                        self.export_to_desktop(
                                            &format!("Semana_{wn}_Resumen.md"),
                                            &md,
                                        );
                                    }
                                    "ics" => {
                                        let ics = export::export_ics(&self.store);
                                        self.export_to_desktop("Mi_Agenda.ics", &ics);
                                    }
                                    _ => {}
                                }
                                close_palette = true;
                            }
                        }
                    } else if row >= task_start && row < task_end {
                        let target = task_hits.get(row - task_start).and_then(|&item_idx| {
                            self.store.items.get(item_idx).map(|item| {
                                Date::new(
                                    item.start_date.year(),
                                    item.start_date.month() as u8,
                                    item.start_date.day() as u8,
                                )
                            })
                        });
                        if let Some(d) = target {
                            self.select_date(d);
                            self.active_view = ActiveView::Daily;
                            close_palette = true;
                        }
                    } else if row >= note_start && row < note_end {
                        let key = note_hits.get(row - note_start);
                        if let Some(key) = key {
                            if let Ok(nd) = NaiveDate::parse_from_str(key, "%Y-%m-%d") {
                                let d =
                                    Date::new(nd.year(), nd.month() as u8, nd.day() as u8);
                                self.select_date(d);
                                self.active_view = ActiveView::Daily;
                                close_palette = true;
                            }
                        }
                    }
                }
            });

        if close_palette {
            self.show_command_palette = false;
            self.palette_selection = None;
        }
    }

    fn render_focus_workspace(&mut self, ui: &mut egui::Ui, p: Palette) {
        let occs = self.store.tasks_visible_on(self.selected_date);
        let active_valid = self
            .focus_timer
            .active_item_id
            .is_some_and(|id| occs.iter().any(|o| o.task.id == id));
        if !active_valid {
            self.focus_timer.active_item_id = occs
                .iter()
                .find(|o| !o.task.done && !o.task.time.is_empty())
                .or_else(|| occs.iter().find(|o| !o.task.done))
                .or_else(|| occs.first())
                .map(|o| o.task.id);
        }

        let selected_occ = self
            .focus_timer
            .active_item_id
            .and_then(|id| occs.iter().find(|o| o.task.id == id))
            .cloned();

        ui.vertical_centered(|ui| {
            ui.set_max_width(640.0);
            ui.add_space(Spacing::MD);

            // Cabecera de Modo Enfoque
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("MODO ENFOQUE")
                        .size(12.0)
                        .strong()
                        .color(p.accent),
                );
                ui.label(
                    RichText::new(format!(
                        "· {:02} {}",
                        self.selected_date.day,
                        month_name_es(self.selected_date.month)
                    ))
                    .size(12.0)
                    .color(p.ink_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Salir (Esc)").size(12.0).color(p.ink_primary),
                            )
                            .fill(p.bg_elevated)
                            .stroke(Stroke::new(1.0_f32, p.border)),
                        )
                        .clicked()
                    {
                        self.focus_mode = false;
                    }
                });
            });
            ui.add_space(Spacing::MD);

            // Tarjeta del temporizador Pomodoro / Deep Work
            egui::Frame::none()
                .fill(p.bg_surface)
                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(24.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let phase_text = if self.focus_timer.is_break() {
                            "DESCANSO"
                        } else if self.focus_timer.is_running() {
                            "EN CURSO"
                        } else {
                            "LISTO PARA ENFOCAR"
                        };
                        ui.label(
                            RichText::new(phase_text)
                                .size(11.5)
                                .strong()
                                .color(p.accent),
                        );
                        ui.add_space(6.0);

                        let secs = self.focus_timer.display_secs();
                        ui.label(
                            RichText::new(format!("{:02}:{:02}", secs / 60, secs % 60))
                                .size(54.0)
                                .strong()
                                .monospace()
                                .color(p.ink_primary),
                        );
                        ui.add_space(10.0);

                        // Selector de preset (25/5, 50/10, Restante del bloque)
                        let block_secs = selected_occ
                            .as_ref()
                            .and_then(|o| {
                                let end_t =
                                    NaiveTime::parse_from_str(&o.task.end_time, "%H:%M").ok()?;
                                let end = end_t.hour() * 60 + end_t.minute();
                                let now = chrono::Local::now().time();
                                let now_min = now.hour() * 60 + now.minute();
                                if self.selected_date == self.today && end > now_min {
                                    Some(((end - now_min) as u64) * 60)
                                } else {
                                    NaiveTime::parse_from_str(&o.task.time, "%H:%M")
                                        .ok()
                                        .map(|s| s.hour() * 60 + s.minute())
                                        .filter(|s| end > *s)
                                        .map(|s| ((end - s) as u64) * 60)
                                }
                            })
                            .unwrap_or(15 * 60);

                        ui.horizontal(|ui| {
                            for preset in [
                                FocusPreset::Pomodoro25,
                                FocusPreset::DeepWork50,
                                FocusPreset::BlockRemaining(block_secs),
                            ] {
                                let active = std::mem::discriminant(&self.focus_timer.preset)
                                    == std::mem::discriminant(&preset);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(preset.label()).size(11.5).color(
                                                if active { p.accent } else { p.ink_secondary },
                                            ),
                                        )
                                        .fill(if active {
                                            p.accent_soft
                                        } else {
                                            p.bg_elevated
                                        }),
                                    )
                                    .clicked()
                                {
                                    self.focus_timer.set_preset(preset);
                                }
                            }
                        });
                        ui.add_space(14.0);

                        ui.horizontal(|ui| {
                            let primary_label = if self.focus_timer.is_running() {
                                "Pausar"
                            } else {
                                "Iniciar"
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(primary_label)
                                            .size(13.5)
                                            .strong()
                                            .color(p.on_accent),
                                    )
                                    .fill(p.accent)
                                    .min_size(Vec2::new(110.0, 36.0)),
                                )
                                .clicked()
                            {
                                if self.focus_timer.is_running() {
                                    self.focus_timer.pause();
                                } else {
                                    self.focus_timer.start_or_resume();
                                }
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Reiniciar").size(12.5).color(p.ink_primary),
                                    )
                                    .fill(p.bg_elevated)
                                    .min_size(Vec2::new(90.0, 36.0)),
                                )
                                .clicked()
                            {
                                self.focus_timer.reset();
                            }
                            if let Some(occ) = &selected_occ {
                                let done_label = if occ.task.done {
                                    "Reabrir"
                                } else {
                                    "Completar bloque"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(done_label)
                                                .size(12.5)
                                                .color(p.ink_primary),
                                        )
                                        .fill(p.bg_elevated)
                                        .min_size(Vec2::new(125.0, 36.0)),
                                    )
                                    .clicked()
                                {
                                    let id = occ.task.id;
                                    if self.store.toggle_task(self.selected_date, id) {
                                        self.persist();
                                    }
                                }
                            }
                        });
                    });
                });

            ui.add_space(Spacing::MD);

            // Selector de bloque + Notas de sesión
            egui::Frame::none()
                .fill(p.bg_surface)
                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("BLOQUE ACTIVO")
                            .size(11.0)
                            .strong()
                            .color(p.ink_muted),
                    );
                    ui.add_space(6.0);
                    if occs.is_empty() {
                        ui.label(
                            RichText::new("No hay bloques programados para este día.")
                                .size(12.5)
                                .color(p.ink_secondary),
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for occ in &occs {
                                let is_sel = self.focus_timer.active_item_id == Some(occ.task.id);
                                let lbl = if occ.task.time.is_empty() {
                                    views::truncate_chars(&occ.task.title, 28)
                                } else {
                                    format!(
                                        "{} {}",
                                        occ.task.time,
                                        views::truncate_chars(&occ.task.title, 22)
                                    )
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(lbl).size(11.5).color(if is_sel {
                                                p.accent
                                            } else {
                                                p.ink_primary
                                            }),
                                        )
                                        .fill(if is_sel {
                                            p.accent_soft
                                        } else {
                                            p.bg_elevated
                                        }),
                                    )
                                    .clicked()
                                {
                                    self.focus_timer.active_item_id = Some(occ.task.id);
                                }
                            }
                        });
                    }

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("NOTAS DE LA SESIÓN")
                            .size(11.0)
                            .strong()
                            .color(p.ink_muted),
                    );
                    ui.add_space(4.0);
                    let mut changed = false;
                    if let Some(active_id) = self.focus_timer.active_item_id {
                        if let Some(item) = self.store.items.iter_mut().find(|i| i.id == active_id)
                        {
                            changed = ui
                                .add_sized(
                                    Vec2::new(ui.available_width(), 90.0),
                                    egui::TextEdit::multiline(&mut item.notes)
                                        .hint_text("Apunta avances o ideas durante el enfoque..."),
                                )
                                .changed();
                        }
                    } else {
                        let key = self.selected_date.to_key();
                        let entry = self.store.daily_notes.entry(key).or_default();
                        changed = ui
                            .add_sized(
                                Vec2::new(ui.available_width(), 90.0),
                                egui::TextEdit::multiline(entry)
                                    .hint_text("Notas rápidas del día..."),
                            )
                            .changed();
                    }
                    if changed {
                        self.mark_notes_dirty();
                    }
                });
        });
    }
}

impl eframe::App for AgendaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if crate::tray::START_HIDDEN.swap(false, std::sync::atomic::Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if ctx.input(|i| i.viewport().close_requested())
            && !crate::tray::QUIT_REQUESTED.load(std::sync::atomic::Ordering::SeqCst)
        {
            // Minimize to the tray instead of exiting unless "Salir" was chosen.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        let real_today = Date::today();
        if real_today != self.today {
            self.today = real_today;
            self.maybe_send_daily_notification();
        }
        self.poll_tray(ctx);
        if self.last_update_check.elapsed() >= Duration::from_secs(300)
            && !matches!(
                self.updater.status,
                UpdateStatus::Checking | UpdateStatus::Downloading(_)
            )
        {
            self.last_update_check = Instant::now();
            self.updater.check_for_updates();
        }
        if let Some(installer_path) = self.updater.poll() {
            if let Err(err) = updater::launch_installer(&installer_path) {
                self.status_error = Some(err);
            } else {
                crate::tray::QUIT_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        self.focus_timer.tick();
        if let Some(dirty_at) = self.notes_dirty_at {
            if dirty_at.elapsed() >= Duration::from_millis(600) {
                self.persist();
            } else {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }
        if matches!(
            self.updater.status,
            UpdateStatus::Checking | UpdateStatus::Downloading(_)
        ) {
            ctx.request_repaint_after(Duration::from_millis(150));
        } else if self.focus_timer.is_running() {
            // 1s tick ONLY while the focus timer is actively running (zero CPU when paused/idle)
            ctx.request_repaint_after(Duration::from_secs(1));
        } else {
            // Tray/menu events wake the UI via callbacks; idle only refreshes the live time line.
            ctx.request_repaint_after(Duration::from_secs(30));
        }
        let theme_key = (self.store.theme_preset, self.dark_mode);
        let p = match self.palette_cache {
            Some((preset, dark, cached)) if preset == theme_key.0 && dark == theme_key.1 => cached,
            _ => {
                let palette = apply_theme(ctx, theme_key.0, theme_key.1);
                self.palette_cache = Some((theme_key.0, theme_key.1, palette));
                palette
            }
        };
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("clarity_header")
            .frame(
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .inner_margin(egui::Margin::symmetric(18.0, 10.0)),
            )
            .show(ctx, |ui| {
                self.render_header(ui, p);
            });

        if !self.focus_mode {
            egui::SidePanel::left("clarity_sidebar")
                .resizable(false)
                .exact_width(220.0)
                .frame(
                    egui::Frame::none()
                        .fill(p.bg_canvas)
                        .stroke(Stroke::new(1.0_f32, p.border_subtle))
                        .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
                )
                .show(ctx, |ui| {
                    self.render_sidebar(ui, p);
                });
        }

        let central_resp = egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_canvas)
                    .inner_margin(egui::Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.focus_mode {
                            self.render_focus_workspace(ui, p);
                        } else {
                            match self.active_view {
                                ActiveView::Monthly => views::monthly::render(self, ui, p),
                                ActiveView::Daily => views::daily::render(self, ui, p),
                                ActiveView::Weekly => views::weekly::render(self, ui, p),
                            }
                        }
                    });
            });
        self.tour_anchors.calendar_area = Some(central_resp.response.rect);

        self.render_new_task_modal(ctx, p);
        self.render_command_palette(ctx, p);

        if self.show_tour {
            apply_step_view(self.tour_step, &mut self.active_view, &mut self.focus_mode);
            if matches!(
                render_tour_overlay(ctx, p, &mut self.tour_step, &self.tour_anchors),
                TourOutcome::CompletedOrSkipped
            ) {
                self.show_tour = false;
                self.store.onboarding_completed = true;
                self.persist();
            } else {
                apply_step_view(self.tour_step, &mut self.active_view, &mut self.focus_mode);
            }
        }

        if let Some(drag_id) = self.drag_backlog_id {
            if let Some(pos) = ctx.pointer_interact_pos() {
                egui::Area::new(egui::Id::new("backlog_drag_ghost"))
                    .fixed_pos(pos + Vec2::new(10.0, 10.0))
                    .interactable(false)
                    .show(ctx, |ui| {
                        let title = self
                            .store
                            .items
                            .iter()
                            .find(|i| i.id == drag_id)
                            .map(|i| i.title.as_str())
                            .unwrap_or("Tarea");
                        egui::Frame::none()
                            .fill(p.accent)
                            .rounding(Rounding::same(Radius::SM))
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(views::truncate_chars(title, 24))
                                        .size(11.5)
                                        .strong()
                                        .color(p.on_accent),
                                );
                            });
                    });
            }
            if !ctx.input(|i| i.pointer.primary_down()) {
                self.drag_backlog_id = None;
            }
        }
    }
}
