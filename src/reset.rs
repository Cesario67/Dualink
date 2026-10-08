//! Réinitialisation de la DualSense : équivalent logiciel d'un débranchement puis rebranchement.
//!
//! Redémarrer un périphérique demande les droits administrateur. L'interface reste donc non
//! élevée : elle relance Dualink avec `--reset-controller` via une invite UAC, et c'est cette
//! copie élevée, sans fenêtre, qui redémarre les interfaces HID de la manette.

use std::os::windows::process::CommandExt;
use std::process::Command;

use windows_sys::Win32::UI::Shell::ShellExecuteW;

use crate::hidhide::HidHide;

pub const RESET_ARG: &str = "--reset-controller";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SW_HIDE: i32 = 0;
/// `ShellExecuteW` renvoie une valeur supérieure à 32 en cas de succès.
const SHELL_EXECUTE_SUCCESS_MIN: isize = 33;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Demande la réinitialisation : lance une copie élevée de Dualink (invite UAC).
/// N'attend pas la fin : la reconnexion de la manette est gérée par le thread de fond.
pub fn request_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("chemin de Dualink introuvable : {e}"))?;
    let exe = wide(&exe.to_string_lossy());
    let verb = wide("runas");
    let args = wide(RESET_ARG);

    // Safety : tous les pointeurs sont des chaînes UTF-16 terminées par zéro, valides pendant l'appel.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            exe.as_ptr(),
            args.as_ptr(),
            std::ptr::null(),
            SW_HIDE,
        )
    } as isize;

    if result >= SHELL_EXECUTE_SUCCESS_MIN {
        Ok(())
    } else {
        Err("demande d'élévation refusée ou impossible (invite UAC annulée ?)".to_owned())
    }
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
