//! Manette Xbox 360 virtuelle (ViGEmBus) et conversion depuis l'état DualSense.

use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::dualsense::DualSenseState;

pub struct VirtualXbox360 {
    target: Xbox360Wired<Client>,
}

impl VirtualXbox360 {
    /// Se connecte au driver ViGEmBus et branche une manette virtuelle.
    pub fn plug_in() -> Result<Self, String> {
        let client = Client::connect()
            .map_err(|e| format!("ViGEmBus introuvable ou inaccessible (est-il installé ?) : {e:?}"))?;
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
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
            .update(&to_xgamepad(state))
            .map_err(|e| format!("envoi de l'état à la manette virtuelle : {e:?}"))
    }
}

/// Mapping Xbox classique : Croix = A, Rond = B, Carré = X, Triangle = Y,
/// Create = Back, Options = Start, bouton PS = Guide.
pub fn to_xgamepad(s: &DualSenseState) -> XGamepad {
    let pairs = [
        (s.dpad_up, XButtons::UP),
        (s.dpad_down, XButtons::DOWN),
        (s.dpad_left, XButtons::LEFT),
        (s.dpad_right, XButtons::RIGHT),
        (s.options, XButtons::START),
        (s.create, XButtons::BACK),
        (s.l3, XButtons::LTHUMB),
        (s.r3, XButtons::RTHUMB),
        (s.l1, XButtons::LB),
        (s.r1, XButtons::RB),
        (s.ps, XButtons::GUIDE),
        (s.cross, XButtons::A),
        (s.circle, XButtons::B),
        (s.square, XButtons::X),
        (s.triangle, XButtons::Y),
    ];
    let raw = pairs
        .iter()
        .filter(|(pressed, _)| *pressed)
        .fold(0u16, |acc, (_, mask)| acc | mask);

    XGamepad {
        buttons: XButtons(raw),
        left_trigger: s.l2,
        right_trigger: s.r2,
        thumb_lx: stick_x(s.left_x),
        thumb_ly: stick_y(s.left_y),
        thumb_rx: stick_x(s.right_x),
        thumb_ry: stick_y(s.right_y),
    }
}

/// 0..=255 vers -32768..=32767.
fn stick_x(v: u8) -> i16 {
    (i32::from(v) * 257 - 32768) as i16
}

/// Même conversion, axe inversé : sur DualSense Y augmente vers le bas, sur XInput vers le haut.
fn stick_y(v: u8) -> i16 {
    !stick_x(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_range_covers_full_i16() {
        assert_eq!(stick_x(0), i16::MIN);
        assert_eq!(stick_x(255), i16::MAX);
    }

    #[test]
    fn y_axis_is_inverted() {
        assert_eq!(stick_y(0), i16::MAX);
        assert_eq!(stick_y(255), i16::MIN);
    }

    #[test]
    fn cross_maps_to_a_and_triggers_pass_through() {
        let state = DualSenseState { cross: true, l2: 200, ..Default::default() };
        let pad = to_xgamepad(&state);
        assert_eq!(pad.buttons.raw, XButtons::A);
        assert_eq!(pad.left_trigger, 200);
    }
}
