//! Construction des rapports de sortie DualSense (vibration), en USB et en Bluetooth.
//!
//! Le format vient de la documentation du pilote Linux `hid-playstation` : un bloc commun de
//! 47 octets, encapsulé tel quel en USB, et accompagné d'un en-tête et d'un CRC32 en Bluetooth.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Usb,
    Bluetooth,
}

const USB_REPORT_ID: u8 = 0x02;
const USB_REPORT_LEN: usize = 63;
const BT_REPORT_ID: u8 = 0x31;
const BT_REPORT_LEN: usize = 78;
/// Étiquette de rapport de sortie en Bluetooth.
const BT_OUTPUT_TAG: u8 = 0x10;
/// Octet ajouté devant les données pour le calcul du CRC Bluetooth.
const BT_CRC_SEED: u8 = 0xA2;

const COMMON_LEN: usize = 47;
/// Active la vibration « compatible » et la sélection haptique (valid_flag0).
const VALID_FLAG0_VIBRATION: u8 = 0x01 | 0x02;
const OFFSET_MOTOR_RIGHT: usize = 2;
const OFFSET_MOTOR_LEFT: usize = 3;

/// Rapport de sortie de vibration. `left` est le gros moteur (basses fréquences), `right` le petit.
/// `seq` n'est utilisé qu'en Bluetooth (compteur sur 4 bits).
pub fn rumble_report(transport: Transport, seq: u8, left: u8, right: u8) -> Vec<u8> {
    let mut common = [0u8; COMMON_LEN];
    common[0] = VALID_FLAG0_VIBRATION;
    common[OFFSET_MOTOR_RIGHT] = right;
    common[OFFSET_MOTOR_LEFT] = left;

    match transport {
        Transport::Usb => {
            let mut report = vec![0u8; USB_REPORT_LEN];
            report[0] = USB_REPORT_ID;
            report[1..=COMMON_LEN].copy_from_slice(&common);
            report
        }
        Transport::Bluetooth => {
            let mut report = vec![0u8; BT_REPORT_LEN];
            report[0] = BT_REPORT_ID;
            report[1] = (seq & 0x0F) << 4;
            report[2] = BT_OUTPUT_TAG;
            report[3..3 + COMMON_LEN].copy_from_slice(&common);

            let crc_at = BT_REPORT_LEN - 4;
            let crc = crc32_with_seed(BT_CRC_SEED, &report[..crc_at]);
            report[crc_at..].copy_from_slice(&crc.to_le_bytes());
            report
        }
    }
}

/// CRC32 (polynôme 0xEDB88320) de `[seed] + data`.
fn crc32_with_seed(seed: u8, data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in std::iter::once(&seed).chain(data) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        // Valeur de référence du CRC-32 standard pour "123456789", sans graine : on neutralise
        // la graine en calculant sur les 9 octets à partir d'une graine égale au premier octet.
        assert_eq!(crc32_with_seed(b'1', b"23456789"), 0xCBF4_3926);
    }

    #[test]
    fn usb_report_places_motors_after_the_report_id() {
        let r = rumble_report(Transport::Usb, 0, 200, 50);
        assert_eq!(r.len(), 63);
        assert_eq!(r[0], 0x02);
        assert_eq!(r[1], 0x03);
        assert_eq!(r[1 + OFFSET_MOTOR_LEFT], 200);
        assert_eq!(r[1 + OFFSET_MOTOR_RIGHT], 50);
    }

    #[test]
    fn bluetooth_report_has_header_and_valid_crc() {
        let r = rumble_report(Transport::Bluetooth, 5, 10, 20);
        assert_eq!((r.len(), r[0], r[1], r[2]), (78, 0x31, 0x50, 0x10));
        assert_eq!(r[3 + OFFSET_MOTOR_LEFT], 10);
        assert_eq!(r[3 + OFFSET_MOTOR_RIGHT], 20);
        let stored = u32::from_le_bytes([r[74], r[75], r[76], r[77]]);
        assert_eq!(stored, crc32_with_seed(0xA2, &r[..74]));
    }

    #[test]
    fn zero_rumble_report_stops_the_motors() {
        let r = rumble_report(Transport::Usb, 0, 0, 0);
        assert_eq!((r[1 + OFFSET_MOTOR_LEFT], r[1 + OFFSET_MOTOR_RIGHT]), (0, 0));
    }
}
