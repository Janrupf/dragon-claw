use crate::pal::platform::PlatformError;
use crate::pal::power::{PowerAction, PowerManager};
use crate::pal::PlatformAbstractionError;
use windows::core::{Error as Win32Error, PCWSTR};
use windows::Win32::Foundation::BOOLEAN;
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Shutdown::{
    InitiateSystemShutdownExW, SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
    SHTDN_REASON_MINOR_OTHER,
};

#[derive(Debug)]
pub struct WindowsPowerManager {
    has_shutdown_privilege: bool,
}

impl WindowsPowerManager {
    pub fn new(has_shutdown_privilege: bool) -> Self {
        Self {
            has_shutdown_privilege,
        }
    }

    fn do_shutdown(&self, reboot: bool) -> Result<(), PlatformError> {
        unsafe {
            if !InitiateSystemShutdownExW(
                PCWSTR::null(),
                PCWSTR::null(),
                0,
                true,
                reboot,
                SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_MINOR_OTHER | SHTDN_REASON_FLAG_PLANNED,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to initiate a system shutdown: {}", err);
                return Err(PlatformError::Win32(err));
            }
        }

        Ok(())
    }

    fn do_suspend(&self, hibernate: bool) -> Result<(), PlatformError> {
        unsafe {
            if !SetSuspendState(
                BOOLEAN::from(hibernate),
                BOOLEAN::from(false),
                BOOLEAN::from(false),
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to initiate a system suspend: {}", err);
                return Err(PlatformError::Win32(err));
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl PowerManager for WindowsPowerManager {
    async fn get_supported_power_actions(
        &self,
    ) -> Result<Vec<PowerAction>, PlatformAbstractionError> {
        let mut actions = Vec::new();

        if self.has_shutdown_privilege {
            // We can always perform those option if we have the shutdown privilege
            actions.push(PowerAction::PowerOff);
            actions.push(PowerAction::Reboot);
            actions.push(PowerAction::Suspend);
            actions.push(PowerAction::Hibernate);
        }

        Ok(actions)
    }

    async fn perform_power_action(
        &self,
        action: PowerAction,
    ) -> Result<(), PlatformAbstractionError> {
        match action {
            PowerAction::PowerOff => self.do_shutdown(false)?,
            PowerAction::Reboot => self.do_shutdown(true)?,
            PowerAction::Suspend => self.do_suspend(false)?,
            PowerAction::Hibernate => self.do_suspend(true)?,
            _ => return Err(PlatformAbstractionError::Unsupported),
        };

        Ok(())
    }
}
