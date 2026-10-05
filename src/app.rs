//! Fenêtre principale : statut, interrupteur d'émulation et visualiseur d'entrées.

use eframe::egui::{self, Color32};

use crate::bridge::{Bridge, Status};
use crate::widgets::{chip, status_dot, stick, trigger};

pub struct DualinkApp {
    bridge: Bridge,
}

impl DualinkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        Self { bridge: Bridge::spawn(move || ctx.request_repaint()) }
    }
}

impl eframe::App for DualinkApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let snapshot = self.bridge.snapshot();
        let input = snapshot.input;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Dualink");
            ui.label("DualSense vers manette Xbox 360 virtuelle");
            ui.add_space(8.0);

            let (color, text) = describe(&snapshot.status);
            ui.horizontal(|ui| {
                status_dot(ui, color);
                ui.label(text);
            });

            let mut enabled = self.bridge.is_enabled();
            if ui.checkbox(&mut enabled, "Activer l'émulation").changed() {
                self.bridge.set_enabled(enabled);
            }

            ui.separator();

            ui.columns(2, |cols| {
                stick(&mut cols[0], "Stick gauche", input.left_x, input.left_y, input.l3);
                stick(&mut cols[1], "Stick droit", input.right_x, input.right_y, input.r3);
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                trigger(ui, "L2", input.l2);
                trigger(ui, "R2", input.r2);
            });
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                chip(ui, "Haut", input.dpad_up);
                chip(ui, "Bas", input.dpad_down);
                chip(ui, "Gauche", input.dpad_left);
                chip(ui, "Droite", input.dpad_right);
            });
            ui.horizontal_wrapped(|ui| {
                chip(ui, "Croix (A)", input.cross);
                chip(ui, "Rond (B)", input.circle);
                chip(ui, "Carré (X)", input.square);
                chip(ui, "Triangle (Y)", input.triangle);
            });
            ui.horizontal_wrapped(|ui| {
                chip(ui, "L1", input.l1);
                chip(ui, "R1", input.r1);
                chip(ui, "Create", input.create);
                chip(ui, "Options", input.options);
                chip(ui, "PS", input.ps);
            });
        });
    }
}

fn describe(status: &Status) -> (Color32, String) {
    match status {
        Status::Active => (Color32::from_rgb(60, 180, 90), "Active : manette Xbox 360 virtuelle branchée".into()),
        Status::WaitingForController => {
            (Color32::from_rgb(230, 170, 40), "En attente d'une DualSense (USB ou Bluetooth)".into())
        }
        Status::Disabled => (Color32::GRAY, "Émulation désactivée".into()),
        Status::ViGemUnavailable(e) => {
            (Color32::from_rgb(210, 70, 70), format!("ViGEmBus indisponible : {e}"))
        }
        Status::Error(e) => (Color32::from_rgb(210, 70, 70), e.clone()),
    }
}
