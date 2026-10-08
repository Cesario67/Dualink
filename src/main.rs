#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridge;
mod ds4;
mod dualsense;
mod hidhide;
mod output_report;
mod reset;
mod virtual_pad;
mod widgets;
mod xbox;

use eframe::egui;

fn main() -> eframe::Result {
    // Copie élevée lancée par le bouton « Réinitialiser la manette » : pas de fenêtre.
    if std::env::args().nth(1).as_deref() == Some(reset::RESET_ARG) {
        std::process::exit(reset::run_elevated_helper());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Dualink")
            .with_inner_size([420.0, 460.0])
            .with_min_inner_size([380.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native("Dualink", options, Box::new(|cc| Ok(Box::new(app::DualinkApp::new(cc)))))
}
