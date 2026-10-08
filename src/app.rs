//! Fenêtre principale : statut, interrupteur d'émulation et visualiseur d'entrées.

use eframe::egui::{self, Color32};

use crate::bridge::{Bridge, Status};
use crate::hidhide::HidHideState;
use crate::reset;
use crate::virtual_pad::Emulation;
use crate::widgets::{chip, status_dot, stick, trigger};

pub struct DualinkApp {
    bridge: Bridge,
    /// Résultat de la dernière demande de réinitialisation de la manette.
    reset_message: Option<Result<(), String>>,
}

impl DualinkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        Self { bridge: Bridge::spawn(move || ctx.request_repaint()), reset_message: None }
    }
}

impl eframe::App for DualinkApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let snapshot = self.bridge.snapshot();
        let input = snapshot.input;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Dualink");
            ui.label("Utilisez votre DualSense comme une manette Xbox 360 ou DualShock 4");
            ui.add_space(8.0);

            let mut mode = self.bridge.emulation();
            let (color, text) = describe(&snapshot.status, mode);
            ui.horizontal(|ui| {
                status_dot(ui, color);
                ui.label(text);
            });

            let mut enabled = self.bridge.is_enabled();
            if ui.checkbox(&mut enabled, "Activer l'émulation").changed() {
                self.bridge.set_enabled(enabled);
            }

            ui.horizontal(|ui| {
                ui.label("Émuler :");
                let xbox = ui.radio_value(&mut mode, Emulation::Xbox360, Emulation::Xbox360.label());
                let ds4 = ui.radio_value(&mut mode, Emulation::DualShock4, Emulation::DualShock4.label());
                if xbox.changed() || ds4.changed() {
                    self.bridge.set_emulation(mode);
                }
            });
            if mode == Emulation::DualShock4 {
                ui.weak("Pas de vibration en mode DualShock 4 pour l'instant.");
            }

            show_hidhide(ui, &self.bridge, &snapshot.hidhide, &mut self.reset_message);

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

/// Case « masquer la DualSense » et état de HidHide.
fn show_hidhide(
    ui: &mut egui::Ui,
    bridge: &Bridge,
    state: &HidHideState,
    reset_message: &mut Option<Result<(), String>>,
) {
    let installed = *state != HidHideState::NotInstalled;
    let mut wanted = bridge.is_hide_wanted();
    let changed = ui
        .add_enabled(
            installed,
            egui::Checkbox::new(&mut wanted, "Masquer la vraie DualSense aux autres applications (HidHide)"),
        )
        .changed();
    if changed {
        bridge.set_hide_wanted(wanted);
    }

    let (color, text) = match state {
        HidHideState::NotInstalled => (Color32::GRAY, "HidHide n'est pas installé : option indisponible".to_owned()),
        HidHideState::Off => (Color32::GRAY, "Manette visible par les autres applications".to_owned()),
        HidHideState::Hidden => (Color32::from_rgb(60, 180, 90), "Manette masquée, Dualink la voit toujours".to_owned()),
        HidHideState::Error(e) => (Color32::from_rgb(210, 70, 70), e.clone()),
    };
    ui.horizontal_wrapped(|ui| {
        ui.add_space(22.0);
        ui.colored_label(color, text);
    });

    // Équivalent d'un débranchement/rebranchement : les applications qui tenaient déjà la manette
    // (Steam, un jeu) la perdent et ne la revoient plus, puisqu'elle est masquée.
    ui.horizontal(|ui| {
        ui.add_space(22.0);
        let button = ui.add_enabled(installed, egui::Button::new("Réinitialiser la manette"));
        if button
            .on_hover_text("Redémarre la DualSense comme un débranchement/rebranchement (demande les droits administrateur)")
            .clicked()
        {
            *reset_message = Some(reset::request_elevated());
        }
        match reset_message {
            Some(Ok(())) => ui.weak("Demandé"),
            Some(Err(e)) => ui.colored_label(Color32::from_rgb(210, 70, 70), e.as_str()),
            None => ui.label(""),
        };
    });
}

fn describe(status: &Status, mode: Emulation) -> (Color32, String) {
    match status {
        Status::Active => (
            Color32::from_rgb(60, 180, 90),
            format!("Active : manette {} virtuelle branchée", mode.label()),
        ),
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
