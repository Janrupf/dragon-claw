mod dbus;
mod discovery;
mod power;
mod status;

use crate::pal::platform::discovery::LinuxDiscoveryManager;
use crate::pal::platform::power::LinuxPowerManager;
use crate::pal::platform::status::LinuxStatusManager;
use crate::pal::{PlatformAbstractionError, PlatformAbstractionLayer, ShutdownRequestFut};
use thiserror::Error;

// No init data required on Linux, PAL is initialized in the `new` function
pub type PlatformInitData = ();

#[derive(Debug)]
pub struct PlatformAbstractionImpl {
    dbus_system_connection: zbus::Connection,
    service_manager: Option<LinuxDiscoveryManager>,
    power_manager: Option<LinuxPowerManager>,
}

impl PlatformAbstractionImpl {
    pub fn dispatch_main<F, R>(main: F) -> Result<R, PlatformAbstractionError>
    where
        F: FnOnce(PlatformInitData, ShutdownRequestFut) -> R,
    {
        // Simply run the main function, we don't need to perform any platform initialization
        Ok(main((), crate::pal::ctrl_c_shutdown_fut()))
    }

    pub async fn new(_: PlatformInitData) -> Result<Self, PlatformAbstractionError> {
        // Connect to system D-Bus
        let dbus_system_connection = zbus::Connection::system()
            .await
            .map_err(PlatformError::from)?;

        tracing::debug!(
            "Connected to system D-Bus \"{}\"",
            dbus_system_connection
                .unique_name()
                .map(|n| n.as_str())
                .unwrap_or("<unknown>")
        );

        // Attempt to connect to Avahi
        let service_manager = LinuxDiscoveryManager::try_connect(&dbus_system_connection).await;

        // Connect to Login1 Manager
        let power_manager = LinuxPowerManager::try_connect(&dbus_system_connection).await;

        Ok(Self {
            dbus_system_connection,
            service_manager,
            power_manager,
        })
    }
}

impl PlatformAbstractionLayer for PlatformAbstractionImpl {
    type PowerManager = LinuxPowerManager;

    fn power_manager(&self) -> Option<&Self::PowerManager> {
        self.power_manager.as_ref()
    }

    type DiscoveryManager = LinuxDiscoveryManager;

    fn discovery_manager(&self) -> Option<&Self::DiscoveryManager> {
        self.service_manager.as_ref()
    }

    type StatusManager = LinuxStatusManager;

    fn status_manager(&self) -> &Self::StatusManager {
        &LinuxStatusManager
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),

    #[error("D-Bus request timed out")]
    DbusTimeout,
}
