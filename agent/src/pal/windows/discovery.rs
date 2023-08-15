use std::borrow::Cow;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use windows::core::Error as Win32Error;

use tokio::sync::Mutex;
use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, WIN32_ERROR};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
    GAA_FLAG_SKIP_FRIENDLY_NAME, GAA_FLAG_SKIP_MULTICAST, GAA_FLAG_SKIP_UNICAST,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6, SOCKADDR_IN, SOCKADDR_IN6};

use crate::pal::discovery::DiscoveryManager;
use crate::pal::platform::dns::ServiceDnsRegistration;
use crate::pal::platform::name::ComputerName;
use crate::pal::platform::PlatformError;
use crate::pal::{PlatformAbstractionError, FALLBACK_NAME};
use crate::ssdp::{IpAddrWithScopeId, SSDPMulticast};

#[derive(Debug)]
pub struct WindowsDiscoveryManager {
    service_name: Cow<'static, str>,
    computer_name: Option<ComputerName>,
    dns_registration: Mutex<Option<ServiceDnsRegistration>>,
    ssdp: Mutex<Option<SSDPMulticast>>,
}

impl WindowsDiscoveryManager {
    pub fn new() -> Self {
        let computer_name = match ComputerName::determine() {
            Ok(name) => Some(name),
            Err(err) => {
                tracing::warn!("Failed to determine computer name: {}", err);
                None
            }
        };

        let service_name = match computer_name.as_ref().map(|n| n.dns_host_name_to_string()) {
            None => FALLBACK_NAME,
            Some(Ok(name)) => Cow::Owned(name),
            Some(Err(err)) => {
                tracing::warn!("Failed to convert computer name to DNS host name: {}", err);
                FALLBACK_NAME
            }
        };

        Self {
            service_name,
            computer_name,
            dns_registration: Mutex::new(None),
            ssdp: Mutex::new(None),
        }
    }

    async fn advertise_with_mdns(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        // We need the computer name in order to advertise the service using DNS
        let computer_name = match self.computer_name.clone() {
            None => return Err(PlatformAbstractionError::Unsupported),
            Some(v) => v,
        };

        let mut dns_registration = self.dns_registration.lock().await;
        if let Some(registration) = dns_registration.take() {
            // If the service is registered we need to deregister it first
            registration
                .perform_deregistration()
                .await
                .map_err(PlatformError::Win32)?;
        }

        // Attempt to register the service
        let registration =
            ServiceDnsRegistration::create(addr, computer_name, self.service_name.as_ref())
                .map_err(PlatformError::Win32)?;
        registration
            .perform_registration()
            .await
            .map_err(PlatformError::Win32)?;

        // Replace the old registration with the new one
        dns_registration.replace(registration);

        Ok(())
    }

    async fn stop_advertising_with_mdns(&self) -> Result<(), PlatformAbstractionError> {
        let mut dns_registration = self.dns_registration.lock().await;
        if let Some(registration) = dns_registration.take() {
            // If the service is registered we deregister it
            registration
                .perform_deregistration()
                .await
                .map_err(PlatformError::Win32)?;
        }

        Ok(())
    }

    /// Advertises the service using SSDP.
    async fn advertise_with_ssdp(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        self.stop_ssdp().await;

        let multicast_manager = SSDPMulticast::setup(
            self.service_name.to_string(),
            addr,
            Self::get_local_addresses,
        )
        .await?;
        self.ssdp.lock().await.replace(multicast_manager);

        Ok(())
    }

    // noinspection DuplicatedCode <- linux implementation
    /// Stops advertising the service using SSDP.
    async fn stop_ssdp(&self) {
        if let Some(ssdp) = self.ssdp.lock().await.take() {
            // Stop all the SSDP multicast sockets
            ssdp.stop().await;
        }
    }

    fn get_local_addresses() -> Result<Vec<IpAddrWithScopeId>, std::io::Error> {
        let mut ok = false;
        let mut buffer = Vec::new();
        for i in 1usize..16usize {
            // Make the buffer 16k * i bytes large
            buffer.resize(i * (1024 * 16), 0u8);

            let mut buffer_size = buffer.len() as u32;
            let err = unsafe {
                GetAdaptersAddresses(
                    0,
                    // Skip all the things we don't need
                    GAA_FLAG_SKIP_ANYCAST
                        | GAA_FLAG_SKIP_MULTICAST
                        | GAA_FLAG_SKIP_DNS_SERVER
                        | GAA_FLAG_SKIP_FRIENDLY_NAME,
                    None,
                    Some(buffer.as_mut_ptr() as _),
                    &mut buffer_size,
                )
            };

            let err = WIN32_ERROR(err);
            if err.is_err() && err != ERROR_BUFFER_OVERFLOW {
                // Translate the error as good as possible
                let err = Win32Error::from(err);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get local addresses: {}", err),
                ));
            }

            ok = err.is_ok();
            if ok {
                break;
            }
        }

        // Check if the buffer was successfully filled or we just broke out of the loop because
        // we exceeded the max size to try.
        if !ok {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get local addresses: No buffer size was large enough",
            ));
        }

        let mut addresses = Vec::new();

        let mut current_adapter = unsafe { &*(buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH) };
        loop {
            if !current_adapter.FirstUnicastAddress.is_null() {
                // Get all unicast addresses
                let mut current_address = unsafe { &*current_adapter.FirstUnicastAddress };
                loop {
                    let address = unsafe { &*current_address.Address.lpSockaddr };
                    let address = match address.sa_family {
                        AF_INET => {
                            let address = unsafe { &*(address as *const _ as *const SOCKADDR_IN) };
                            Some(IpAddrWithScopeId::V4(Ipv4Addr::from(address.sin_addr)))
                        }
                        AF_INET6 => {
                            let address = unsafe { &*(address as *const _ as *const SOCKADDR_IN6) };
                            Some(IpAddrWithScopeId::V6 {
                                addr: Ipv6Addr::from(address.sin6_addr),
                                scope_id: current_adapter.Ipv6IfIndex,
                            })
                        }
                        // Ignore unknown address families
                        _ => None,
                    };

                    // Add the address if it has been converted
                    if let Some(address) = address {
                        addresses.push(address);
                    }

                    if current_address.Next.is_null() {
                        break;
                    }

                    // Advance the iterator
                    current_address = unsafe { &*current_address.Next };
                }
            }

            if current_adapter.Next.is_null() {
                break;
            }

            // Advance the iterator
            current_adapter = unsafe { &*current_adapter.Next };
        }

        tracing::trace!("Found local addresses: {:?}", addresses);

        Ok(addresses)
    }
}

#[async_trait::async_trait]
impl DiscoveryManager for WindowsDiscoveryManager {
    async fn advertise_service(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        let (mdns_res, ssdp_res) = tokio::join!(
            self.advertise_with_mdns(addr),
            self.advertise_with_ssdp(addr)
        );

        if let Err(err) = &mdns_res {
            tracing::warn!("Failed to advertise with mDNS: {}", err);
        }

        if let Err(err) = &ssdp_res {
            tracing::warn!("Failed to advertise with SSDP: {}", err);
        }

        let success = mdns_res.is_ok() || ssdp_res.is_ok();
        if !success {
            Err(PlatformAbstractionError::Unsupported)
        } else {
            Ok(())
        }
    }

    async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError> {
        self.stop_ssdp().await;
        self.stop_advertising_with_mdns().await
    }
}
