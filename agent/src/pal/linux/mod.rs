mod avahi;

use crate::pal::platform::avahi::AvahiServer2Proxy;
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

        Ok(Self {
            dbus_system_connection,
            avahi,
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
        Err(PlatformAbstractionError::Unsupported)
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),

    #[error("D-Bus request timed out")]
    DbusTimeout,
}
