//! Lecture de la DualSense en HID brut (USB et Bluetooth) et décodage des rapports d'entrée.

use hidapi::{HidApi, HidDevice};

const SONY_VID: u16 = 0x054C;
const DUALSENSE_PID: u16 = 0x0CE6;
const DUALSENSE_EDGE_PID: u16 = 0x0DF2;

/// Rapport d'entrée USB (64 octets) ou Bluetooth « simple » (10 octets, avant le mode complet).
const REPORT_ID_USB_OR_BT_SIMPLE: u8 = 0x01;
/// Rapport d'entrée Bluetooth complet : même contenu que l'USB, décalé d'un octet.
const REPORT_ID_BT_FULL: u8 = 0x31;
/// Rapport de calibration : le lire fait basculer la manette Bluetooth en mode complet.
const FEATURE_REPORT_CALIBRATION: u8 = 0x05;

pub const STICK_CENTER: u8 = 128;

/// État d'entrée décodé, indépendant du transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseState {
    pub left_x: u8,
    pub left_y: u8,
    pub right_x: u8,
    pub right_y: u8,
    pub l2: u8,
    pub r2: u8,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub square: bool,
    pub cross: bool,
    pub circle: bool,
    pub triangle: bool,
    pub l1: bool,
    pub r1: bool,
    pub create: bool,
    pub options: bool,
    pub l3: bool,
    pub r3: bool,
    pub ps: bool,
}

/// État au repos : sticks centrés, rien d'appuyé.
impl Default for DualSenseState {
    fn default() -> Self {
        Self {
            left_x: STICK_CENTER,
            left_y: STICK_CENTER,
            right_x: STICK_CENTER,
            right_y: STICK_CENTER,
            l2: 0,
            r2: 0,
            dpad_up: false,
            dpad_down: false,
            dpad_left: false,
            dpad_right: false,
            square: false,
            cross: false,
            circle: false,
            triangle: false,
            l1: false,
            r1: false,
            create: false,
            options: false,
            l3: false,
            r3: false,
            ps: false,
        }
    }
}

pub struct DualSense {
    device: HidDevice,
}

impl DualSense {
    /// Ouvre la première DualSense (ou DualSense Edge) détectée.
    pub fn open(api: &HidApi) -> Result<Self, String> {
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == SONY_VID
                    && matches!(d.product_id(), DUALSENSE_PID | DUALSENSE_EDGE_PID)
            })
            .ok_or("aucune DualSense détectée (branchez-la en USB ou appairez-la en Bluetooth)")?;

        let device = info
            .open_device(api)
            .map_err(|e| format!("ouverture de la manette impossible : {e}"))?;

        // En Bluetooth, la manette n'envoie que le rapport simple tant qu'on n'a pas lu ce rapport.
        let mut buf = [0u8; 41];
        buf[0] = FEATURE_REPORT_CALIBRATION;
        let _ = device.get_feature_report(&mut buf);

        Ok(Self { device })
    }

    /// Attend un rapport d'entrée. `Ok(None)` si le délai expire ou si le rapport est inconnu.
    pub fn read_state(&self, timeout_ms: i32) -> Result<Option<DualSenseState>, String> {
        let mut buf = [0u8; 128];
        let len = self
            .device
            .read_timeout(&mut buf, timeout_ms)
            .map_err(|e| format!("lecture HID : {e}"))?;
        Ok(parse_report(&buf[..len]))
    }
}

/// Décode un rapport d'entrée brut. Renvoie `None` pour un rapport non reconnu ou trop court.
pub fn parse_report(report: &[u8]) -> Option<DualSenseState> {
    match (report.first().copied()?, report.len()) {
        (REPORT_ID_USB_OR_BT_SIMPLE, 64) => parse_full(&report[1..]),
        (REPORT_ID_BT_FULL, len) if len >= 11 => parse_full(&report[2..]),
        (REPORT_ID_USB_OR_BT_SIMPLE, 10) => parse_bt_simple(&report[1..]),
        _ => None,
    }
}

/// `data` commence juste après l'identifiant (et l'octet d'en-tête en Bluetooth).
fn parse_full(data: &[u8]) -> Option<DualSenseState> {
    if data.len() < 10 {
        return None;
    }
    Some(build_state(
        [data[0], data[1], data[2], data[3]],
        data[4],
        data[5],
        [data[7], data[8], data[9]],
    ))
}

/// Rapport Bluetooth simple : sticks, puis boutons, puis gâchettes.
fn parse_bt_simple(data: &[u8]) -> Option<DualSenseState> {
    if data.len() < 9 {
        return None;
    }
    Some(build_state(
        [data[0], data[1], data[2], data[3]],
        data[7],
        data[8],
        [data[4], data[5], data[6]],
    ))
}

fn build_state(sticks: [u8; 4], l2: u8, r2: u8, buttons: [u8; 3]) -> DualSenseState {
    let [face, shoulder, system] = buttons;
    let hat = face & 0x0F;
    let has = |mask: u8, bit: u8| mask & bit != 0;

    // Croix directionnelle : 0 = haut, puis sens horaire par pas de 45 degrés, 8 = neutre.
    let up = matches!(hat, 7 | 0 | 1);
    let right = matches!(hat, 1..=3);
    let down = matches!(hat, 3..=5);
    let left = matches!(hat, 5..=7);

    DualSenseState {
        left_x: sticks[0],
        left_y: sticks[1],
        right_x: sticks[2],
        right_y: sticks[3],
        l2,
        r2,
        dpad_up: up,
        dpad_down: down,
        dpad_left: left,
        dpad_right: right,
        square: has(face, 0x10),
        cross: has(face, 0x20),
        circle: has(face, 0x40),
        triangle: has(face, 0x80),
        l1: has(shoulder, 0x01),
        r1: has(shoulder, 0x02),
        create: has(shoulder, 0x10),
        options: has(shoulder, 0x20),
        l3: has(shoulder, 0x40),
        r3: has(shoulder, 0x80),
        ps: has(system, 0x01),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DPAD_NEUTRAL: u8 = 8;

    fn usb_report(face: u8, shoulder: u8, system: u8) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = REPORT_ID_USB_OR_BT_SIMPLE;
        r[1..5].copy_from_slice(&[128, 128, 128, 128]);
        r[8] = face;
        r[9] = shoulder;
        r[10] = system;
        r
    }

    #[test]
    fn neutral_dpad_presses_nothing() {
        let s = parse_report(&usb_report(DPAD_NEUTRAL, 0, 0)).unwrap();
        assert!(!s.dpad_up && !s.dpad_down && !s.dpad_left && !s.dpad_right);
    }

    #[test]
    fn dpad_diagonal_presses_two_directions() {
        let s = parse_report(&usb_report(1, 0, 0)).unwrap(); // haut-droite
        assert!(s.dpad_up && s.dpad_right && !s.dpad_down && !s.dpad_left);
        let s = parse_report(&usb_report(5, 0, 0)).unwrap(); // bas-gauche
        assert!(s.dpad_down && s.dpad_left && !s.dpad_up && !s.dpad_right);
    }

    #[test]
    fn face_and_shoulder_buttons() {
        let s = parse_report(&usb_report(DPAD_NEUTRAL | 0x20, 0x01 | 0x20, 0x01)).unwrap();
        assert!(s.cross && s.l1 && s.options && s.ps);
        assert!(!s.circle && !s.r1 && !s.create);
    }

    #[test]
    fn bluetooth_full_report_is_shifted_by_one_byte() {
        let usb = usb_report(DPAD_NEUTRAL | 0x80, 0x02, 0);
        let mut bt = vec![0u8; 78];
        bt[0] = REPORT_ID_BT_FULL;
        bt[2..2 + 63].copy_from_slice(&usb[1..]);
        assert_eq!(parse_report(&bt), parse_report(&usb));
    }

    #[test]
    fn unknown_or_short_reports_are_ignored() {
        assert_eq!(parse_report(&[]), None);
        assert_eq!(parse_report(&[0x42; 64]), None);
        assert_eq!(parse_report(&[REPORT_ID_USB_OR_BT_SIMPLE, 0, 0]), None);
    }
}
