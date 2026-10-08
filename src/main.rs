#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridge;
mod deps;
mod ds4;
mod dualsense;
mod elevate;
mod hidhide;
mod icon;
mod output_report;
mod reset;
mod theme;
mod virtual_pad;
mod widgets;
mod xbox;

use eframe::egui;

fn main() -> eframe::Result {
    // Copies élevées lancées par l'interface (invite UAC) : pas de fenêtre.
    match std::env::args().nth(1).as_deref() {
        Some(reset::RESET_ARG) => std::process::exit(reset::run_elevated_helper()),
        Some(deps::INSTALL_ARG) => {
            let list = std::env::args().nth(2).unwrap_or_default();
            std::process::exit(deps::run_elevated_helper(&list));
        }
        _ => {}
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Dualink")
            .with_icon(std::sync::Arc::new(icon::app_icon()))
            .with_inner_size([1020.0, 720.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("Dualink", options, Box::new(|cc| Ok(Box::new(app::DualinkApp::new(cc)))))
}
