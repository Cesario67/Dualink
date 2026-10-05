//! Thread de fond : lit la DualSense, alimente la manette virtuelle et publie un état
//! consultable par l'interface. Gère la reconnexion et l'activation/désactivation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use hidapi::HidApi;

use crate::dualsense::{DualSense, DualSenseState};
use crate::xbox::VirtualXbox360;

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
}

struct Shared {
    snapshot: Mutex<Snapshot>,
    enabled: AtomicBool,
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
            }),
            enabled: AtomicBool::new(true),
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
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.shared.quit.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Met à jour l'état partagé et ne notifie que si quelque chose a changé.
fn publish(shared: &Shared, notify: &dyn Fn(), status: Status, input: DualSenseState) {
    let next = Snapshot { status, input };
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
    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            let status = Status::Error(format!("initialisation HID : {e}"));
            publish(shared, notify, status, DualSenseState::default());
            return;
        }
    };

    let mut pad: Option<DualSense> = None;
    let mut virtual_pad: Option<VirtualXbox360> = None;
    let mut last_sent: Option<DualSenseState> = None;
    let idle = DualSenseState::default();

    while !shared.quit.load(Ordering::Relaxed) {
        if !shared.enabled.load(Ordering::Relaxed) {
            pad = None;
            virtual_pad = None;
            last_sent = None;
            publish(shared, notify, Status::Disabled, idle);
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        if pad.is_none() {
            // Sans rafraîchissement, hidapi ne voit pas une manette branchée après son démarrage.
            let _ = api.refresh_devices();
            match DualSense::open(&api) {
                Ok(opened) => pad = Some(opened),
                Err(_) => {
                    publish(shared, notify, Status::WaitingForController, idle);
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }

        // La manette virtuelle n'existe que tant que la vraie est là : pas de fantôme Xbox.
        if virtual_pad.is_none() {
            match VirtualXbox360::plug_in() {
                Ok(plugged) => virtual_pad = Some(plugged),
                Err(e) => {
                    publish(shared, notify, Status::ViGemUnavailable(e), idle);
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            }
        }

        let (Some(device), Some(out)) = (pad.as_ref(), virtual_pad.as_mut()) else {
            continue;
        };
        match device.read_state(100) {
            Ok(Some(state)) => {
                publish(shared, notify, Status::Active, state);
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
                publish(shared, notify, Status::WaitingForController, idle);
            }
        }
    }
}
