use eframe::egui::{self, Color32, Key, Order, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

use crate::app::ActiveView;
use crate::theme::{Palette, Radius};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourTarget {
    CalendarArea,
    ViewsNav,
    Backlog,
    CommandPalette,
    FocusMode,
    ThemeAndHelp,
}

pub struct TourStep {
    pub target: TourTarget,
    pub view: Option<ActiveView>,
    pub title: &'static str,
    pub body: &'static str,
}

pub const TOUR_STEPS: [TourStep; 8] = [
    TourStep {
        target: TourTarget::CalendarArea,
        view: Some(ActiveView::Monthly),
        title: "1. Calendario Mensual y rango multi-día",
        body: "Haz clic en un día para ver su panel lateral, doble clic para abrirlo en Diaria, o arrastra sobre varios días seguidos para crear una tarea multi-día.",
    },
    TourStep {
        target: TourTarget::CalendarArea,
        view: Some(ActiveView::Weekly),
        title: "2. Vista Semanal (Horario y Tablero)",
        body: "Alterna arriba entre Modo Horario y Tablero. En Modo Horario puedes arrastrar para crear bloques, mover tareas entre días y estirar su duración desde el borde inferior.",
    },
    TourStep {
        target: TourTarget::CalendarArea,
        view: Some(ActiveView::Daily),
        title: "3. Agenda Diaria y Time-Blocking",
        body: "Organiza tu día hora a hora (06:00–23:00 o 24h) con línea de hora actual en vivo y columnas paralelas automáticas si se solapan bloques.",
    },
    TourStep {
        target: TourTarget::ViewsNav,
        view: None,
        title: "4. Cambio rápido de vistas",
        body: "Usa estos botones o los atajos de teclado M (Mensual), W (Semanal), D (Diaria) y T (Hoy) para moverte al instante.",
    },
    TourStep {
        target: TourTarget::Backlog,
        view: Some(ActiveView::Daily),
        title: "5. Bandeja Sin Programar (Backlog)",
        body: "Anota aquí tareas sin hora fija y arrástralas directamente sobre la rejilla horaria de Diaria o Semanal para programarlas.",
    },
    TourStep {
        target: TourTarget::CommandPalette,
        view: None,
        title: "6. Paleta rápida (Ctrl+K)",
        body: "Pulsa Ctrl+K para buscar tareas o notas, ejecutar comandos o crear bloques con lenguaje natural (ej. \"Estudiar Rust 18:00-19:30 #Estudio\").",
    },
    TourStep {
        target: TourTarget::FocusMode,
        view: None,
        title: "7. Modo Enfoque y Pomodoro (F)",
        body: "Pulsa En foco (o F) para ocultar distracciones, centrar tu bloque actual y usar el temporizador 25/5, 50/10 o tiempo restante del bloque.",
    },
    TourStep {
        target: TourTarget::ThemeAndHelp,
        view: Some(ActiveView::Monthly),
        title: "8. Tema, notificaciones y ayuda (?)",
        body: "Cambia entre modo claro/oscuro, recibe una notificación diaria en Windows con tus pendientes y repite esta guía cuando quieras con el botón \"?\".",
    },
];

pub fn apply_step_view(step_idx: usize, active_view: &mut ActiveView, focus_mode: &mut bool) {
    *focus_mode = false;
    if let Some(step) = TOUR_STEPS.get(step_idx) {
        if let Some(v) = step.view {
            *active_view = v;
        }
    }
}

/// Lightweight UI bounding boxes captured during layout for spotlighting.
#[derive(Default, Clone, Copy)]
pub struct TourAnchors {
    pub calendar_area: Option<Rect>,
    pub views_nav: Option<Rect>,
    pub backlog: Option<Rect>,
    pub command_palette: Option<Rect>,
    pub focus_mode: Option<Rect>,
    pub theme_and_help: Option<Rect>,
}

impl TourAnchors {
    pub fn rect_for(&self, target: TourTarget) -> Option<Rect> {
        match target {
            TourTarget::CalendarArea => self.calendar_area,
            TourTarget::ViewsNav => self.views_nav,
            TourTarget::Backlog => self.backlog,
            TourTarget::CommandPalette => self.command_palette,
            TourTarget::FocusMode => self.focus_mode,
            TourTarget::ThemeAndHelp => self.theme_and_help,
        }
    }
}

/// Computes a clamped callout position next to `spotlight` inside `screen`.
pub fn callout_pos_for_target(spotlight: Rect, screen: Rect, card_size: Vec2) -> Pos2 {
    let margin = 16.0_f32;
    let mut x = if spotlight.right() + margin + card_size.x <= screen.right() - margin {
        spotlight.right() + margin
    } else if spotlight.left() - margin - card_size.x >= screen.left() + margin {
        spotlight.left() - margin - card_size.x
    } else {
        spotlight.center().x - card_size.x * 0.5
    };

    let mut y = if spotlight.bottom() + margin + card_size.y <= screen.bottom() - margin {
        if spotlight.width() > screen.width() * 0.5 {
            spotlight.center().y - card_size.y * 0.5
        } else {
            spotlight.top()
        }
    } else {
        spotlight.top() - margin - card_size.y
    };

    x = x.clamp(screen.left() + margin, (screen.right() - card_size.x - margin).max(margin));
    y = y.clamp(screen.top() + margin, (screen.bottom() - card_size.y - margin).max(margin));
    Pos2::new(x, y)
}

pub enum TourOutcome {
    None,
    CompletedOrSkipped,
}

pub fn render_tour_overlay(
    ctx: &egui::Context,
    p: Palette,
    step_idx: &mut usize,
    anchors: &TourAnchors,
) -> TourOutcome {
    let total = TOUR_STEPS.len();
    if *step_idx >= total {
        *step_idx = 0;
    }
    let step = &TOUR_STEPS[*step_idx];
    let screen = ctx.screen_rect();
    let spotlight = anchors
        .rect_for(step.target)
        .unwrap_or_else(|| Rect::from_center_size(screen.center(), Vec2::new(260.0, 120.0)))
        .expand(6.0)
        .intersect(screen);

    let mut outcome = TourOutcome::None;
    let mut next_pressed = false;
    let mut prev_pressed = false;

    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            outcome = TourOutcome::CompletedOrSkipped;
        } else if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Enter) {
            next_pressed = true;
        } else if i.key_pressed(Key::ArrowLeft) {
            prev_pressed = true;
        }
    });

    // 4-rect spotlight cutout so the highlighted UI stays crisp and unobscured
    egui::Area::new(egui::Id::new("onboarding_tour_backdrop"))
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let painter = ui.painter();
            let dim = Color32::from_black_alpha(150);

            // Top band
            if spotlight.top() > screen.top() {
                painter.rect_filled(
                    Rect::from_min_max(screen.min, Pos2::new(screen.max.x, spotlight.top())),
                    0.0,
                    dim,
                );
            }
            // Bottom band
            if spotlight.bottom() < screen.max.y {
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(screen.min.x, spotlight.bottom()), screen.max),
                    0.0,
                    dim,
                );
            }
            // Left band
            if spotlight.left() > screen.left() {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(screen.min.x, spotlight.top()),
                        Pos2::new(spotlight.left(), spotlight.bottom()),
                    ),
                    0.0,
                    dim,
                );
            }
            // Right band
            if spotlight.right() < screen.max.x {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(spotlight.right(), spotlight.top()),
                        Pos2::new(screen.max.x, spotlight.bottom()),
                    ),
                    0.0,
                    dim,
                );
            }

            // Highlight ring around target
            painter.rect_stroke(
                spotlight,
                Rounding::same(Radius::MD),
                Stroke::new(2.5_f32, p.accent),
            );
        });

    let card_size = Vec2::new(330.0, 178.0);
    let card_pos = callout_pos_for_target(spotlight, screen, card_size);

    egui::Area::new(egui::Id::new("onboarding_tour_card"))
        .order(Order::Tooltip)
        .fixed_pos(card_pos)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(p.bg_surface)
                .stroke(Stroke::new(1.5_f32, p.accent))
                .rounding(Rounding::same(Radius::LG))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_width(card_size.x - 32.0);
                    ui.horizontal(|ui| {
                        egui::Frame::none()
                            .fill(p.accent_soft)
                            .rounding(Rounding::same(Radius::SM))
                            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("Paso {} de {}", *step_idx + 1, total))
                                        .size(11.0)
                                        .strong()
                                        .color(p.accent),
                                );
                            });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Saltar").size(11.5).color(p.ink_muted),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                outcome = TourOutcome::CompletedOrSkipped;
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(step.title)
                            .size(14.5)
                            .strong()
                            .color(p.ink_primary),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(step.body)
                            .size(12.5)
                            .color(p.ink_secondary),
                    );
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if *step_idx > 0
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Anterior").size(12.0).color(p.ink_primary),
                                    )
                                    .fill(p.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, p.border)),
                                )
                                .clicked()
                        {
                            prev_pressed = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let is_last = *step_idx + 1 == total;
                            let next_label = if is_last { "¡Empezar!" } else { "Siguiente →" };
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(next_label)
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(p.accent)
                                    .min_size(Vec2::new(96.0, 28.0)),
                                )
                                .clicked()
                            {
                                next_pressed = true;
                            }
                        });
                    });
                });
        });

    if prev_pressed && *step_idx > 0 {
        *step_idx -= 1;
    } else if next_pressed {
        if *step_idx + 1 >= total {
            outcome = TourOutcome::CompletedOrSkipped;
        } else {
            *step_idx += 1;
        }
    }

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_position_should_stay_inside_screen() {
        let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(1280.0, 720.0));
        let card = Vec2::new(330.0, 178.0);
        let target = Rect::from_min_size(Pos2::new(1200.0, 650.0), Vec2::new(70.0, 40.0));
        let pos = callout_pos_for_target(target, screen, card);
        assert!(pos.x >= 16.0);
        assert!(pos.y >= 16.0);
        assert!(pos.x + card.x <= screen.right() - 16.0);
        assert!(pos.y + card.y <= screen.bottom() - 16.0);
    }
}
