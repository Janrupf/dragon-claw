mod avahi;
mod login1;

use crate::pal::platform::avahi::AvahiServer2Proxy;
use crate::pal::platform::login1::Login1ManagerProxy;
use crate::pal::PlatformAbstractionError;
use futures_util::FutureExt;
use std::borrow::Cow;
use std::net::SocketAddr;
use thiserror::Error;

const DBUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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
}

impl PlatformAbstractionImpl {
    pub async fn new() -> Result<Self, PlatformAbstractionError> {
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
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),

    #[error("D-Bus request timed out")]
    DbusTimeout,
}
