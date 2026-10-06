//! Manette DualShock 4 virtuelle (ViGEmBus) et conversion depuis l'état DualSense.
//!
//! Le rapport DS4 a la même disposition que celui de la DualSense : sticks de 0 à 255 avec Y vers
//! le bas (pas d'inversion), croix en « hat » de 0 à 8, boutons sur 16 bits.

use vigem_client::{Client, DS4Report, DualShock4Wired, TargetId};

use crate::dualsense::DualSenseState;

/// Seuil à partir duquel les gâchettes comptent aussi comme boutons numériques L2/R2.
const TRIGGER_DIGITAL_THRESHOLD: u8 = 30;

const HAT_NEUTRAL: u16 = 8;
const SQUARE: u16 = 1 << 4;
const CROSS: u16 = 1 << 5;
const CIRCLE: u16 = 1 << 6;
const TRIANGLE: u16 = 1 << 7;
const L1: u16 = 1 << 8;
const R1: u16 = 1 << 9;
const L2: u16 = 1 << 10;
const R2: u16 = 1 << 11;
const SHARE: u16 = 1 << 12;
const OPTIONS: u16 = 1 << 13;
const L3: u16 = 1 << 14;
const R3: u16 = 1 << 15;
const SPECIAL_PS: u8 = 1 << 0;
const SPECIAL_TOUCHPAD: u8 = 1 << 1;

pub struct VirtualDs4 {
    target: DualShock4Wired<Client>,
}

impl VirtualDs4 {
    pub fn plug_in() -> Result<Self, String> {
        let client = Client::connect()
            .map_err(|e| format!("ViGEmBus introuvable ou inaccessible (est-il installé ?) : {e:?}"))?;
        let mut target = DualShock4Wired::new(client, TargetId::DUALSHOCK4_WIRED);
        target
            .plugin()
            .map_err(|e| format!("branchement de la manette virtuelle : {e:?}"))?;
        target
            .wait_ready()
            .map_err(|e| format!("manette virtuelle non prête : {e:?}"))?;
        Ok(Self { target })
    }

    pub fn send(&mut self, state: &DualSenseState) -> Result<(), String> {
        self.target
            .update(&to_report(state))
            .map_err(|e| format!("envoi de l'état à la manette virtuelle : {e:?}"))
    }
}

/// Croix en « hat » : 0 haut, puis sens horaire par pas de 45 degrés, 8 neutre.
fn hat(s: &DualSenseState) -> u16 {
    match (s.dpad_up, s.dpad_right, s.dpad_down, s.dpad_left) {
        (true, false, false, false) => 0,
        (true, true, false, false) => 1,
        (false, true, false, false) => 2,
        (false, true, true, false) => 3,
        (false, false, true, false) => 4,
        (false, false, true, true) => 5,
        (false, false, false, true) => 6,
        (true, false, false, true) => 7,
        _ => HAT_NEUTRAL,
    }
}

pub fn to_report(s: &DualSenseState) -> DS4Report {
    let flags = [
        (s.square, SQUARE),
        (s.cross, CROSS),
        (s.circle, CIRCLE),
        (s.triangle, TRIANGLE),
        (s.l1, L1),
        (s.r1, R1),
        (s.l2 >= TRIGGER_DIGITAL_THRESHOLD, L2),
        (s.r2 >= TRIGGER_DIGITAL_THRESHOLD, R2),
        (s.create, SHARE),
        (s.options, OPTIONS),
        (s.l3, L3),
        (s.r3, R3),
    ];
    let buttons = flags
        .iter()
        .filter(|(pressed, _)| *pressed)
        .fold(hat(s), |acc, (_, mask)| acc | mask);

    let special = (if s.ps { SPECIAL_PS } else { 0 }) | (if s.touchpad { SPECIAL_TOUCHPAD } else { 0 });

    DS4Report {
        thumb_lx: s.left_x,
        thumb_ly: s.left_y,
        thumb_rx: s.right_x,
        thumb_ry: s.right_y,
        buttons,
        special,
        trigger_l: s.l2,
        trigger_r: s.r2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_state_is_neutral_hat_and_centered_sticks() {
        let r = to_report(&DualSenseState::default());
        assert_eq!(r.buttons, HAT_NEUTRAL);
        assert_eq!((r.thumb_lx, r.thumb_ly), (128, 128));
    }

    #[test]
    fn sticks_are_not_inverted() {
        let s = DualSenseState { left_y: 0, ..Default::default() };
        assert_eq!(to_report(&s).thumb_ly, 0);
    }

    #[test]
    fn dpad_diagonals_and_conflicts() {
        let up_right = DualSenseState { dpad_up: true, dpad_right: true, ..Default::default() };
        assert_eq!(hat(&up_right), 1);
        let up_left = DualSenseState { dpad_up: true, dpad_left: true, ..Default::default() };
        assert_eq!(hat(&up_left), 7);
        let opposite = DualSenseState { dpad_up: true, dpad_down: true, ..Default::default() };
        assert_eq!(hat(&opposite), HAT_NEUTRAL);
    }

    #[test]
    fn face_buttons_share_options_and_special() {
        let s = DualSenseState {
            cross: true,
            create: true,
            ps: true,
            touchpad: true,
            l2: 255,
            ..Default::default()
        };
        let r = to_report(&s);
        assert_eq!(r.buttons, HAT_NEUTRAL | CROSS | SHARE | L2);
        assert_eq!(r.special, SPECIAL_PS | SPECIAL_TOUCHPAD);
        assert_eq!(r.trigger_l, 255);
    }

    #[test]
    fn light_trigger_pull_is_not_a_digital_press() {
        let s = DualSenseState { r2: 10, ..Default::default() };
        assert_eq!(to_report(&s).buttons & R2, 0);
    }
}
