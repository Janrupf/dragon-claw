use crate::pal::discovery::DiscoveryManager;
use crate::pal::platform::dbus::avahi::{AvahiEntryGroupProxy, AvahiServer2Proxy};
use crate::pal::platform::dbus::dbus_call;
use crate::pal::PlatformAbstractionError;
use std::borrow::Cow;
use std::net::SocketAddr;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct LinuxDiscoveryManager {
    avahi: AvahiServer2Proxy<'static>,
    registered_dns_service: Mutex<Option<AvahiEntryGroupProxy<'static>>>,
}

impl LinuxDiscoveryManager {
    /// Attempts to connect to Avahi and returns a new instance if successful.
    pub async fn try_connect(dbus_connection: &zbus::Connection) -> Option<Self> {
        let avahi = match dbus_call!(AvahiServer2Proxy::new(dbus_connection)).await {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(
                    "Failed to connect to Avahi, discovery will be unavailable: {}",
                    err
                );
                return None;
            }
        };

        if let Err(err) = dbus_call!(avahi.get_version_string()).await {
            tracing::warn!("Failed to get Avahi version: {}", err);
            return None;
        }

        Some(Self {
            avahi,
            registered_dns_service: Mutex::new(None),
        })
    }
}

#[async_trait::async_trait]
impl DiscoveryManager for LinuxDiscoveryManager {
    async fn advertise_service(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        let version = dbus_call!(self.avahi.get_version_string()).await?;
        tracing::info!("Avahi version: {}", version);

        let host_name = match dbus_call!(self.avahi.get_host_name()).await {
            Ok(v) => Cow::Owned(v),
            Err(err) => {
                tracing::warn!("Failed to get host name: {}", err);
                Cow::Borrowed("Dragon Claw Computer")
            }
        };
        tracing::info!("Host name: {}", host_name);

        let group = dbus_call!(self.avahi.entry_group_new()).await?;

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

    async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError> {
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
}
