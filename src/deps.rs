//! Installation des composants externes : ViGEmBus (obligatoire) et HidHide (recommandé).
//!
//! L'installation se fait dans une copie élevée de Dualink (`--install-dependencies <liste>`), pour
//! qu'une seule invite UAC suffise. Pour chaque composant manquant, dans l'ordre :
//! 1. l'installateur fourni dans `redist\` à côté de `dualink.exe` (installateur Setup, archive) : hors ligne ;
//! 2. l'installateur officiel téléchargé depuis GitHub, vérifié par son SHA256 ;
//! 3. `winget`, en dernier recours.
//!
//! Les traces sont conservées dans `%LOCALAPPDATA%\Dualink\install.log`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use sha2::{Digest, Sha256};
use vigem_client::Client;

use crate::elevate::relaunch_elevated;
use crate::hidhide::HidHide;

pub const INSTALL_ARG: &str = "--install-dependencies";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Code de sortie d'un installateur MSI/Burn qui a réussi mais demande un redémarrage.
const EXIT_SUCCESS_REBOOT_REQUIRED: i32 = 3010;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    ViGEmBus,
    HidHide,
}

/// Où trouver et comment vérifier l'installateur d'un composant.
struct Package {
    name: &'static str,
    cli_name: &'static str,
    winget_id: &'static str,
    /// Début du nom de l'installateur dans `redist\` (la version varie).
    redist_prefix: &'static str,
    file: &'static str,
    url: &'static str,
    /// Empreinte SHA256 attendue du fichier téléchargé (identique à celle des manifestes winget).
    sha256: &'static str,
}

const VIGEMBUS: Package = Package {
    name: "ViGEmBus",
    cli_name: "vigem",
    winget_id: "ViGEm.ViGEmBus",
    redist_prefix: "ViGEmBus_",
    file: "ViGEmBus_1.22.0_x64_x86_arm64.exe",
    url: "https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe",
    sha256: "89220a7865076b342892f98865f3499fb7c4cfd673159e89d352c360fd014c6a",
};

const HIDHIDE: Package = Package {
    name: "HidHide",
    cli_name: "hidhide",
    winget_id: "Nefarius.HidHide",
    redist_prefix: "HidHide_",
    file: "HidHide_1.5.230_x64.exe",
    url: "https://github.com/nefarius/HidHide/releases/download/v1.5.230.0/HidHide_1.5.230_x64.exe",
    sha256: "f4bbbcb82e6258641b887c74bc81c4c5f66e4aa811808dfc304347687b7605f6",
};

impl Component {
    fn package(self) -> &'static Package {
        match self {
            Self::ViGEmBus => &VIGEMBUS,
            Self::HidHide => &HIDHIDE,
        }
    }

    fn from_cli_name(name: &str) -> Option<Self> {
        [Self::ViGEmBus, Self::HidHide].into_iter().find(|c| c.package().cli_name == name)
    }

    pub fn is_installed(self) -> bool {
        match self {
            Self::ViGEmBus => Client::connect().is_ok(),
            Self::HidHide => HidHide::detect().is_some(),
        }
    }
}

/// Demande l'installation des composants donnés (invite UAC). La détection est faite en continu
/// par le thread de fond : l'interface se met à jour toute seule une fois l'installation terminée.
pub fn request_install(components: &[Component]) -> Result<(), String> {
    let list: Vec<&str> = components.iter().map(|c| c.package().cli_name).collect();
    relaunch_elevated(&format!("{INSTALL_ARG} {}", list.join(",")))
}

/// Vrai si des installateurs sont fournis dans `redist\` à côté de l'exécutable (installation hors ligne).
pub fn has_bundled_installers() -> bool {
    [&VIGEMBUS, &HIDHIDE].iter().any(|p| bundled_installer(p.redist_prefix).is_some())
}

/// Chemin du fichier de traces de l'installation.
pub fn log_path() -> Option<PathBuf> {
    Some(data_dir()?.join("install.log"))
}

fn data_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Dualink");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn open_log() -> Option<File> {
    OpenOptions::new().create(true).append(true).open(log_path()?).ok()
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

/// SHA256 d'un fichier, en hexadécimal minuscule.
fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let digest = Sha256::digest(fs::read(path)?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Télécharge l'installateur officiel (curl.exe, fourni avec Windows 10/11) et vérifie son SHA256.
/// Le fichier est supprimé si l'empreinte ne correspond pas.
fn download_verified(package: &Package) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("Dualink-install");
    fs::create_dir_all(&dir).map_err(|e| format!("dossier temporaire : {e}"))?;
    let target = dir.join(package.file);

    let (stdout, stderr) = output_streams();
    let status = Command::new("curl.exe")
        .args(["--location", "--fail", "--silent", "--show-error", "--output"])
        .arg(&target)
        .arg(package.url)
        .stdout(stdout)
        .stderr(stderr)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("curl.exe : {e}"))?;
    if !status.success() {
        return Err(format!("téléchargement échoué (code {:?})", status.code()));
    }

    let actual = sha256_hex(&target).map_err(|e| format!("lecture du fichier téléchargé : {e}"))?;
    if actual != package.sha256 {
        let _ = fs::remove_file(&target);
        return Err(format!("empreinte SHA256 inattendue ({actual}), fichier supprimé"));
    }
    Ok(target)
}

/// Les installateurs de ViGEmBus et HidHide sont des bundles WiX Burn : `/quiet /norestart`.
fn run_installer(installer: &Path, package: &Package) -> std::io::Result<ExitStatus> {
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

fn install(package: &Package) -> bool {
    if let Some(installer) = bundled_installer(package.redist_prefix) {
        log_line(&format!("{} : installateur fourni {}", package.name, installer.display()));
        return succeeded(run_installer(&installer, package));
    }

    log_line(&format!("{} : téléchargement de {}", package.name, package.url));
    match download_verified(package) {
        Ok(installer) => {
            log_line(&format!("{} : SHA256 vérifié, installation", package.name));
            let ok = succeeded(run_installer(&installer, package));
            let _ = fs::remove_file(installer);
            return ok;
        }
        Err(e) => log_line(&format!("{} : {e}", package.name)),
    }

    log_line(&format!("{} : repli sur winget ({})", package.name, package.winget_id));
    succeeded(run_winget(package))
}

/// Exécuté par la copie élevée. `list` : composants séparés par des virgules (`vigem,hidhide`).
/// Renvoie le code de sortie du processus (0 = tout est installé).
pub fn run_elevated_helper(list: &str) -> i32 {
    let mut all_ok = true;
    for component in list.split(',').filter_map(Component::from_cli_name) {
        if component.is_installed() {
            continue;
        }
        let package = component.package();
        let ok = install(package);
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

    #[test]
    fn sha256_matches_the_known_digest_of_abc() {
        let file = std::env::temp_dir().join(format!("dualink-sha-test-{}", std::process::id()));
        fs::write(&file, b"abc").unwrap();
        assert_eq!(
            sha256_hex(&file).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = fs::remove_file(file);
    }

    #[test]
    fn cli_names_round_trip() {
        assert_eq!(Component::from_cli_name("vigem"), Some(Component::ViGEmBus));
        assert_eq!(Component::from_cli_name("hidhide"), Some(Component::HidHide));
        assert_eq!(Component::from_cli_name("autre"), None);
    }

    /// Test réseau, à lancer à la main : `cargo test -- --ignored download`.
    /// Télécharge réellement HidHide et vérifie l'empreinte, sans l'installer.
    #[test]
    #[ignore]
    fn download_verifies_the_real_hidhide_installer() {
        let path = download_verified(&HIDHIDE).expect("téléchargement vérifié");
        assert!(path.is_file());
        let _ = fs::remove_file(path);
    }
}
