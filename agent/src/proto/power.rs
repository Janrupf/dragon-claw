use super::PowerAction as ProtoPowerAction;
use crate::pal::power::PowerAction as PalPowerAction;

impl From<PalPowerAction> for ProtoPowerAction {
    fn from(value: PalPowerAction) -> Self {
        match value {
            PalPowerAction::PowerOff => Self::PowerOff,
            PalPowerAction::Reboot => Self::Reboot,
            PalPowerAction::RebootToFirmware => Self::RebootToFirmware,
            PalPowerAction::Lock => Self::Lock,
            PalPowerAction::LogOut => Self::LogOut,
            PalPowerAction::Suspend => Self::Suspend,
            PalPowerAction::Hibernate => Self::Hibernate,
            PalPowerAction::HybridSuspend => Self::HybridSuspend,
        }
    }
}

impl From<ProtoPowerAction> for PalPowerAction {
    fn from(value: ProtoPowerAction) -> Self {
        match value {
            ProtoPowerAction::PowerOff => Self::PowerOff,
            ProtoPowerAction::Reboot => Self::Reboot,
            ProtoPowerAction::RebootToFirmware => Self::RebootToFirmware,
            ProtoPowerAction::Lock => Self::Lock,
            ProtoPowerAction::LogOut => Self::LogOut,
            ProtoPowerAction::Suspend => Self::Suspend,
            ProtoPowerAction::Hibernate => Self::Hibernate,
            ProtoPowerAction::HybridSuspend => Self::HybridSuspend,
        }
    }
}
