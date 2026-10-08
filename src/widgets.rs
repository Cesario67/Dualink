//! Composants de l'interface : carte, pastille, interrupteur, navigation et manette dessinée.

use eframe::egui::{
    self, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Margin, Pos2, Rect, Response, RichText, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::dualsense::{DualSenseState, STICK_CENTER};
use crate::theme::{ACCENT, ACCENT_SOFT, CARD, CARD_BORDER, HOVER, MUTED, TEXT};

const TOGGLE_OFF: Color32 = Color32::from_rgb(52, 63, 84);

fn radius(value: u8) -> CornerRadius {
    CornerRadius::same(value)
}

/// Conteneur arrondi avec bordure, pour regrouper des réglages.
pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_BORDER))
        .corner_radius(radius(14))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}

/// Pastille d'état : point et texte colorés sur fond teinté.
pub fn pill(ui: &mut Ui, color: Color32, text: &str) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.16))
        .corner_radius(radius(255))
        .inner_margin(Margin::symmetric(12, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.5, color);
                ui.label(RichText::new(text).color(color).strong());
            });
        });
}

/// Interrupteur animé. Renvoie une réponse « changed » quand la valeur bascule.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(46.0, 26.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let mut background = TOGGLE_OFF.lerp_to_gamma(ACCENT, t);
    if !ui.is_enabled() {
        background = background.gamma_multiply(0.4);
    }
    ui.painter().rect_filled(rect, radius(13), background);
    let x = egui::lerp((rect.left() + 13.0)..=(rect.right() - 13.0), t);
    ui.painter().circle_filled(Pos2::new(x, rect.center().y), 10.0, Color32::WHITE);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// Entrée de la barre latérale, sur toute la largeur.
pub fn nav_item(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, radius(10), ACCENT_SOFT);
        let bar = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top() + 9.0),
            Pos2::new(rect.left() + 3.5, rect.bottom() - 9.0),
        );
        painter.rect_filled(bar, radius(2), ACCENT);
    } else if response.hovered() {
        painter.rect_filled(rect, radius(10), HOVER);
    }
    painter.text(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(15.5),
        if selected { TEXT } else { MUTED },
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

// --- Manette dessinée -------------------------------------------------------------------------

const BODY: Color32 = Color32::from_rgb(26, 33, 46);
const BODY_EDGE: Color32 = Color32::from_rgb(48, 60, 82);
const IDLE: Color32 = Color32::from_rgb(40, 51, 71);
const CROSS_COLOR: Color32 = Color32::from_rgb(96, 150, 255);
const CIRCLE_COLOR: Color32 = Color32::from_rgb(240, 90, 90);
const SQUARE_COLOR: Color32 = Color32::from_rgb(226, 128, 206);
const TRIANGLE_COLOR: Color32 = Color32::from_rgb(70, 206, 166);

/// Dessin de la manette : les éléments s'allument et bougent avec l'état courant.
/// Les coordonnées sont exprimées en largeurs de dessin (x de 0 à 1, y de 0 à `HEIGHT_RATIO`).
pub fn controller(ui: &mut Ui, state: &DualSenseState) {
    const HEIGHT_RATIO: f32 = 0.62;
    let width = ui.available_width().min(620.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, width * HEIGHT_RATIO), Sense::hover());
    let painter = ui.painter_at(rect);
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * width, rect.top() + y * width);
    let area = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
    let unit = |value: f32| value * width;
    let lit = |on: bool, color: Color32| if on { color } else { IDLE };

    // Corps : deux poignées et un boîtier central, bordure puis remplissage.
    for (grow, color) in [(unit(0.008), BODY_EDGE), (0.0, BODY)] {
        painter.rect_filled(area(0.10, 0.10, 0.90, 0.42).expand(grow), radius(40), color);
        painter.circle_filled(at(0.20, 0.44), unit(0.165) + grow, color);
        painter.circle_filled(at(0.80, 0.44), unit(0.165) + grow, color);
    }

    // Gâchettes L2 / R2 : jauge qui monte avec la pression, puis épaulières L1 / R1.
    for (x0, x1, value, label) in [(0.17, 0.33, state.l2, "L2"), (0.67, 0.83, state.r2, "R2")] {
        let slot = area(x0, 0.0, x1, 0.085);
        painter.rect_filled(slot, radius(6), IDLE);
        let filled = slot.height() * f32::from(value) / 255.0;
        let fill = Rect::from_min_max(Pos2::new(slot.left(), slot.bottom() - filled), slot.right_bottom());
        painter.rect_filled(fill, radius(6), ACCENT);
        painter.text(slot.center(), Align2::CENTER_CENTER, label, FontId::proportional(unit(0.02)), TEXT);
    }
    painter.rect_filled(area(0.15, 0.092, 0.35, 0.135), radius(8), lit(state.l1, ACCENT));
    painter.rect_filled(area(0.65, 0.092, 0.85, 0.135), radius(8), lit(state.r1, ACCENT));

    // Croix directionnelle.
    let (dx, dy, arm, thick) = (0.26, 0.225, unit(0.045), unit(0.034));
    for (offset, pressed) in [
        (vec2(0.0, -1.0), state.dpad_up),
        (vec2(0.0, 1.0), state.dpad_down),
        (vec2(-1.0, 0.0), state.dpad_left),
        (vec2(1.0, 0.0), state.dpad_right),
    ] {
        let center = at(dx, dy) + offset * arm;
        painter.rect_filled(Rect::from_center_size(center, Vec2::splat(thick)), radius(4), lit(pressed, ACCENT));
    }

    // Boutons d'action : symboles PlayStation, colorés quand ils sont enfoncés.
    let (fx, fy, spread, size) = (0.74, 0.225, unit(0.062), unit(0.03));
    let place = |dx: f32, dy: f32| at(fx, fy) + vec2(dx, dy) * spread;
    face_button(&painter, place(0.0, 1.0), size, CROSS_COLOR, state.cross, Symbol::Cross);
    face_button(&painter, place(1.0, 0.0), size, CIRCLE_COLOR, state.circle, Symbol::Circle);
    face_button(&painter, place(-1.0, 0.0), size, SQUARE_COLOR, state.square, Symbol::Square);
    face_button(&painter, place(0.0, -1.0), size, TRIANGLE_COLOR, state.triangle, Symbol::Triangle);

    // Pavé tactile, Create / Options et bouton PS.
    painter.rect_filled(area(0.39, 0.12, 0.61, 0.26), radius(10), lit(state.touchpad, ACCENT));
    painter.rect_filled(area(0.335, 0.165, 0.365, 0.18), radius(4), lit(state.create, ACCENT));
    painter.rect_filled(area(0.635, 0.165, 0.665, 0.18), radius(4), lit(state.options, ACCENT));
    painter.circle_filled(at(0.5, 0.315), unit(0.017), lit(state.ps, ACCENT));

    // Sticks : la pastille suit la position, et se remplit quand le stick est enfoncé (L3 / R3).
    for (cx, x, y, pressed) in [(0.38, state.left_x, state.left_y, state.l3), (0.62, state.right_x, state.right_y, state.r3)] {
        let center = at(cx, 0.385);
        painter.circle_filled(center, unit(0.058), IDLE);
        painter.circle_stroke(center, unit(0.058), Stroke::new(1.5, BODY_EDGE));
        let travel = unit(0.03);
        let dot = center + vec2(axis(x), axis(y)) * travel;
        painter.circle_filled(dot, unit(0.034), if pressed { ACCENT } else { BODY_EDGE });
        painter.circle_stroke(dot, unit(0.034), Stroke::new(1.5, ACCENT));
    }
}

enum Symbol {
    Cross,
    Circle,
    Square,
    Triangle,
}

fn face_button(painter: &egui::Painter, center: Pos2, size: f32, color: Color32, pressed: bool, symbol: Symbol) {
    painter.circle_filled(center, size, if pressed { color } else { IDLE });
    painter.circle_stroke(center, size, Stroke::new(1.5, color.gamma_multiply(if pressed { 1.0 } else { 0.55 })));

    let ink = if pressed { Color32::from_rgb(13, 17, 24) } else { color };
    let stroke = Stroke::new(2.0, ink);
    let r = size * 0.45;
    match symbol {
        Symbol::Cross => {
            painter.line_segment([center + vec2(-r, -r), center + vec2(r, r)], stroke);
            painter.line_segment([center + vec2(-r, r), center + vec2(r, -r)], stroke);
        }
        Symbol::Circle => {
            painter.circle_stroke(center, r, stroke);
        }
        Symbol::Square => {
            painter.rect_stroke(Rect::from_center_size(center, Vec2::splat(r * 1.8)), radius(2), stroke, StrokeKind::Middle);
        }
        Symbol::Triangle => {
            let points = vec![center + vec2(0.0, -r * 1.05), center + vec2(r * 1.05, r * 0.85), center + vec2(-r * 1.05, r * 0.85)];
            painter.add(Shape::closed_line(points, stroke));
        }
    }
}

/// -1.0..=1.0 depuis un octet centré sur 128.
fn axis(value: u8) -> f32 {
    ((f32::from(value) - f32::from(STICK_CENTER)) / 127.0).clamp(-1.0, 1.0)
}
