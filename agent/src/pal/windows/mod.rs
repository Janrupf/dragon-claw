mod discovery;
mod dns;
mod power;
mod process;
mod service;
mod status;
mod util;

use crate::pal::platform::discovery::WindowsDiscoveryManager;
use crate::pal::platform::power::WindowsPowerManager;
use crate::pal::platform::process::OwnProcess;
use crate::pal::platform::service::dispatcher::ServiceDispatcher;
use crate::pal::platform::service::ServiceEnvironment;
use crate::pal::platform::status::WindowsStatusManager;
use crate::pal::{PlatformAbstractionError, PlatformAbstractionLayer, ShutdownRequestFut};
use std::sync::Arc;
use thiserror::Error;
use windows::core::Error as Win32Error;

#[derive(Debug)]
pub struct PlatformInitData {
    process: OwnProcess,
    service_environment: ServiceEnvironment,
    service_dispatcher: Option<Arc<ServiceDispatcher>>,
    has_shutdown_privilege: bool,
    has_system_environment_privilege: bool,
}

#[derive(Debug)]
pub struct PlatformAbstractionImpl {
    process: OwnProcess,
    service_environment: ServiceEnvironment,
    power_manager: WindowsPowerManager,
    discovery_manager: WindowsDiscoveryManager,
    status_manager: WindowsStatusManager,
}

impl PlatformAbstractionImpl {
    pub fn dispatch_main<F, R>(main: F) -> Result<R, PlatformAbstractionError>
    where
        F: FnOnce(PlatformInitData, ShutdownRequestFut) -> R,
    {
        // Initialize process data
        let mut init_data = Self::perform_pre_init()?;

        if init_data.service_environment != ServiceEnvironment::None {
            // We need to perform service specific initialization
            ServiceDispatcher::dispatch_service_main(move |dispatcher, shutdown_fut| {
                // We are now running as a real windows service
                let dispatcher = Arc::new(dispatcher);
                init_data.service_dispatcher = Some(dispatcher);

                main(init_data, shutdown_fut)
            })
            .map_err(PlatformError::Win32)
            .map_err(PlatformAbstractionError::Platform)
        } else {
            // Not a service, run the main without a wrapper
            Ok(main(init_data, crate::pal::ctrl_c_shutdown_fut()))
        }
    }

    fn perform_pre_init() -> Result<PlatformInitData, PlatformError> {
        let process = OwnProcess::open().map_err(PlatformError::Win32)?;

        // Make sure we can shutdown the system
        let has_shutdown_privilege =
            match process.enable_privileges(&[windows::Win32::Security::SE_SHUTDOWN_NAME]) {
                Ok(()) => true,
                Err(err) => {
                    tracing::warn!("Failed to enable shutdown privilege: {}", err);
                    false
                }
            };

        // Additionally we would like to access UEFI variables for reboot-to-firmware
        let has_system_environment_privilege = match process
            .enable_privileges(&[windows::Win32::Security::SE_SYSTEM_ENVIRONMENT_NAME])
        {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!("Failed to enable system environment privilege: {}", err);
                false
            }
        };

        let service_environment = match ServiceEnvironment::detect(&process) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("Failed to detect service environment: {}", err);
                tracing::warn!("Assuming to be running as a normal application");
                ServiceEnvironment::None
            }
        };

        tracing::trace!("Service environment: {:?}", service_environment);

        Ok(PlatformInitData {
            process,
            service_environment,
            service_dispatcher: None,
            has_shutdown_privilege,
            has_system_environment_privilege,
        })
    }

    pub async fn new(data: PlatformInitData) -> Result<Self, PlatformAbstractionError> {
        Ok(Self {
            process: data.process,
            service_environment: data.service_environment,
            discovery_manager: WindowsDiscoveryManager::new(),
            status_manager: WindowsStatusManager::new(data.service_dispatcher),
            power_manager: WindowsPowerManager::new(
                data.has_shutdown_privilege,
                data.has_system_environment_privilege,
            ),
        })
    }
}

#[async_trait::async_trait]
impl PlatformAbstractionLayer for PlatformAbstractionImpl {
    type PowerManager = WindowsPowerManager;

    fn power_manager(&self) -> Option<&Self::PowerManager> {
        Some(&self.power_manager)
    }

    type DiscoveryManager = WindowsDiscoveryManager;

    fn discovery_manager(&self) -> Option<&Self::DiscoveryManager> {
        Some(&self.discovery_manager)
    }

    type StatusManager = WindowsStatusManager;

    fn status_manager(&self) -> &Self::StatusManager {
        &self.status_manager
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error(transparent)]
    Win32(Win32Error),
}
