use crate::pal::discovery::DiscoveryManager;
use crate::pal::platform::dbus::avahi::{AvahiEntryGroupProxy, AvahiServer2Proxy};
use crate::pal::platform::dbus::dbus_call;
use crate::pal::{PlatformAbstractionError, FALLBACK_NAME};
use crate::ssdp::{IpAddrWithScopeId, SSDPMulticast};
use std::borrow::Cow;
use std::ffi::CString;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct LinuxDiscoveryManager {
    avahi: Option<AvahiServer2Proxy<'static>>,
    host_name: Cow<'static, str>,
    registered_dns_service: Mutex<Option<AvahiEntryGroupProxy<'static>>>,
    ssdp: Mutex<Option<SSDPMulticast>>,
}

impl LinuxDiscoveryManager {
    /// Attempts to connect to Avahi.
    pub async fn new(dbus_connection: &zbus::Connection) -> Self {
        let (avahi, host_name) = match dbus_call!(AvahiServer2Proxy::new(dbus_connection)).await {
            Ok(avahi) => {
                let host_name = match dbus_call!(avahi.get_host_name()).await {
                    Ok(v) => {
                        tracing::info!("Host name: {}", v);
                        Some(Cow::Owned(v))
                    }
                    Err(err) => {
                        tracing::warn!("Failed to get host name: {}", err);
                        None
                    }
                };
                (Some(avahi), host_name)
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to connect to Avahi, mDNS discovery will be unavailable: {}",
                    err
                );
                (None, None)
            }
        };

        // If avahi did not give us a host name, attempt to get it from libc
        let host_name = host_name.unwrap_or_else(|| {
            // Attempt to get hostname from libc
            let mut buf = vec![0u8; 256];
            if unsafe { libc::gethostname(buf.as_mut_ptr() as _, 255) } != 0 {
                tracing::warn!(
                    "Failed to call gethostname: {}",
                    std::io::Error::last_os_error()
                );
                return FALLBACK_NAME;
            }

            // Make sure to always have null-terminated string
            if buf[255] != 0 {
                buf[255] = 0;
            }

            match CString::from_vec_with_nul(buf).map(|v| v.into_string()) {
                Ok(Ok(v)) => Cow::Owned(v),
                Ok(Err(err)) => {
                    tracing::warn!("Failed to convert hostname to string: {}", err);
                    FALLBACK_NAME
                }
                Err(err) => {
                    tracing::warn!("Failed to convert hostname to string: {}", err);
                    FALLBACK_NAME
                }
            }
        });

        Self {
            avahi,
            host_name,
            registered_dns_service: Mutex::new(None),
            ssdp: Mutex::new(None),
        }
    }

    async fn advertise_with_avahi(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        let avahi = self
            .avahi
            .as_ref()
            .ok_or(PlatformAbstractionError::Unsupported)?;

        let version = dbus_call!(avahi.get_version_string()).await?;
        tracing::info!("Avahi version: {}", version);

        let group = dbus_call!(avahi.entry_group_new()).await?;

        dbus_call!(group.add_service(
            -1, // All interfaces
            match &addr {
                SocketAddr::V4(_) => 0,
                SocketAddr::V6(_) => 1,
            },
            0,
            &self.host_name,
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

    /// Advertises the service using SSDP.
    async fn advertise_with_ssdp(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        self.stop_ssdp().await;

        let multicast_manager =
            SSDPMulticast::setup(self.host_name.to_string(), addr, Self::get_local_addresses)
                .await?;
        self.ssdp.lock().await.replace(multicast_manager);

        Ok(())
    }

    async fn stop_ssdp(&self) {
        if let Some(ssdp) = self.ssdp.lock().await.take() {
            // Stop all the SSDP multicast sockets
            ssdp.stop().await;
        }
    }

    fn get_local_addresses() -> Result<Vec<IpAddrWithScopeId>, std::io::Error> {
        let mut addresses = std::ptr::null_mut();
        if unsafe { libc::getifaddrs(&mut addresses) } == -1 {
            // Failed to get the addresses
            return Err(std::io::Error::last_os_error());
        }

        let mut out = Vec::new();

        let mut next = addresses;
        while !next.is_null() {
            // Get the current address
            let current = unsafe { &*next };
            if current.ifa_addr.is_null() {
                next = current.ifa_next;
                continue;
            }

            let current_address = unsafe { &*current.ifa_addr };

            // Get the address family
            let family = current_address.sa_family as libc::c_int;

            match family {
                libc::AF_INET => {
                    let address = unsafe { &*(current.ifa_addr as *const libc::sockaddr_in) };
                    out.push(IpAddrWithScopeId::V4(Ipv4Addr::from(
                        address.sin_addr.s_addr.to_le_bytes(),
                    )));
                }

                libc::AF_INET6 => {
                    let address = unsafe { &*(current.ifa_addr as *const libc::sockaddr_in6) };
                    out.push(IpAddrWithScopeId::V6 {
                        addr: Ipv6Addr::from(address.sin6_addr.s6_addr),
                        scope_id: address.sin6_scope_id,
                    });
                }

                _ => {
                    // Unknown address family
                }
            };

            next = current.ifa_next;
        }

        unsafe { libc::freeifaddrs(addresses) };

        tracing::trace!("Local addresses: {:?}", out);

        Ok(out)
    }
}

#[async_trait::async_trait]
impl DiscoveryManager for LinuxDiscoveryManager {
    async fn advertise_service(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        let (avahi_res, ssdp_res) = tokio::join!(
            self.advertise_with_avahi(addr),
            self.advertise_with_ssdp(addr)
        );

        if let Err(err) = &avahi_res {
            tracing::warn!("Failed to advertise with Avahi: {}", err);
        }

        if let Err(err) = &ssdp_res {
            tracing::warn!("Failed to advertise with SSDP: {}", err);
        }

        let success = avahi_res.is_ok() || ssdp_res.is_ok();
        if !success {
            Err(PlatformAbstractionError::Unsupported)
        } else {
            Ok(())
        }
    }

    async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError> {
        let mut error = None;

        // Take the group out of the mutex
        let mut registered_dns_service = self.registered_dns_service.lock().await;
        if let Some(group) = registered_dns_service.take() {
            // Release the group
            if let Err(err) = dbus_call!(group.free()).await {
                tracing::warn!("Failed to free Avahi group: {}", err);
                error = Some(err);
            }
        }

        // Stop the SSDP multicast sockets
        self.stop_ssdp().await;

        match error {
            None => Ok(()),
            Some(err) => Err(err.into()),
        }
    }
}
