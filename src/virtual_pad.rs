//! Manette virtuelle exposée aux jeux : Xbox 360 ou DualShock 4, au choix.

use crate::dualsense::DualSenseState;
use crate::ds4::VirtualDs4;
use crate::xbox::VirtualXbox360;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emulation {
    Xbox360,
    DualShock4,
}

impl Emulation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Xbox360 => "Xbox 360",
            Self::DualShock4 => "DualShock 4",
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::Xbox360 => 0,
            Self::DualShock4 => 1,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        if value == 1 { Self::DualShock4 } else { Self::Xbox360 }
    }
}

/// Ordre de vibration envoyé par un jeu : `large` gros moteur, `small` petit moteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rumble {
    pub large: u8,
    pub small: u8,
}

pub enum VirtualPad {
    Xbox(VirtualXbox360),
    Ds4(VirtualDs4),
}

impl VirtualPad {
    pub fn plug_in(mode: Emulation) -> Result<Self, String> {
        match mode {
            Emulation::Xbox360 => VirtualXbox360::plug_in().map(Self::Xbox),
            Emulation::DualShock4 => VirtualDs4::plug_in().map(Self::Ds4),
        }
    }

    pub fn mode(&self) -> Emulation {
        match self {
            Self::Xbox(_) => Emulation::Xbox360,
            Self::Ds4(_) => Emulation::DualShock4,
        }
    }

    pub fn send(&mut self, state: &DualSenseState) -> Result<(), String> {
        match self {
            Self::Xbox(pad) => pad.send(state),
            Self::Ds4(pad) => pad.send(state),
        }
    }

    /// Dernier ordre de vibration reçu depuis le dernier appel. Toujours `None` en mode DS4 :
    /// `vigem-client` n'expose pas les notifications de vibration pour la DualShock 4.
    pub fn take_rumble(&mut self) -> Option<Rumble> {
        match self {
            Self::Xbox(pad) => pad.take_rumble(),
            Self::Ds4(_) => None,
        }
    }
}
