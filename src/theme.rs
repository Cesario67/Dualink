//! Thème sombre bleu de l'application : palette, styles de texte et aspect des widgets.

use eframe::egui::{
    self, vec2, Color32, CornerRadius, FontFamily, FontId, Shadow, Stroke, TextStyle, Theme, ThemePreference,
};

pub const BG: Color32 = Color32::from_rgb(13, 17, 24);
pub const SIDEBAR: Color32 = Color32::from_rgb(9, 12, 18);
pub const CARD: Color32 = Color32::from_rgb(22, 28, 39);
pub const CARD_BORDER: Color32 = Color32::from_rgb(34, 43, 58);
pub const HOVER: Color32 = Color32::from_rgb(28, 36, 50);
pub const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_STRONG: Color32 = Color32::from_rgb(37, 99, 235);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(24, 40, 74);
pub const TEXT: Color32 = Color32::from_rgb(230, 234, 242);
pub const MUTED: Color32 = Color32::from_rgb(138, 148, 166);
pub const OK: Color32 = Color32::from_rgb(52, 199, 120);
pub const WARN: Color32 = Color32::from_rgb(245, 165, 36);
pub const ERR: Color32 = Color32::from_rgb(239, 68, 68);

const BUTTON_IDLE: Color32 = Color32::from_rgb(31, 40, 56);
const BUTTON_HOVER: Color32 = Color32::from_rgb(40, 52, 73);

fn radius(value: u8) -> CornerRadius {
    CornerRadius::same(value)
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(ThemePreference::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.spacing.button_padding = vec2(16.0, 8.0);
        style.spacing.interact_size.y = 34.0;

        style.text_styles.insert(TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));

        let visuals = &mut style.visuals;
        visuals.panel_fill = BG;
        visuals.window_fill = CARD;
        visuals.window_stroke = Stroke::new(1.0, CARD_BORDER);
        visuals.window_corner_radius = radius(12);
        visuals.window_shadow = Shadow::NONE;
        visuals.popup_shadow = Shadow::NONE;
        visuals.extreme_bg_color = SIDEBAR;
        visuals.faint_bg_color = CARD;
        visuals.hyperlink_color = ACCENT;
        visuals.selection.bg_fill = ACCENT;
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = CARD;
        widgets.noninteractive.weak_bg_fill = CARD;
        widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_BORDER);
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        widgets.noninteractive.corner_radius = radius(10);

        widgets.inactive.bg_fill = BUTTON_IDLE;
        widgets.inactive.weak_bg_fill = BUTTON_IDLE;
        widgets.inactive.bg_stroke = Stroke::new(1.0, CARD_BORDER);
        widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        widgets.inactive.corner_radius = radius(10);

        widgets.hovered.bg_fill = BUTTON_HOVER;
        widgets.hovered.weak_bg_fill = BUTTON_HOVER;
        widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        widgets.hovered.corner_radius = radius(10);

        widgets.active.bg_fill = ACCENT_STRONG;
        widgets.active.weak_bg_fill = ACCENT_STRONG;
        widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        widgets.active.corner_radius = radius(10);

        widgets.open.bg_fill = BUTTON_HOVER;
        widgets.open.weak_bg_fill = BUTTON_HOVER;
        widgets.open.corner_radius = radius(10);
    });
}
