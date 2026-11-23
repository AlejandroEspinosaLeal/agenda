use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, Key, Layout, Pos2, RichText, Rounding, Sense, Stroke, Vec2, WindowLevel,
};

use crate::calendar::{days_in_month, month_name_es, shift_month, Date};
use crate::model::{Priority, TaskDraft, TaskStore};
use crate::theme::{apply_theme, category_colors, Palette};
use crate::tray::{TrayAction, TrayController};
use crate::updater::{self, UpdateStatus, Updater};
use crate::views::{self, COLOR_PRESETS, TAG_PRESETS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Monthly,
    Daily,
    Weekly,
}

pub struct AgendaApp {
    pub today: Date,
    pub view_year: i32,
    pub view_month: u8,
    pub selected_date: Date,
    pub active_view: ActiveView,
    pub focus_mode: bool,
    pub mini_mode: bool,
    pub show_taskbar_pill: bool,
    pub show_new_task_modal: bool,
    pub tray: Option<TrayController>,
    pub updater: Updater,
    pub store: TaskStore,
    pub storage_path: PathBuf,
    // Draft fields for creating tasks
    pub new_title: String,
    pub new_notes: String,
    pub new_time: String,
    pub new_end_time: String,
    pub new_location: String,
    pub new_tag: String,
    pub new_span_days: u8,
    pub new_color: Option<[u8; 3]>,
    pub custom_rgb: [u8; 3],
    pub new_priority: Priority,
    // Filters & theme
    pub search_query: String,
    pub category_filter: Option<String>,
    pub dark_mode: bool,
    pub status_error: Option<String>,
}

impl AgendaApp {
    pub fn new(start_in_tray: bool) -> Self {
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

        let app = Self {
            today,
            view_year: today.year,
            view_month: today.month,
            selected_date: today,
            active_view: ActiveView::Monthly,
            focus_mode: false,
            mini_mode: start_in_tray,
            show_taskbar_pill: true,
            show_new_task_modal: false,
            tray: None,
            updater,
            store,
            storage_path,
            new_title: String::new(),
            new_notes: String::new(),
            new_time: String::new(),
            new_end_time: String::new(),
            new_location: String::new(),
            new_tag: "Trabajo".to_owned(),
            new_span_days: 1,
            new_color: None,
            custom_rgb: [67, 56, 202],
            new_priority: Priority::Normal,
            search_query: String::new(),
            category_filter: None,
            dark_mode: false,
            status_error,
        };
        if let Some(t) = tray.as_mut() {
            let pending = app.pending_titles_today();
            t.sync_today(&pending);
        }
        Self { tray, ..app }
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

    pub fn select_date(&mut self, date: Date) {
        self.view_year = date.year;
        self.view_month = date.month;
        self.selected_date = date;
    }

    fn next_pending_today(&self) -> (usize, Option<crate::model::Task>) {
        let mut pending: Vec<_> = self
            .store
            .tasks_visible_on(self.today)
            .into_iter()
            .filter(|occ| !occ.task.done)
            .map(|occ| occ.task)
            .collect();
        let count = pending.len();
        pending.sort_by(|a, b| a.time.cmp(&b.time));
        (count, pending.into_iter().next())
    }

    fn render_taskbar_pill(&mut self, ctx: &egui::Context, p: Palette) {
        if !self.show_taskbar_pill {
            return;
        }
        let pill_size = Vec2::new(224.0, 34.0);
        let default_pos = ctx
            .input(|i| i.viewport().monitor_size)
            .map(|mon| Pos2::new((mon.x - pill_size.x - 240.0).max(0.0), (mon.y - 42.0).max(0.0)))
            .unwrap_or_else(|| Pos2::new(980.0, 680.0));

        let id = egui::ViewportId::from_hash_of("mi_agenda_taskbar_pill");
        let builder = egui::ViewportBuilder::default()
            .with_title("Mi Agenda Widget")
            .with_inner_size(pill_size)
            .with_position(default_pos)
            .with_decorations(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop);

        let (pending_count, next_task) = self.next_pending_today();

        let mut open_mini = false;
        ctx.show_viewport_immediate(id, builder, |ui_ctx, _class| {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
                .show(ui_ctx, |ui| {
                    let rect = ui.max_rect();
                    let resp = ui.interact(
                        rect,
                        ui.id().with("pill_drag_click"),
                        Sense::click_and_drag(),
                    );
                    if resp.drag_started() {
                        ui_ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if resp.clicked() {
                        open_mini = true;
                    }

                    // Sleek dark-frosted pill matching Windows 11 taskbar
                    let bg = if resp.hovered() {
                        Color32::from_rgb(42, 48, 60)
                    } else {
                        Color32::from_rgb(28, 32, 40)
                    };
                    let border = if pending_count > 0 {
                        p.accent
                    } else {
                        Color32::from_rgb(62, 70, 86)
                    };

                    let painter = ui.painter();
                    painter.rect_filled(rect.shrink(1.0), Rounding::same(10.0), bg);
                    painter.rect_stroke(
                        rect.shrink(1.0),
                        Rounding::same(10.0),
                        Stroke::new(1.0_f32, border),
                    );

                    let dot_color = match &next_task {
                        Some(task) => {
                            views::task_colors(task.color, &task.tag, task.priority, p, true).0
                        }
                        None => p.done_green,
                    };
                    painter.circle_filled(
                        egui::pos2(rect.min.x + 15.0, rect.center().y),
                        4.5,
                        dot_color,
                    );

                    let label = match &next_task {
                        Some(task) => {
                            let base = if task.time.is_empty() {
                                task.title.clone()
                            } else {
                                format!("{} · {}", task.time, task.title)
                            };
                            views::truncate_chars(&base, 22)
                        }
                        None => "Sin tareas hoy".to_owned(),
                    };
                    painter.text(
                        egui::pos2(rect.min.x + 26.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(244, 246, 250),
                    );

                    if pending_count > 1 {
                        let badge_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.max.x - 32.0, rect.center().y - 9.0),
                            Vec2::new(24.0, 18.0),
                        );
                        painter.rect_filled(badge_rect, Rounding::same(9.0), p.accent);
                        painter.text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("+{pending_count}"),
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                    }
                });
        });

        if open_mini {
            self.today = Date::today();
            self.select_date(self.today);
            self.set_mini_mode(ctx, true);
        }
    }

    pub fn set_mini_mode(&mut self, ctx: &egui::Context, mini: bool) {
        self.mini_mode = mini;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        if mini {
            let size = Vec2::new(360.0, 520.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop));
            if let Some(mon) = ctx.input(|i| i.viewport().monitor_size) {
                let x = (mon.x - size.x - 16.0).max(0.0);
                let y = (mon.y - size.y - 68.0).max(0.0);
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(x, y)));
            }
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(WindowLevel::Normal));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1360.0, 840.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(80.0, 40.0)));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        let Some(action) = self.tray.as_ref().and_then(TrayController::poll_action) else {
            return;
        };
        match action {
            TrayAction::OpenMini => {
                self.today = Date::today();
                self.select_date(self.today);
                self.set_mini_mode(ctx, true);
            }
            TrayAction::OpenFull => {
                self.set_mini_mode(ctx, false);
            }
            TrayAction::Quit => {
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
        let added = self.store.add_task(
            date,
            TaskDraft {
                title: &self.new_title,
                notes: &self.new_notes,
                time: &self.new_time,
                end_time: &self.new_end_time,
                location: &self.new_location,
                tag: &self.new_tag,
                span_days: self.new_span_days,
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
            if self.show_new_task_modal {
                self.show_new_task_modal = false;
                return;
            }
            if !self.search_query.is_empty() {
                self.search_query.clear();
                return;
            }
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            match self.active_view {
                ActiveView::Monthly => self.change_month(-1),
                ActiveView::Daily => self.select_date(self.selected_date.add_days(-1)),
                ActiveView::Weekly => self.select_date(self.selected_date.add_days(-7)),
            }
        } else if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            match self.active_view {
                ActiveView::Monthly => self.change_month(1),
                ActiveView::Daily => self.select_date(self.selected_date.add_days(1)),
                ActiveView::Weekly => self.select_date(self.selected_date.add_days(7)),
            }
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui, p: Palette) {
        let focus_search =
            ui.input(|i| i.modifiers.ctrl && (i.key_pressed(Key::K) || i.key_pressed(Key::F)));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;

            // Brand mark
            egui::Frame::none()
                .fill(p.accent)
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("MA").size(13.0).strong().color(Color32::WHITE));
                });
            ui.label(
                RichText::new("Mi Agenda")
                    .size(18.0)
                    .strong()
                    .color(p.ink_primary),
            );

            ui.add_space(16.0);

            // Global Search Bar
            let search_w = (ui.available_width() - 420.0).clamp(180.0, 420.0);
            let search_resp = ui.add_sized(
                Vec2::new(search_w, 34.0),
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Buscar tareas, notas o eventos... (Ctrl+K)")
                    .vertical_align(Align::Center),
            );
            if focus_search {
                search_resp.request_focus();
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
                let theme_label = if self.dark_mode { "Claro" } else { "Oscuro" };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(theme_label)
                                .size(12.5)
                                .strong()
                                .color(p.ink_primary),
                        )
                        .fill(p.bg_elevated)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .min_size(Vec2::new(72.0, 34.0)),
                    )
                    .on_hover_text("Cambiar tema claro / oscuro")
                    .clicked()
                {
                    self.dark_mode = !self.dark_mode;
                }

                let focus_label = if self.focus_mode {
                    "Salir de foco"
                } else {
                    "En foco"
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(focus_label)
                                .size(12.5)
                                .strong()
                                .color(if self.focus_mode {
                                    p.accent
                                } else {
                                    p.ink_primary
                                }),
                        )
                        .fill(if self.focus_mode {
                            p.accent_soft
                        } else {
                            p.bg_elevated
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if self.focus_mode { p.accent } else { p.border },
                        ))
                        .min_size(Vec2::new(86.0, 34.0)),
                    )
                    .on_hover_text("Oculta barras laterales para concentrarte")
                    .clicked()
                {
                    self.focus_mode = !self.focus_mode;
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Mini")
                                .size(12.5)
                                .strong()
                                .color(p.ink_primary),
                        )
                        .fill(p.bg_elevated)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .min_size(Vec2::new(56.0, 34.0)),
                    )
                    .on_hover_text("Cambiar a Mini-Calendario junto al reloj")
                    .clicked()
                {
                    self.set_mini_mode(ui.ctx(), true);
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("+ Nueva Tarea")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(p.accent)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(124.0, 34.0)),
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
            ui.add_space(6.0);
            ui.label(
                RichText::new("PLANIFICACIÓN")
                    .size(11.0)
                    .strong()
                    .color(p.ink_muted),
            );
            ui.add_space(6.0);

            let items = [
                (ActiveView::Monthly, "Calendario Mensual"),
                (ActiveView::Daily, "Agenda Diaria"),
                (ActiveView::Weekly, "Vista Semanal"),
            ];

            for (view, label) in items {
                let active = self.active_view == view;
                let btn = egui::Button::new(
                    RichText::new(label)
                        .size(13.5)
                        .strong()
                        .color(if active { Color32::WHITE } else { p.ink_primary }),
                )
                .fill(if active { p.accent } else { Color32::TRANSPARENT })
                .stroke(Stroke::NONE)
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(ui.available_width(), 38.0));

                if ui.add(btn).clicked() {
                    self.active_view = view;
                }
                ui.add_space(2.0);
            }

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                // Updater status & button at the very bottom of sidebar
                match &self.updater.status {
                    UpdateStatus::Available(info) => {
                        let info_clone = info.clone();
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("Actualizar a {}", info.version))
                                        .size(11.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(p.done_green)
                                .min_size(Vec2::new(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            self.updater.start_download(info_clone);
                        }
                    }
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

                let (pending, done) = self.store.counts_for(self.today);
                let total = pending + done;
                let pct = if total == 0 {
                    0.0
                } else {
                    done as f32 / total as f32
                };

                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(Stroke::new(1.0_f32, p.border_subtle))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Productividad Hoy")
                                        .size(12.0)
                                        .strong()
                                        .color(p.ink_primary),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{}%", (pct * 100.0).round() as u8))
                                            .size(12.0)
                                            .strong()
                                            .color(p.accent),
                                    );
                                });
                            });
                            ui.add_space(6.0);
                            let w = ui.available_width();
                            let (bar_rect, _) =
                                ui.allocate_exact_size(Vec2::new(w, 6.0), Sense::hover());
                            ui.painter()
                                .rect_filled(bar_rect, Rounding::same(3.0), p.bg_elevated);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(bar_rect.min, Vec2::new(w * pct, 6.0)),
                                Rounding::same(3.0),
                                p.accent,
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!("{done} de {total} bloques completados"))
                                    .size(11.0)
                                    .color(p.ink_muted),
                            );
                        });
                    });
            });
        });
    }

    fn render_new_task_modal(&mut self, ctx: &egui::Context, p: Palette) {
        if !self.show_new_task_modal {
            return;
        }
        let mut open = self.show_new_task_modal;
        let mut close_on_submit = false;
        let date = self.selected_date;

        egui::Window::new(format!(
            "Nueva Tarea · {} de {}",
            date.day,
            month_name_es(date.month)
        ))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Título de la tarea")
                    .size(12.0)
                    .color(p.ink_secondary),
            );
            let title_resp = ui.add(
                egui::TextEdit::singleline(&mut self.new_title)
                    .hint_text("ej. Comprar billetes de tren")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Inicio").size(11.5).color(p.ink_secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_time)
                            .hint_text("09:30")
                            .desired_width(82.0),
                    );
                });
                ui.vertical(|ui| {
                    ui.label(RichText::new("Fin").size(11.5).color(p.ink_secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_end_time)
                            .hint_text("10:30")
                            .desired_width(82.0),
                    );
                });
                ui.vertical(|ui| {
                    ui.label(RichText::new("Lugar / Etiqueta").size(11.5).color(p.ink_secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_location)
                            .hint_text("Opcional...")
                            .desired_width(f32::INFINITY),
                    );
                });
            });

            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.new_notes)
                    .hint_text("Nota o descripción breve (opcional)")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(8.0);
            ui.label(RichText::new("Duración").size(11.5).color(p.ink_secondary));
            ui.horizontal(|ui| {
                for (days, label) in [(1u8, "1 día"), (3u8, "3 días"), (7u8, "1 semana")] {
                    let selected = self.new_span_days == days;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(11.5)
                            .color(if selected { Color32::WHITE } else { p.ink_primary }),
                    )
                    .fill(if selected { p.accent } else { p.bg_elevated });
                    if ui.add(btn).clicked() {
                        self.new_span_days = days;
                    }
                }
            });

            ui.add_space(6.0);
            ui.label(RichText::new("Categoría").size(11.5).color(p.ink_secondary));
            ui.horizontal_wrapped(|ui| {
                for cat in TAG_PRESETS {
                    let selected = self.new_tag == cat;
                    let (fg, bg) = category_colors(cat, self.dark_mode);
                    let btn = egui::Button::new(
                        RichText::new(cat)
                            .size(11.5)
                            .strong()
                            .color(if selected { Color32::WHITE } else { fg }),
                    )
                    .fill(if selected { fg } else { bg });
                    if ui.add(btn).clicked() {
                        self.new_tag = if selected { String::new() } else { cat.to_owned() };
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Prioridad:").size(11.5).color(p.ink_secondary));
                for prio in [Priority::Low, Priority::Normal, Priority::High] {
                    let selected = self.new_priority == prio;
                    let (fg, bg) = views::priority_colors(prio, p);
                    let btn = egui::Button::new(
                        RichText::new(prio.label_es())
                            .size(11.5)
                            .color(if selected { fg } else { p.ink_secondary }),
                    )
                    .fill(if selected { bg } else { p.bg_elevated })
                    .stroke(Stroke::new(
                        if selected { 1.5_f32 } else { 1.0_f32 },
                        if selected { fg } else { p.border_subtle },
                    ));
                    if ui.add(btn).clicked() {
                        self.new_priority = prio;
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Color:").size(11.5).color(p.ink_secondary));
                let auto = self.new_color.is_none();
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Auto")
                                .size(11.0)
                                .color(if auto { p.accent } else { p.ink_secondary }),
                        )
                        .fill(if auto { p.accent_soft } else { p.bg_elevated }),
                    )
                    .clicked()
                {
                    self.new_color = None;
                }
                for preset in COLOR_PRESETS {
                    let selected = self.new_color == Some(preset);
                    let c = Color32::from_rgb(preset[0], preset[1], preset[2]);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                    ui.painter().circle_filled(rect.center(), 7.0, c);
                    if selected {
                        ui.painter().circle_stroke(
                            rect.center(),
                            8.5,
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

            ui.add_space(10.0);
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

            if submit && self.submit_draft(date) {
                close_on_submit = true;
            }
        });

        self.show_new_task_modal = open && !close_on_submit;
    }
}

impl eframe::App for AgendaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_tray(ctx);
        if let Some(installer_path) = self.updater.poll() {
            if let Err(err) = updater::launch_installer(&installer_path) {
                self.status_error = Some(err);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        if self.tray.is_some()
            || matches!(
                self.updater.status,
                UpdateStatus::Checking | UpdateStatus::Downloading(_)
            )
        {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
        let p = apply_theme(ctx, self.dark_mode);
        self.handle_shortcuts(ctx);

        if self.mini_mode {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(p.bg_canvas)
                        .inner_margin(egui::Margin::symmetric(12.0, 12.0)),
                )
                .show(ctx, |ui| {
                    views::mini::render(self, ctx, ui, p);
                });
            self.render_taskbar_pill(ctx, p);
            return;
        }

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

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_canvas)
                    .inner_margin(egui::Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_view {
                        ActiveView::Monthly => views::monthly::render(self, ui, p),
                        ActiveView::Daily => views::daily::render(self, ui, p),
                        ActiveView::Weekly => views::weekly::render(self, ui, p),
                    });
            });

        self.render_new_task_modal(ctx, p);
        self.render_taskbar_pill(ctx, p);
    }
}
