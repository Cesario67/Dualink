//! Installation en un clic des composants externes : ViGEmBus (obligatoire) et HidHide (optionnel).
//!
//! On passe par `winget`, présent sur Windows 10/11 récents : il télécharge les installateurs
//! officiels et les exécute en silencieux. L'installation se fait dans une copie élevée de Dualink
//! (`--install-dependencies`), pour qu'une seule invite UAC suffise. La sortie de winget est
//! conservée dans `%LOCALAPPDATA%\Dualink\install.log`.

use std::fs::{self, File, OpenOptions};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use vigem_client::Client;

use crate::elevate::relaunch_elevated;
use crate::hidhide::HidHide;

pub const INSTALL_ARG: &str = "--install-dependencies";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const VIGEMBUS_ID: &str = "ViGEm.ViGEmBus";
const HIDHIDE_ID: &str = "Nefarius.HidHide";

/// Demande l'installation (invite UAC). La détection des composants est faite en continu par
/// le thread de fond : l'interface se met à jour toute seule une fois l'installation terminée.
pub fn request_install() -> Result<(), String> {
    relaunch_elevated(INSTALL_ARG)
}

/// Identifiants winget des composants qui manquent.
fn missing_packages() -> Vec<&'static str> {
    let mut missing = Vec::new();
    if Client::connect().is_err() {
        missing.push(VIGEMBUS_ID);
    }
    if HidHide::detect().is_none() {
        missing.push(HIDHIDE_ID);
    }
    missing
}

fn log_path() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Dualink");
    fs::create_dir_all(&dir).ok()?;
    Some(dir.join("install.log"))
}

fn open_log() -> Option<File> {
    OpenOptions::new().create(true).append(true).open(log_path()?).ok()
}

/// Exécuté par la copie élevée. Renvoie le code de sortie du processus (0 = tout est installé).
pub fn run_elevated_helper() -> i32 {
    let mut all_ok = true;
    for id in missing_packages() {
        let (stdout, stderr) = match (open_log(), open_log()) {
            (Some(out), Some(err)) => (Stdio::from(out), Stdio::from(err)),
            _ => (Stdio::null(), Stdio::null()),
        };
        let status = Command::new("winget")
            .args([
                "install",
                "--id",
                id,
                "--exact",
                "--silent",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--disable-interactivity",
            ])
            .stdout(stdout)
            .stderr(stderr)
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        all_ok &= status.is_ok_and(|s| s.success());
    }
    if all_ok { 0 } else { 1 }
}
