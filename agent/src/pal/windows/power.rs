use crate::pal::platform::util::ToWin32ErrorCode;
use crate::pal::platform::PlatformError;
use crate::pal::power::{PowerAction, PowerManager};
use crate::pal::PlatformAbstractionError;
use windows::core::{Error as Win32Error, PCWSTR};
use windows::w as wide_string;
use windows::Win32::Foundation::{BOOLEAN, ERROR_ENVVAR_NOT_FOUND};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Shutdown::{
    InitiateSystemShutdownExW, SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
    SHTDN_REASON_MINOR_OTHER,
};
use windows::Win32::System::SystemInformation::{
    FirmwareTypeBios, FirmwareTypeUefi, GetFirmwareType, FIRMWARE_TYPE,
};
use windows::Win32::System::WindowsProgramming::{
    GetFirmwareEnvironmentVariableExW, SetFirmwareEnvironmentVariableExW,
};

const EFI_GLOBAL_VARIABLE: PCWSTR = wide_string!("{8BE4DF61-93CA-11D2-AA0D-00E098032B8C}");
const EFI_OS_INDICATION_SUPPORTED: PCWSTR = wide_string!("OsIndicationsSupported");
const EFI_OS_INDICATIONS: PCWSTR = wide_string!("OsIndications");
const EFI_OS_INDICATIONS_BOOT_TO_FW_UI: u64 = 0x0000000000000001;
const EFI_VARIABLE_ATTRIBUTE_NON_VOLATILE: u32 = 0x1;

const EFI_VARIABLE_ATTRIBUTE_BOOTSERVICE_ACCESS: u32 = 0x2;
const EFI_VARIABLE_ATTRIBUTE_RUNTIME_ACCESS: u32 = 0x4;

#[derive(Debug)]
pub struct WindowsPowerManager {
    has_shutdown_privilege: bool,
    has_system_environment_privilege: bool,
}

impl WindowsPowerManager {
    pub fn new(has_shutdown_privilege: bool, has_system_environment_privilege: bool) -> Self {
        Self {
            has_shutdown_privilege,
            has_system_environment_privilege,
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

    //noinspection DuplicatedCode
    fn can_reboot_to_firmware(&self) -> bool {
        if !self.has_system_environment_privilege {
            // We can't access UEFI variables, so we can't reboot to the firmware UI
            return false;
        }

        unsafe {
            let mut firmware_type = FIRMWARE_TYPE::default();
            if !GetFirmwareType(&mut firmware_type).as_bool() {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to get firmware type: {}", err);
                return false;
            }

            #[allow(non_upper_case_globals)]
            match firmware_type {
                FirmwareTypeBios => {
                    tracing::debug!("Running on BIOS firmware");
                    return false;
                }
                FirmwareTypeUefi => {
                    tracing::debug!("Running on UEFI firmware");
                }
                _ => {
                    tracing::debug!("Running on unknown firmware");
                    return false;
                }
            };

            // Read the OsIndicationsSupported variable to see if we can reboot to the firmware UI
            let mut os_indications_support: u64 = 0;
            let mut attributes = EFI_VARIABLE_ATTRIBUTE_NON_VOLATILE
                | EFI_VARIABLE_ATTRIBUTE_BOOTSERVICE_ACCESS
                | EFI_VARIABLE_ATTRIBUTE_RUNTIME_ACCESS;

            if GetFirmwareEnvironmentVariableExW(
                EFI_OS_INDICATION_SUPPORTED,
                EFI_GLOBAL_VARIABLE,
                Some(&mut os_indications_support as *mut _ as *mut _),
                std::mem::size_of_val(&os_indications_support) as _,
                Some(&mut attributes as *mut _ as *mut _),
            ) == 0
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to read OsIndicationsSupported: {}", err);
                return false;
            }

            // Check if the bitmask is set
            let is_reboot_to_firmware_supported = os_indications_support
                & EFI_OS_INDICATIONS_BOOT_TO_FW_UI
                == EFI_OS_INDICATIONS_BOOT_TO_FW_UI;

            tracing::debug!(
                "Reboot to firmware UI is {}",
                if is_reboot_to_firmware_supported {
                    "supported"
                } else {
                    "not supported"
                }
            );
            is_reboot_to_firmware_supported
        }
    }

    //noinspection DuplicatedCode
    fn set_reboot_to_firmware(&self) -> Result<(), Win32Error> {
        unsafe {
            let mut os_indications: u64 = 0;
            let mut attributes = EFI_VARIABLE_ATTRIBUTE_NON_VOLATILE
                | EFI_VARIABLE_ATTRIBUTE_BOOTSERVICE_ACCESS
                | EFI_VARIABLE_ATTRIBUTE_RUNTIME_ACCESS;

            if GetFirmwareEnvironmentVariableExW(
                EFI_OS_INDICATIONS,
                EFI_GLOBAL_VARIABLE,
                Some(&mut os_indications as *mut _ as *mut _),
                std::mem::size_of_val(&os_indications) as _,
                Some(&mut attributes as *mut _ as *mut _),
            ) == 0
            {
                let err = Win32Error::from_win32();

                if err.to_win32_error_code() != ERROR_ENVVAR_NOT_FOUND {
                    tracing::error!("Failed to read OsIndications: {}", err);
                    return Err(err);
                }

                // If the variable doesn't exist, treat it as if it was 0
                os_indications = 0;
                attributes = EFI_VARIABLE_ATTRIBUTE_NON_VOLATILE
                    | EFI_VARIABLE_ATTRIBUTE_BOOTSERVICE_ACCESS
                    | EFI_VARIABLE_ATTRIBUTE_RUNTIME_ACCESS;
            }

            // Set the bit
            os_indications |= EFI_OS_INDICATIONS_BOOT_TO_FW_UI;

            if !SetFirmwareEnvironmentVariableExW(
                EFI_OS_INDICATIONS,
                EFI_GLOBAL_VARIABLE,
                Some(&os_indications as *const _ as *const _),
                std::mem::size_of_val(&os_indications) as _,
                attributes,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to set OsIndications: {}", err);
                return Err(err);
            };

            Ok(())
        }
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

            if self.can_reboot_to_firmware() {
                actions.push(PowerAction::RebootToFirmware);
            }
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
            PowerAction::RebootToFirmware => {
                self.set_reboot_to_firmware()
                    .map_err(PlatformError::Win32)?;
                self.do_shutdown(true)?
            }
            _ => return Err(PlatformAbstractionError::Unsupported),
        };

        Ok(())
    }
}
