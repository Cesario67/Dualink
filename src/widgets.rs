//! Petits composants de dessin pour le visualiseur de manette.

use eframe::egui::{Color32, ProgressBar, RichText, Sense, Stroke, Ui, vec2};

use crate::dualsense::STICK_CENTER;

const STICK_SIZE: f32 = 96.0;

/// Stick analogique : cercle avec un point. Le point se remplit quand le stick est cliqué (L3/R3).
pub fn stick(ui: &mut Ui, label: &str, x: u8, y: u8, pressed: bool) {
    ui.vertical_centered(|ui| {
        let (response, painter) = ui.allocate_painter(vec2(STICK_SIZE, STICK_SIZE), Sense::hover());
        let center = response.rect.center();
        let radius = STICK_SIZE / 2.0 - 4.0;
        let outline = ui.visuals().widgets.noninteractive.fg_stroke.color;
        let accent = ui.visuals().selection.bg_fill;

        painter.circle_stroke(center, radius, Stroke::new(1.5, outline));
        painter.line_segment(
            [center - vec2(radius, 0.0), center + vec2(radius, 0.0)],
            Stroke::new(0.5, outline),
        );
        painter.line_segment(
            [center - vec2(0.0, radius), center + vec2(0.0, radius)],
            Stroke::new(0.5, outline),
        );

        let offset = vec2(axis(x), axis(y)) * (radius - 8.0);
        let dot = center + offset;
        if pressed {
            painter.circle_filled(dot, 8.0, accent);
        } else {
            painter.circle_stroke(dot, 8.0, Stroke::new(2.0, accent));
        }
        ui.label(label);
    });
}

/// Gâchette analogique 0..=255 sous forme de barre.
pub fn trigger(ui: &mut Ui, label: &str, value: u8) {
    ui.add(
        ProgressBar::new(f32::from(value) / 255.0)
            .desired_width(150.0)
            .text(format!("{label}  {value}")),
    );
}

/// Pastille de bouton : surlignée quand le bouton est enfoncé.
pub fn chip(ui: &mut Ui, label: &str, pressed: bool) {
    let text = if pressed {
        RichText::new(label)
            .strong()
            .color(Color32::BLACK)
            .background_color(ui.visuals().selection.bg_fill)
    } else {
        RichText::new(label).weak()
    };
    ui.label(text);
}

/// -1.0..=1.0 depuis un octet centré sur 128.
fn axis(v: u8) -> f32 {
    ((f32::from(v) - f32::from(STICK_CENTER)) / 127.0).clamp(-1.0, 1.0)
}

/// Pastille d'état colorée (rond plein).
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}
