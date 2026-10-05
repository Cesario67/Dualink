#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridge;
mod dualsense;
mod widgets;
mod xbox;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Dualink")
            .with_inner_size([420.0, 460.0])
            .with_min_inner_size([380.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native("Dualink", options, Box::new(|cc| Ok(Box::new(app::DualinkApp::new(cc)))))
}
