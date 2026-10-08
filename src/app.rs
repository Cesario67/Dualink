//! Fenêtre principale : barre latérale et trois pages (Manette, Masquage, Installation).

use eframe::egui::{self, vec2, Align, Color32, Layout, Margin, RichText, Ui};

use crate::bridge::{Bridge, Snapshot, Status};
use crate::deps::{self, Component};
use crate::hidhide::HidHideState;
use crate::reset;
use crate::theme::{self, ACCENT, BG, ERR, MUTED, OK, SIDEBAR, WARN};
use crate::virtual_pad::Emulation;
use crate::widgets::{card, controller, nav_item, pill, toggle};

const SIDEBAR_WIDTH: f32 = 224.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Controller,
    Masking,
    Installation,
}

pub struct DualinkApp {
    bridge: Bridge,
    page: Page,
    /// Résultat de la dernière demande de réinitialisation de la manette.
    reset_message: Option<Result<(), String>>,
    /// Résultat de la dernière demande d'installation des composants manquants.
    install_message: Option<Result<(), String>>,
    /// Des installateurs sont fournis dans `redist\` à côté de l'exécutable.
    bundled_installers: bool,
}

impl DualinkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        Self {
            bridge: Bridge::spawn(move || ctx.request_repaint()),
            page: initial_page(),
            reset_message: None,
            install_message: None,
            bundled_installers: deps::has_bundled_installers(),
        }
    }
}

/// Page ouverte au démarrage. `DUALINK_PAGE` (masking, installation) sert au développement,
/// pour vérifier une page sans cliquer.
fn initial_page() -> Page {
    match std::env::var("DUALINK_PAGE").as_deref() {
        Ok("masking") => Page::Masking,
        Ok("installation") => Page::Installation,
        _ => Page::Controller,
    }
}

impl eframe::App for DualinkApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let snapshot = self.bridge.snapshot();
        let mode = self.bridge.emulation();

        egui::Panel::left("sidebar")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .frame(egui::Frame::new().fill(SIDEBAR).inner_margin(Margin::same(16)))
            .show(ui, |ui| self.show_sidebar(ui, &snapshot, mode));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::symmetric(30, 26)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.page {
                    Page::Controller => self.show_controller(ui, &snapshot, mode),
                    Page::Masking => self.show_masking(ui, &snapshot),
                    Page::Installation => self.show_installation(ui, &snapshot),
                });
            });
    }
}

impl DualinkApp {
    fn show_sidebar(&mut self, ui: &mut Ui, snapshot: &Snapshot, mode: Emulation) {
        ui.add_space(6.0);
        ui.label(RichText::new("Dualink").size(25.0).strong().color(Color32::WHITE));
        ui.label(RichText::new("DualSense vers Xbox / DS4").small().color(MUTED));
        ui.add_space(22.0);

        for (page, label) in [
            (Page::Controller, "Manette"),
            (Page::Masking, "Masquage"),
            (Page::Installation, "Installation"),
        ] {
            if nav_item(ui, label, self.page == page).clicked() {
                self.page = page;
            }
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).small().color(MUTED));
            ui.add_space(6.0);
            let (color, text) = status_summary(&snapshot.status, mode);
            pill(ui, color, &text);
        });
    }

    fn show_controller(&mut self, ui: &mut Ui, snapshot: &Snapshot, mode: Emulation) {
        let (color, text) = status_summary(&snapshot.status, mode);
        page_header(ui, "Manette", "Visualiseur en direct de votre DualSense", Some((color, text)));

        if let Status::ViGemUnavailable(message) | Status::Error(message) = &snapshot.status {
            card(ui, |ui| {
                ui.label(RichText::new(message).color(ERR));
                if matches!(snapshot.status, Status::ViGemUnavailable(_)) {
                    ui.label(RichText::new("Ouvrez la page Installation pour l'installer en un clic.").color(MUTED));
                }
            });
        }

        card(ui, |ui| {
            ui.vertical_centered(|ui| controller(ui, &snapshot.input));
        });

        card(ui, |ui| {
            let mut enabled = self.bridge.is_enabled();
            setting_row(ui, "Émulation", "Crée la manette virtuelle vue par les jeux", |ui| {
                if toggle(ui, &mut enabled).changed() {
                    self.bridge.set_enabled(enabled);
                }
            });
            ui.separator();

            let mut selected = mode;
            setting_row(ui, "Manette émulée", "Changer de mode recrée la manette virtuelle", |ui| {
                ui.selectable_value(&mut selected, Emulation::DualShock4, Emulation::DualShock4.label());
                ui.selectable_value(&mut selected, Emulation::Xbox360, Emulation::Xbox360.label());
            });
            if selected != mode {
                self.bridge.set_emulation(selected);
            }
            if selected == Emulation::DualShock4 {
                ui.label(RichText::new("Pas de vibration en mode DualShock 4 pour l'instant.").small().color(WARN));
            }
        });
    }

    fn show_masking(&mut self, ui: &mut Ui, snapshot: &Snapshot) {
        let installed = snapshot.hidhide != HidHideState::NotInstalled;
        let (color, text, detail) = match &snapshot.hidhide {
            HidHideState::NotInstalled => (WARN, "HidHide absent", "HidHide n'est pas installé : option indisponible.".to_owned()),
            HidHideState::Off => (MUTED, "Manette visible", "La manette est visible par les autres applications.".to_owned()),
            HidHideState::Hidden => (OK, "Manette masquée", "Masquée pour les autres applications, Dualink la voit toujours.".to_owned()),
            HidHideState::Error(e) => (ERR, "Erreur", e.clone()),
        };
        page_header(ui, "Masquage", "Cache la vraie DualSense à Steam et aux jeux (via HidHide)", Some((color, text.to_owned())));

        card(ui, |ui| {
            let mut wanted = self.bridge.is_hide_wanted();
            setting_row(
                ui,
                "Masquer la vraie DualSense",
                "Évite que Steam ou un jeu voient la vraie manette en plus de la manette virtuelle.",
                |ui| {
                    ui.add_enabled_ui(installed, |ui| {
                        if toggle(ui, &mut wanted).changed() {
                            self.bridge.set_hide_wanted(wanted);
                        }
                    });
                },
            );
            ui.label(RichText::new(detail).small().color(color));
            ui.label(
                RichText::new("Lancez Dualink avant Steam et vos jeux : un programme qui a déjà ouvert la manette la garde.")
                    .small()
                    .color(MUTED),
            );
        });

        card(ui, |ui| {
            setting_row(
                ui,
                "Réinitialiser la manette",
                "Équivalent d'un débranchement/rebranchement (droits administrateur). Utile si Steam tenait déjà la manette.",
                |ui| {
                    if ui.add_enabled(installed, egui::Button::new("Réinitialiser")).clicked() {
                        self.reset_message = Some(reset::request_elevated());
                    }
                },
            );
            match &self.reset_message {
                Some(Ok(())) => {
                    ui.label(RichText::new("Réinitialisation demandée.").small().color(MUTED));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).small().color(ERR));
                }
                None => {}
            }
        });
    }

    fn show_installation(&mut self, ui: &mut Ui, snapshot: &Snapshot) {
        let rows = [
            (Component::ViGEmBus, "ViGEmBus", "Obligatoire : crée les manettes virtuelles", snapshot.vigem_installed),
            (
                Component::HidHide,
                "HidHide",
                "Recommandé : masque la vraie manette aux autres applications",
                snapshot.hidhide != HidHideState::NotInstalled,
            ),
        ];
        let missing: Vec<Component> = rows.iter().filter(|row| !row.3).map(|row| row.0).collect();
        if missing.is_empty() {
            self.install_message = None;
        }

        let summary = if missing.is_empty() {
            (OK, "Tout est installé".to_owned())
        } else {
            (WARN, "Composants manquants".to_owned())
        };
        page_header(ui, "Installation", "Composants nécessaires au fonctionnement de Dualink", Some(summary));

        for (component, name, role, installed) in rows {
            card(ui, |ui| {
                setting_row(ui, name, role, |ui| {
                    if installed {
                        pill(ui, OK, "Installé");
                    } else {
                        if primary_button(ui, "Installer").clicked() {
                            self.install_message = Some(deps::request_install(&[component]));
                        }
                        pill(ui, WARN, "Manquant");
                    }
                });
            });
        }

        card(ui, |ui| {
            if missing.len() > 1 && primary_button(ui, "Tout installer").clicked() {
                self.install_message = Some(deps::request_install(&missing));
            }
            match &self.install_message {
                Some(Ok(())) => {
                    ui.label(
                        RichText::new("Installation demandée : acceptez l'invite Windows puis patientez, l'état se met à jour tout seul.")
                            .color(MUTED),
                    );
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(ERR));
                }
                None => {}
            }
            ui.label(
                RichText::new(if self.bundled_installers {
                    "Source : installateurs fournis avec Dualink (aucune connexion Internet nécessaire)."
                } else {
                    "Source : installateurs officiels téléchargés depuis GitHub (SHA256 vérifié), ou winget en secours. Connexion Internet nécessaire."
                })
                .small()
                .color(MUTED),
            );
            ui.label(RichText::new("Une invite d'autorisation Windows s'affichera. HidHide peut demander un redémarrage.").small().color(MUTED));
            if let Some(log) = deps::log_path() {
                ui.label(RichText::new(format!("Journal : {}", log.display())).small().color(MUTED));
            }
        });
    }
}

/// Titre de page avec sous-titre et pastille d'état alignée à droite.
fn page_header(ui: &mut Ui, title: &str, subtitle: &str, status: Option<(Color32, String)>) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new(title).strong().color(Color32::WHITE));
            ui.label(RichText::new(subtitle).color(MUTED));
        });
        if let Some((color, text)) = status {
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| pill(ui, color, &text));
        }
    });
    ui.add_space(4.0);
}

/// Ligne de réglage : titre et description à gauche, contrôle à droite.
fn setting_row(ui: &mut Ui, title: &str, description: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 190.0).max(160.0));
            ui.label(RichText::new(title).strong().size(16.0));
            ui.label(RichText::new(description).small().color(MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), add_control);
    });
}

fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).strong().color(Color32::WHITE)).fill(ACCENT).min_size(vec2(110.0, 34.0)))
}

/// Pastille d'état courte pour la barre latérale et l'en-tête de la page Manette.
fn status_summary(status: &Status, mode: Emulation) -> (Color32, String) {
    match status {
        Status::Active => (OK, format!("Active : {}", mode.label())),
        Status::WaitingForController => (WARN, "En attente de la manette".to_owned()),
        Status::Disabled => (MUTED, "Émulation désactivée".to_owned()),
        Status::ViGemUnavailable(_) => (ERR, "ViGEmBus manquant".to_owned()),
        Status::Error(_) => (ERR, "Erreur".to_owned()),
    }
}
