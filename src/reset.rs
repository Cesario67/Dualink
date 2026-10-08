//! Réinitialisation de la DualSense : équivalent logiciel d'un débranchement puis rebranchement.
//!
//! Redémarrer un périphérique demande les droits administrateur : le bouton relance Dualink avec
//! `--reset-controller` (invite UAC), et cette copie élevée, sans fenêtre, redémarre les interfaces
//! HID de la manette.

use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::elevate::relaunch_elevated;
use crate::hidhide::HidHide;

pub const RESET_ARG: &str = "--reset-controller";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Demande la réinitialisation. La reconnexion de la manette est gérée par le thread de fond.
pub fn request_elevated() -> Result<(), String> {
    relaunch_elevated(RESET_ARG)
}

/// Exécuté par la copie élevée : redémarre les interfaces HID de la DualSense.
/// Renvoie le code de sortie du processus.
pub fn run_elevated_helper() -> i32 {
    let Some(hidhide) = HidHide::detect() else { return 2 };
    let Ok(devices) = hidhide.dualsense_devices() else { return 3 };
    if devices.is_empty() {
        return 4;
    }

    let mut all_ok = true;
    for device in &devices {
        let status = Command::new("pnputil")
            .args(["/restart-device", device])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        all_ok &= status.is_ok_and(|s| s.success());
    }
    if all_ok { 0 } else { 5 }
}
