//! Masquage de la DualSense physique via HidHide (pilote de filtre de Nefarius).
//!
//! On passe par `HidHideCLI.exe`, livré avec HidHide, plutôt que par les IOCTL du pilote.
//! La configuration de HidHide est partagée avec les autres outils (DSX, DS4Windows) : on ne
//! retire que ce que Dualink a lui-même ajouté. Ces ajouts sont consignés dans un fichier, pour
//! pouvoir tout défaire au prochain lancement si Dualink est tué avant d'avoir restauré.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SONY_IDS: [&str; 2] = ["VID_054C&PID_0CE6", "VID_054C&PID_0DF2"];
/// Forme des chemins d'instance Bluetooth : `...VID&0002054C_PID&0CE6...`.
const SONY_IDS_BLUETOOTH: [&str; 2] = ["054C_PID&0CE6", "054C_PID&0DF2"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HidHideState {
    NotInstalled,
    Off,
    Hidden,
    Error(String),
}

/// Ce que Dualink a modifié dans la configuration de HidHide.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Undo {
    registered_app: Option<String>,
    hidden_devices: Vec<String>,
    cloak_turned_on: bool,
}

impl Undo {
    fn is_empty(&self) -> bool {
        self.registered_app.is_none() && self.hidden_devices.is_empty() && !self.cloak_turned_on
    }

    fn serialize(&self) -> String {
        let mut out = String::new();
        if let Some(app) = &self.registered_app {
            out.push_str(&format!("app\t{app}\n"));
        }
        for device in &self.hidden_devices {
            out.push_str(&format!("dev\t{device}\n"));
        }
        if self.cloak_turned_on {
            out.push_str("cloak\n");
        }
        out
    }

    fn parse(text: &str) -> Self {
        let mut undo = Self::default();
        for line in text.lines() {
            match line.split_once('\t') {
                Some(("app", path)) => undo.registered_app = Some(path.to_owned()),
                Some(("dev", path)) => undo.hidden_devices.push(path.to_owned()),
                None if line == "cloak" => undo.cloak_turned_on = true,
                _ => {}
            }
        }
        undo
    }
}

pub struct HidHide {
    cli: PathBuf,
    journal: PathBuf,
    undo: Undo,
}

impl HidHide {
    /// `None` si HidHide n'est pas installé.
    pub fn detect() -> Option<Self> {
        let program_files = std::env::var_os("ProgramFiles")?;
        let cli = Path::new(&program_files)
            .join(r"Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe");
        if !cli.is_file() {
            return None;
        }
        let local = std::env::var_os("LOCALAPPDATA")?;
        let journal = Path::new(&local).join("Dualink").join("hidhide-undo.txt");
        Some(Self { cli, journal, undo: Undo::default() })
    }

    /// Défait les changements d'une exécution précédente interrompue (plantage, arrêt forcé).
    pub fn recover_leftover(&mut self) -> Result<(), String> {
        if let Ok(text) = fs::read_to_string(&self.journal) {
            self.undo = Undo::parse(&text);
            self.restore()?;
        }
        Ok(())
    }

    /// Masque les DualSense présentes aux autres applications, en gardant Dualink autorisé.
    pub fn apply(&mut self) -> Result<(), String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("chemin de Dualink introuvable : {e}"))?
            .to_string_lossy()
            .into_owned();

        let registered = contains_ignore_case(&parse_commands(&self.run(&["--app-list"])?), &exe);
        let hidden = parse_commands(&self.run(&["--dev-list"])?);
        let cloak_on = self.run(&["--cloak-state"])?.contains("--cloak-on");
        let devices = parse_dualsense_devices(&self.run(&["--dev-gaming"])?);

        let mut undo = Undo::default();
        let mut args: Vec<String> = Vec::new();
        if !registered {
            args.extend(["--app-reg".into(), exe.clone()]);
            undo.registered_app = Some(exe);
        }
        for device in devices {
            if !contains_ignore_case(&hidden, &device) {
                args.extend(["--dev-hide".into(), device.clone()]);
                undo.hidden_devices.push(device);
            }
        }
        if !cloak_on {
            args.push("--cloak-on".into());
            undo.cloak_turned_on = true;
        }
        if args.is_empty() {
            return Ok(());
        }

        // Journal d'abord : si on est tué entre les deux, le prochain lancement saura défaire.
        self.undo = undo;
        self.write_journal();
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&refs).map(|_| ())
    }

    /// Annule ce que Dualink a ajouté, et seulement cela.
    pub fn restore(&mut self) -> Result<(), String> {
        if self.undo.is_empty() {
            self.clear_journal();
            return Ok(());
        }
        let mut args: Vec<String> = Vec::new();
        for device in &self.undo.hidden_devices {
            args.extend(["--dev-unhide".into(), device.clone()]);
        }
        if self.undo.cloak_turned_on {
            // On ne coupe le masquage que s'il ne reste plus rien de caché pour un autre outil.
            let still_hidden = parse_commands(&self.run(&["--dev-list"])?)
                .into_iter()
                .any(|d| !contains_ignore_case(&self.undo.hidden_devices, &d));
            if !still_hidden {
                args.push("--cloak-off".into());
            }
        }
        if let Some(app) = &self.undo.registered_app {
            args.extend(["--app-unreg".into(), app.clone()]);
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&refs)?;
        self.undo = Undo::default();
        self.clear_journal();
        Ok(())
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.cli)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("HidHideCLI : {e}"))?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            Ok(text)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            let detail = if err.trim().is_empty() { text.trim().to_owned() } else { err.trim().to_owned() };
            Err(format!(
                "HidHideCLI a échoué (droits administrateur nécessaires ?) : {detail}"
            ))
        }
    }

    fn write_journal(&self) {
        if let Some(dir) = self.journal.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&self.journal, self.undo.serialize());
    }

    fn clear_journal(&self) {
        let _ = fs::remove_file(&self.journal);
    }
}

fn contains_ignore_case(list: &[String], item: &str) -> bool {
    list.iter().any(|s| s.eq_ignore_ascii_case(item))
}

/// Extrait les arguments entre guillemets d'une sortie de la forme `--dev-hide "chemin"`.
fn parse_commands(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let start = line.find('"')? + 1;
            let end = line.rfind('"')?;
            (end > start).then(|| line[start..end].to_owned())
        })
        .collect()
}

fn is_dualsense_path(path: &str) -> bool {
    let upper = path.to_ascii_uppercase();
    SONY_IDS.iter().chain(SONY_IDS_BLUETOOTH.iter()).any(|id| upper.contains(id))
}

/// Chemins d'instance des interfaces « manette » de DualSense actuellement présentes.
fn parse_dualsense_devices(json: &str) -> Vec<String> {
    let Ok(Value::Array(groups)) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    groups
        .iter()
        .filter_map(|group| group.get("devices")?.as_array())
        .flatten()
        .filter(|device| {
            device.get("present").and_then(Value::as_bool) == Some(true)
                && device.get("gamingDevice").and_then(Value::as_bool) == Some(true)
        })
        .filter_map(|device| device.get("deviceInstancePath")?.as_str())
        .filter(|path| is_dualsense_path(path))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[ { "friendlyName" : "Controller (XBOX 360 For Windows)" , "devices" : [
{ "present" : true , "gamingDevice" : true ,
  "deviceInstancePath" : "HID\\VID_045E&PID_028E&IG_00\\4&31a8b9e1&0&0000" } ] } ,
{ "friendlyName" : "DualSense" , "devices" : [
{ "present" : true , "gamingDevice" : true ,
  "deviceInstancePath" : "HID\\VID_054C&PID_0CE6&MI_03\\8&3B676666&0&0000" } ,
{ "present" : false , "gamingDevice" : true ,
  "deviceInstancePath" : "HID\\VID_054C&PID_0CE6&MI_03\\9&AAAAAAAA&0&0000" } ,
{ "present" : true , "gamingDevice" : false ,
  "deviceInstancePath" : "HID\\VID_054C&PID_0CE6&MI_00\\7&BBBBBBBB&0&0000" } ] } ]"#;

    #[test]
    fn keeps_only_present_gaming_dualsense_interfaces() {
        assert_eq!(
            parse_dualsense_devices(SAMPLE),
            vec![r"HID\VID_054C&PID_0CE6&MI_03\8&3B676666&0&0000".to_owned()]
        );
    }

    #[test]
    fn bluetooth_instance_paths_are_recognised() {
        assert!(is_dualsense_path(r"HID\{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&0CE6\9&1&0&0000"));
        assert!(!is_dualsense_path(r"HID\VID_045E&PID_028E&IG_00\4&31a8b9e1&0&0000"));
    }

    #[test]
    fn invalid_json_gives_no_devices() {
        assert!(parse_dualsense_devices("not json").is_empty());
    }

    #[test]
    fn parses_cli_listings() {
        let out = "--dev-hide \"HID\\VID_054C&PID_0CE6&MI_03\\8&3B&0&0000\"\n--dev-hide \"X\"\n";
        assert_eq!(parse_commands(out), vec![r"HID\VID_054C&PID_0CE6&MI_03\8&3B&0&0000", "X"]);
        assert!(parse_commands("--cloak-on").is_empty());
    }

    #[test]
    fn undo_journal_round_trips() {
        let undo = Undo {
            registered_app: Some(r"C:\Dualink\dualink.exe".into()),
            hidden_devices: vec![r"HID\A".into(), r"HID\B".into()],
            cloak_turned_on: true,
        };
        assert_eq!(Undo::parse(&undo.serialize()), undo);
        assert!(Undo::parse("").is_empty());
    }
}
