//! Icône de l'application, dessinée par le code (carré bleu arrondi et manette blanche).

use eframe::egui::IconData;

const SIZE: u32 = 128;
const TOP: [f32; 3] = [59.0, 130.0, 246.0];
const BOTTOM: [f32; 3] = [30.0, 85.0, 214.0];

/// Distance signée à un rectangle arrondi (négative à l'intérieur).
fn rounded_rect(px: f32, py: f32, cx: f32, cy: f32, half_w: f32, half_h: f32, radius: f32) -> f32 {
    let qx = (px - cx).abs() - (half_w - radius);
    let qy = (py - cy).abs() - (half_h - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn circle(px: f32, py: f32, cx: f32, cy: f32, radius: f32) -> f32 {
    (px - cx).hypot(py - cy) - radius
}

/// Couverture (0 à 1) d'un pixel selon sa distance signée, avec un bord adouci d'un pixel.
fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

fn mix(base: [f32; 3], over: [f32; 3], amount: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| base[i] + (over[i] - base[i]) * amount)
}

pub fn app_icon() -> IconData {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let background = coverage(rounded_rect(px, py, 64.0, 64.0, 64.0, 64.0, 28.0));
            let shade = mix(TOP, BOTTOM, py / SIZE as f32);

            // Silhouette de la manette : boîtier et deux poignées.
            let body = rounded_rect(px, py, 64.0, 60.0, 38.0, 17.0, 15.0)
                .min(circle(px, py, 37.0, 76.0, 16.0))
                .min(circle(px, py, 91.0, 76.0, 16.0));
            let controller = coverage(body);

            // Détails : croix directionnelle à gauche, deux boutons à droite.
            let details = rounded_rect(px, py, 44.0, 60.0, 9.0, 3.0, 1.5)
                .min(rounded_rect(px, py, 44.0, 60.0, 3.0, 9.0, 1.5))
                .min(circle(px, py, 84.0, 54.0, 4.5))
                .min(circle(px, py, 94.0, 63.0, 4.5));
            let detail = coverage(details);

            let with_controller = mix(shade, [255.0, 255.0, 255.0], controller);
            let color = mix(with_controller, shade, detail * controller);
            rgba.extend_from_slice(&[color[0] as u8, color[1] as u8, color[2] as u8, (background * 255.0) as u8]);
        }
    }
    IconData { rgba, width: SIZE, height: SIZE }
}
