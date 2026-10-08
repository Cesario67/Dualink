//! Installation en un clic des composants externes : ViGEmBus (obligatoire) et HidHide (optionnel).
//!
//! L'installation se fait dans une copie élevée de Dualink (`--install-dependencies`), pour qu'une
//! seule invite UAC suffise. Pour chaque composant manquant :
//! 1. si l'installateur est dans le dossier `redist\` à côté de `dualink.exe` (archive de release),
//!    on le lance en silencieux : aucune connexion Internet n'est nécessaire ;
//! 2. sinon on passe par `winget`, qui télécharge l'installateur officiel.
//!
//! Les traces sont conservées dans `%LOCALAPPDATA%\Dualink\install.log`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use vigem_client::Client;

use crate::elevate::relaunch_elevated;
use crate::hidhide::HidHide;

pub const INSTALL_ARG: &str = "--install-dependencies";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Code de sortie d'un installateur MSI/Burn qui a réussi mais demande un redémarrage.
const EXIT_SUCCESS_REBOOT_REQUIRED: i32 = 3010;

/// Un composant installable.
struct Package {
    name: &'static str,
    winget_id: &'static str,
    /// Début du nom de l'installateur dans `redist\` (la version varie).
    redist_prefix: &'static str,
    is_installed: fn() -> bool,
}

const PACKAGES: [Package; 2] = [
    Package {
        name: "ViGEmBus",
        winget_id: "ViGEm.ViGEmBus",
        redist_prefix: "ViGEmBus_",
        is_installed: || Client::connect().is_ok(),
    },
    Package {
        name: "HidHide",
        winget_id: "Nefarius.HidHide",
        redist_prefix: "HidHide_",
        is_installed: || HidHide::detect().is_some(),
    },
];

/// Demande l'installation (invite UAC). La détection des composants est faite en continu par
/// le thread de fond : l'interface se met à jour toute seule une fois l'installation terminée.
pub fn request_install() -> Result<(), String> {
    relaunch_elevated(INSTALL_ARG)
}

fn data_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Dualink");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn open_log() -> Option<File> {
    OpenOptions::new().create(true).append(true).open(data_dir()?.join("install.log")).ok()
}

fn log_line(text: &str) {
    if let Some(mut log) = open_log() {
        let _ = writeln!(log, "{text}");
    }
}

/// Installateur fourni dans `redist\`, à côté de l'exécutable.
fn bundled_installer(redist_prefix: &str) -> Option<PathBuf> {
    let redist = std::env::current_exe().ok()?.parent()?.join("redist");
    find_installer(&redist, redist_prefix)
}

fn find_installer(redist: &Path, prefix: &str) -> Option<PathBuf> {
    fs::read_dir(redist)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
                && path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix))
        })
}

fn output_streams() -> (Stdio, Stdio) {
    match (open_log(), open_log()) {
        (Some(out), Some(err)) => (Stdio::from(out), Stdio::from(err)),
        _ => (Stdio::null(), Stdio::null()),
    }
}

/// Les installateurs de ViGEmBus et HidHide sont des bundles WiX Burn : `/quiet /norestart`.
fn run_bundled(installer: &Path, package: &Package) -> std::io::Result<ExitStatus> {
    let (stdout, stderr) = output_streams();
    let mut command = Command::new(installer);
    command.args(["/quiet", "/norestart"]);
    if let Some(dir) = data_dir() {
        command.arg("/log").arg(dir.join(format!("install-{}.log", package.name)));
    }
    command.stdout(stdout).stderr(stderr).creation_flags(CREATE_NO_WINDOW).status()
}

fn run_winget(package: &Package) -> std::io::Result<ExitStatus> {
    let (stdout, stderr) = output_streams();
    Command::new("winget")
        .args([
            "install",
            "--id",
            package.winget_id,
            "--exact",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .stdout(stdout)
        .stderr(stderr)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
}

fn succeeded(status: std::io::Result<ExitStatus>) -> bool {
    status.is_ok_and(|s| s.success() || s.code() == Some(EXIT_SUCCESS_REBOOT_REQUIRED))
}

/// Exécuté par la copie élevée. Renvoie le code de sortie du processus (0 = tout est installé).
pub fn run_elevated_helper() -> i32 {
    let mut all_ok = true;
    for package in PACKAGES.iter().filter(|p| !(p.is_installed)()) {
        let status = match bundled_installer(package.redist_prefix) {
            Some(installer) => {
                log_line(&format!("{} : installateur local {}", package.name, installer.display()));
                run_bundled(&installer, package)
            }
            None => {
                log_line(&format!("{} : pas d'installateur local, winget ({})", package.name, package.winget_id));
                run_winget(package)
            }
        };
        let ok = succeeded(status);
        log_line(&format!("{} : {}", package.name, if ok { "ok" } else { "échec" }));
        all_ok &= ok;
    }
    if all_ok { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_bundled_installer_by_prefix_whatever_the_version() {
        let dir = std::env::temp_dir().join(format!("dualink-redist-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for file in ["ViGEmBus_1.22.0_x64_x86_arm64.exe", "HidHide_1.5.230_x64.exe", "notes.txt"] {
            fs::write(dir.join(file), b"").unwrap();
        }

        assert!(find_installer(&dir, "ViGEmBus_").is_some_and(|p| p.ends_with("ViGEmBus_1.22.0_x64_x86_arm64.exe")));
        assert!(find_installer(&dir, "HidHide_").is_some());
        assert!(find_installer(&dir, "Autre_").is_none());
        assert!(find_installer(&dir.join("absent"), "HidHide_").is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
