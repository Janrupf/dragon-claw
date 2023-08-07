#![allow(clippy::too_many_arguments)]

use crate::net::discovery::{DiscoveryData, DiscoveryError};
use std::borrow::Cow;
use std::fmt::Debug;
use std::time::Duration;
use zbus::zvariant::Optional;

const AVAHI_TIMEOUT: Duration = Duration::from_millis(100);

macro_rules! avahi_call {
    ($e:expr) => {{
        match ::tokio::time::timeout(AVAHI_TIMEOUT, $e).await {
            Err(_) => {
                ::tracing::error!("Avahi call timed out");
                return Err(DiscoveryError::Unavailable);
            }
            Ok(v) => v,
        }
    }};
}

#[derive(Debug)]
pub struct DiscoveryServer {
    proxy: AvahiServer2Proxy<'static>,
}

impl DiscoveryServer {
    /// Starts a new discovery server.
    #[tracing::instrument]
    pub async fn start(data: DiscoveryData) -> Result<Self, DiscoveryError> {
        let connection = match zbus::Connection::system().await {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("DBus not available: {}", err);
                return Err(DiscoveryError::Unavailable);
            }
        };

        tracing::debug!("Connected to DBus!");

        // Create the proxy
        let proxy = match AvahiServer2Proxy::new(&connection).await {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("Avahi not available: {}", err);
                return Err(DiscoveryError::Unavailable);
            }
        };

        match avahi_call!(proxy.get_version_string()) {
            Err(err) => {
                tracing::error!("Failed to get avahi version: {}", err);
                return Err(DiscoveryError::Unavailable);
            }
            Ok(version) => {
                tracing::info!("Avahi version: {}", version);
            }
        };

        let host_name = match avahi_call!(proxy.get_host_name()) {
            Ok(v) => Cow::Owned(v),
            Err(err) => {
                tracing::warn!("Failed to get host name: {}", err);
                Cow::Borrowed("Dragon Claw Computer")
            }
        };

        let group = match avahi_call!(proxy.entry_group_new()) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("Failed to create entry group: {}", err);
                return Err(DiscoveryError::Unavailable);
            }
        };

        if let Err(err) = avahi_call!(group.add_service(
            -1, // All interfaces
            0,  // IPv4
            0,
            &host_name,
            "_dragon-claw._tcp",
            None.into(),
            None.into(),
            data.addr.port(),
            &[],
        )) {
            tracing::error!("Failed to add service: {}", err);
            return Err(DiscoveryError::Unavailable);
        }

        if let Err(err) = avahi_call!(group.commit()) {
            tracing::error!("Failed to commit entry group: {}", err);
            return Err(DiscoveryError::Unavailable);
        }

        Ok(Self { proxy })
    }
}

#[zbus::dbus_proxy(
    interface = "org.freedesktop.Avahi.Server2",
    default_service = "org.freedesktop.Avahi",
    default_path = "/"
)]
trait AvahiServer2 {
    async fn get_version_string(&self) -> zbus::Result<String>;

    async fn get_host_name(&self) -> zbus::Result<String>;

    #[dbus_proxy(object = "AvahiEntryGroup")]
    async fn entry_group_new(&self);
}

#[zbus::dbus_proxy(
    interface = "org.freedesktop.Avahi.EntryGroup",
    default_service = "org.freedesktop.Avahi",
    assume_defaults = false
)]
trait AvahiEntryGroup {
    async fn add_service(
        &self,
        interface: i32,
        protocol: i32,
        flags: u32,
        name: &str,
        ty: &str,
        domain: Optional<&str>,
        host: Optional<&str>,
        port: u16,
        txt: &[&[u8]],
    ) -> zbus::Result<()>;

    async fn commit(&self) -> zbus::Result<()>;
}
