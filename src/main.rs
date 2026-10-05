mod dualsense;
mod xbox;

use std::process::ExitCode;

use hidapi::HidApi;

use dualsense::DualSense;
use xbox::VirtualXbox360;

fn run() -> Result<(), String> {
    let api = HidApi::new().map_err(|e| format!("initialisation HID : {e}"))?;
    let pad = DualSense::open(&api)?;
    let mut virtual_pad = VirtualXbox360::plug_in()?;

    println!("DualSense connectée, manette Xbox 360 virtuelle branchée. Ctrl+C pour quitter.");

    let mut last = None;
    loop {
        // Le timeout évite un blocage si la manette se tait ; on ne renvoie que les changements.
        let Some(state) = pad.read_state(100)? else { continue };
        if last != Some(state) {
            virtual_pad.send(&state)?;
            last = Some(state);
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}
