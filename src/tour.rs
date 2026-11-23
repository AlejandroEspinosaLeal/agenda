use eframe::egui::{self, Color32, Key, Order, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

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
    pub title: &'static str,
    pub body: &'static str,
}

pub const TOUR_STEPS: [TourStep; 6] = [
    TourStep {
        target: TourTarget::CalendarArea,
        title: "1. Tu calendario interactivo",
        body: "En Mensual, arrastra sobre varios días seguidos para crear una tarea multi-día. En Diaria o Semanal, arrastra sobre las horas para crear, mover o estirar bloques.",
    },
    TourStep {
        target: TourTarget::ViewsNav,
        title: "2. Cambia de vista al instante",
        body: "Alterna entre Mensual (M), Semanal (W) con modo horario/tablero, y Diaria (D) con rejilla de horas y línea de hora actual.",
    },
    TourStep {
        target: TourTarget::Backlog,
        title: "3. Bandeja de pendientes (Backlog)",
        body: "Guarda aquí tareas sin hora fija. Cuando quieras hacerlas, arrástralas directamente desde la barra lateral hasta una hora del calendario.",
    },
    TourStep {
        target: TourTarget::CommandPalette,
        title: "4. Paleta rápida (Ctrl+K)",
        body: "Pulsa Ctrl+K para buscar cualquier tarea o nota, ejecutar comandos o crear bloques escribiendo natural: \"Estudiar Rust 18:00-19:30 #Estudio\".",
    },
    TourStep {
        target: TourTarget::FocusMode,
        title: "5. Modo Enfoque y Pomodoro",
        body: "Activa el Modo Enfoque (tecla F) para ocultar distracciones, centrar tu bloque actual y usar el temporizador 25/5 o 50/10 con notas.",
    },
    TourStep {
        target: TourTarget::ThemeAndHelp,
        title: "6. Tema, notificaciones y ayuda",
        body: "Cambia entre modo claro/oscuro, recibe una notificación diaria en Windows con tus pendientes del día y vuelve a abrir esta guía cuando quieras desde el botón \"?\".",
    },
];

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
