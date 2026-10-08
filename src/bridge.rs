//! Thread de fond : lit la DualSense, alimente la manette virtuelle et publie un état
//! consultable par l'interface. Gère la reconnexion, l'activation/désactivation et le
//! masquage de la vraie manette via HidHide.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use hidapi::HidApi;

use crate::dualsense::{DualSense, DualSenseState};
use crate::hidhide::{HidHide, HidHideState};
use crate::virtual_pad::{Emulation, VirtualPad};

/// Intervalle de recherche de HidHide tant qu'il n'est pas installé.
const DETECTION_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Disabled,
    WaitingForController,
    ViGemUnavailable(String),
    Active,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub status: Status,
    pub input: DualSenseState,
    pub hidhide: HidHideState,
}

struct Shared {
    snapshot: Mutex<Snapshot>,
    enabled: AtomicBool,
    hide_wanted: AtomicBool,
    emulation: AtomicU8,
    quit: AtomicBool,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Snapshot> {
        self.snapshot.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub struct Bridge {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl Bridge {
    /// Démarre le thread. `notify` est appelé à chaque changement visible (pour redessiner l'UI).
    pub fn spawn(notify: impl Fn() + Send + 'static) -> Self {
        let shared = Arc::new(Shared {
            snapshot: Mutex::new(Snapshot {
                status: Status::WaitingForController,
                input: DualSenseState::default(),
                hidhide: HidHideState::Off,
            }),
            enabled: AtomicBool::new(true),
            hide_wanted: AtomicBool::new(true),
            emulation: AtomicU8::new(Emulation::Xbox360.to_u8()),
            quit: AtomicBool::new(false),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::spawn(move || run(&worker_shared, &notify));
        Self { shared, worker: Some(worker) }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared.lock().clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.shared.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.shared.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_hide_wanted(&self) -> bool {
        self.shared.hide_wanted.load(Ordering::Relaxed)
    }

    pub fn set_hide_wanted(&self, wanted: bool) {
        self.shared.hide_wanted.store(wanted, Ordering::Relaxed);
    }

    pub fn emulation(&self) -> Emulation {
        Emulation::from_u8(self.shared.emulation.load(Ordering::Relaxed))
    }

    pub fn set_emulation(&self, mode: Emulation) {
        self.shared.emulation.store(mode.to_u8(), Ordering::Relaxed);
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.shared.quit.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Pilote HidHide : masque ou restaure la manette selon ce qu'on lui demande.
struct Hider {
    hidhide: Option<HidHide>,
    wanted: bool,
    state: HidHideState,
    last_detection: Instant,
}

impl Hider {
    fn new() -> Self {
        let mut hider = Self {
            hidhide: None,
            wanted: false,
            state: HidHideState::NotInstalled,
            last_detection: Instant::now(),
        };
        hider.detect();
        hider
    }

    /// Cherche HidHide et défait les restes d'une exécution précédente interrompue.
    fn detect(&mut self) {
        self.last_detection = Instant::now();
        self.hidhide = HidHide::detect();
        self.state = match self.hidhide.as_mut() {
            None => HidHideState::NotInstalled,
            Some(h) => match h.recover_leftover() {
                Ok(()) => HidHideState::Off,
                Err(e) => HidHideState::Error(e),
            },
        };
    }

    /// Tant que HidHide est absent, le cherche de temps en temps : l'utilisateur peut l'installer
    /// pendant que Dualink tourne.
    fn poll_installation(&mut self) {
        if self.hidhide.is_none() && self.last_detection.elapsed() >= DETECTION_INTERVAL {
            self.detect();
        }
    }

    /// Demande de réappliquer le masquage au prochain `sync(true)` (opération idempotente).
    fn rescan(&mut self) {
        self.wanted = false;
    }

    /// Idempotent : ne relance HidHide que lorsque la demande change, pour ne pas lancer
    /// le CLI en boucle (y compris après un échec, tant que la demande reste la même).
    fn sync(&mut self, want_hidden: bool) {
        let Some(hidhide) = self.hidhide.as_mut() else { return };
        if self.wanted == want_hidden {
            return;
        }
        self.wanted = want_hidden;
        let result = if want_hidden { hidhide.apply() } else { hidhide.restore() };
        self.state = match (result, want_hidden) {
            (Ok(()), true) => HidHideState::Hidden,
            (Ok(()), false) => HidHideState::Off,
            (Err(e), _) => HidHideState::Error(e),
        };
    }
}

/// Met à jour l'état partagé et ne notifie que si quelque chose a changé.
fn publish(shared: &Shared, notify: &dyn Fn(), hider: &Hider, status: Status, input: DualSenseState) {
    let next = Snapshot { status, input, hidhide: hider.state.clone() };
    {
        let mut current = shared.lock();
        if *current == next {
            return;
        }
        *current = next;
    }
    notify();
}

fn run(shared: &Shared, notify: &dyn Fn()) {
    let mut hider = Hider::new();
    let idle = DualSenseState::default();

    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            let status = Status::Error(format!("initialisation HID : {e}"));
            publish(shared, notify, &hider, status, idle);
            return;
        }
    };

    let mut pad: Option<DualSense> = None;
    let mut virtual_pad: Option<VirtualPad> = None;
    let mut last_sent: Option<DualSenseState> = None;

    while !shared.quit.load(Ordering::Relaxed) {
        hider.poll_installation();

        // Le masquage suit la case HidHide et la durée de vie de Dualink, pas l'interrupteur
        // d'émulation : démasquer le temps d'une pause laisserait Steam reprendre la manette.
        if !shared.hide_wanted.load(Ordering::Relaxed) {
            hider.sync(false);
        }

        if !shared.enabled.load(Ordering::Relaxed) {
            pad = None;
            virtual_pad = None;
            last_sent = None;
            publish(shared, notify, &hider, Status::Disabled, idle);
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        if pad.is_none() {
            // Sans rafraîchissement, hidapi ne voit pas une manette branchée après son démarrage.
            let _ = api.refresh_devices();
            match DualSense::open(&api) {
                Ok(opened) => {
                    pad = Some(opened);
                    // Une manette (re)branchée peut avoir un autre chemin d'instance : on le masque aussi.
                    hider.rescan();
                }
                Err(_) => {
                    publish(shared, notify, &hider, Status::WaitingForController, idle);
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }

        // Changement de mode demandé : on débranche la manette virtuelle pour la recréer.
        let mode = Emulation::from_u8(shared.emulation.load(Ordering::Relaxed));
        if virtual_pad.as_ref().is_some_and(|v| v.mode() != mode) {
            virtual_pad = None;
            last_sent = None;
            if let Some(device) = &pad {
                let _ = device.set_rumble(0, 0);
            }
        }

        // La manette virtuelle n'existe que tant que la vraie est là : pas de fantôme.
        if virtual_pad.is_none() {
            match VirtualPad::plug_in(mode) {
                Ok(plugged) => virtual_pad = Some(plugged),
                Err(e) => {
                    publish(shared, notify, &hider, Status::ViGemUnavailable(e), idle);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            }
        }

        // On ne masque qu'une fois la manette ouverte et la virtuelle branchée.
        hider.sync(shared.hide_wanted.load(Ordering::Relaxed));

        let (Some(device), Some(out)) = (pad.as_ref(), virtual_pad.as_mut()) else {
            continue;
        };
        match device.read_state(100) {
            Ok(Some(state)) => {
                publish(shared, notify, &hider, Status::Active, state);
                if last_sent != Some(state) {
                    if out.send(&state).is_err() {
                        virtual_pad = None;
                        last_sent = None;
                        continue;
                    }
                    last_sent = Some(state);
                }
            }
            Ok(None) => {}
            Err(_) => {
                pad = None;
                virtual_pad = None;
                last_sent = None;
                publish(shared, notify, &hider, Status::WaitingForController, idle);
                continue;
            }
        }

        // Vibration demandée par le jeu : on la relaie à la vraie manette.
        if let (Some(device), Some(rumble)) =
            (pad.as_ref(), virtual_pad.as_mut().and_then(VirtualPad::take_rumble))
        {
            let _ = device.set_rumble(rumble.large, rumble.small);
        }
    }

    // Fermeture : la DualSense redevient visible pour les autres applications.
    hider.sync(false);
}
