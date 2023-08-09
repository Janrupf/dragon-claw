mod avahi;
mod login1;

use crate::pal::platform::avahi::{AvahiEntryGroupProxy, AvahiServer2Proxy};
use crate::pal::platform::login1::Login1ManagerProxy;
use crate::pal::{ApplicationStatus, PlatformAbstractionError, ShutdownRequestFut};
use futures_util::FutureExt;
use std::borrow::Cow;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::sync::Mutex;

const DBUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

// No init data required on Linux, PAL is initialized in the `new` function
pub type PlatformInitData = ();

macro_rules! dbus_call {
    ($e:expr) => {{
        ::tokio::time::timeout(DBUS_TIMEOUT, $e).map(|v| match v {
            Err(_) => Err(PlatformError::DbusTimeout),
            Ok(Err(err)) => Err(PlatformError::Dbus(err)),
            Ok(Ok(v)) => Ok(v),
        })
    }};
}

#[derive(Debug)]
pub struct PlatformAbstractionImpl {
    dbus_system_connection: zbus::Connection,
    avahi: Option<AvahiServer2Proxy<'static>>,
    login1manager: Option<Login1ManagerProxy<'static>>,
    registered_dns_service: Mutex<Option<AvahiEntryGroupProxy<'static>>>,
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
        let avahi = match dbus_call!(AvahiServer2Proxy::new(&dbus_system_connection)).await {
            Ok(v) => Some(v),
            Err(err) => {
                tracing::warn!(
                    "Failed to connect to Avahi, discovery will be unavailable: {}",
                    err
                );
                None
            }
        };

        // Connect to Login1 Manager
        let login1manager = match dbus_call!(Login1ManagerProxy::new(&dbus_system_connection)).await
        {
            Ok(v) => Some(v),
            Err(err) => {
                tracing::warn!(
                    "Failed to connect to Login1 Manager, shutdown will be unavailable: {}",
                    err
                );
                None
            }
        };

        Ok(Self {
            dbus_system_connection,
            avahi,
            login1manager,
            registered_dns_service: Mutex::new(None),
        })
    }

    pub async fn advertise_service(
        &self,
        addr: SocketAddr,
    ) -> Result<(), PlatformAbstractionError> {
        let avahi = match self.avahi.as_ref() {
            Some(v) => v,
            None => return Err(PlatformAbstractionError::Unsupported),
        };

        let version = dbus_call!(avahi.get_version_string()).await?;
        tracing::info!("Avahi version: {}", version);

        let host_name = match dbus_call!(avahi.get_host_name()).await {
            Ok(v) => Cow::Owned(v),
            Err(err) => {
                tracing::warn!("Failed to get host name: {}", err);
                Cow::Borrowed("Dragon Claw Computer")
            }
        };
        tracing::info!("Host name: {}", host_name);

        let group = dbus_call!(avahi.entry_group_new()).await?;

        dbus_call!(group.add_service(
            -1, // All interfaces
            match &addr {
                SocketAddr::V4(_) => 0,
                SocketAddr::V6(_) => 1,
            },
            0,
            &host_name,
            "_dragon-claw._tcp",
            None.into(),
            None.into(),
            addr.port(),
            &[],
        ))
        .await?;

        dbus_call!(group.commit()).await?;

        // Store the group in the registered service mutex
        let mut registered_dns_service = self.registered_dns_service.lock().await;
        registered_dns_service.replace(group);

        Ok(())
    }

    pub async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError> {
        // Take the group out of the mutex
        let mut registered_dns_service = self.registered_dns_service.lock().await;
        let group = match registered_dns_service.take() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Release the group
        dbus_call!(group.free()).await?;
        Ok(())
    }

    pub async fn shutdown_system(&self) -> Result<(), PlatformAbstractionError> {
        let login1manager = match self.login1manager.as_ref() {
            Some(v) => v,
            None => return Err(PlatformAbstractionError::Unsupported),
        };

        let can_power_off = dbus_call!(login1manager.can_power_off()).await?;
        if can_power_off != "yes" {
            // For now we fail, technically "challenge" could also be an acceptable answer
            // but this would require prompting the user for confirmation, which is hard to do
            // since this is a background service.
            tracing::error!("CanPowerOff returned {}", can_power_off);
            return Err(PlatformAbstractionError::Unsupported);
        }

        tracing::info!("Shutting down system");
        // Delay the future for 1 second to give the caller a chance to return a response
        // before we shut down the system.
        let login1manager = login1manager.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let _ = login1manager.power_off(false).await;
        });

        Ok(())
    }

    pub async fn set_status(&self, _: ApplicationStatus) {
        // Nothing to do on Linux
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),

    #[error("D-Bus request timed out")]
    DbusTimeout,
}
