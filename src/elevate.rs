//! Relance de Dualink avec les droits administrateur (invite UAC) pour une action ponctuelle.
//!
//! L'interface reste non élevée : seules ces actions, exécutées par une copie sans fenêtre,
//! demandent des droits (redémarrer un périphérique, installer un pilote).

use windows_sys::Win32::UI::Shell::ShellExecuteW;

const SW_HIDE: i32 = 0;
/// `ShellExecuteW` renvoie une valeur supérieure à 32 en cas de succès.
const SHELL_EXECUTE_SUCCESS_MIN: isize = 33;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Lance `dualink.exe <arg>` en administrateur. N'attend pas la fin du processus.
pub fn relaunch_elevated(arg: &str) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("chemin de Dualink introuvable : {e}"))?;
    let exe = wide(&exe.to_string_lossy());
    let verb = wide("runas");
    let args = wide(arg);

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
