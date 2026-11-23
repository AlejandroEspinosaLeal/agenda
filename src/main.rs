#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod calendar;
mod export;
mod focus;
mod model;
mod nlp;
mod theme;
mod timeblock;
mod tour;
mod tray;
mod updater;
mod views;

use app::AgendaApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let autostart = std::env::args().any(|a| a == "--autostart");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(if autostart {
                [360.0, 520.0]
            } else {
                [1360.0, 840.0]
            })
            .with_min_inner_size([320.0, 420.0])
            .with_visible(!autostart)
            .with_title("Mi Agenda"),
        ..Default::default()
    };

    eframe::run_native(
        "Mi Agenda",
        options,
        Box::new(move |cc| {
            theme::setup_fonts(&cc.egui_ctx);
            Ok(Box::new(AgendaApp::new(autostart)))
        }),
    )
}
