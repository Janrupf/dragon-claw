use crate::pal::discovery::DiscoveryManager;
use crate::pal::platform::dns::ServiceDnsRegistration;
use crate::pal::platform::PlatformError;
use crate::pal::PlatformAbstractionError;
use std::net::SocketAddr;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct WindowsDiscoveryManager {
    dns_registration: Mutex<Option<ServiceDnsRegistration>>,
}

impl WindowsDiscoveryManager {
    pub fn new() -> Self {
        Self {
            dns_registration: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryManager for WindowsDiscoveryManager {
    async fn advertise_service(&self, addr: SocketAddr) -> Result<(), PlatformAbstractionError> {
        let mut dns_registration = self.dns_registration.lock().await;
        if let Some(registration) = dns_registration.take() {
            // If the service is registered we need to deregister it first
            registration
                .perform_deregistration()
                .await
                .map_err(PlatformError::Win32)?;
        }

        // Attempt to register the service
        let registration = ServiceDnsRegistration::create(addr).map_err(PlatformError::Win32)?;
        registration
            .perform_registration()
            .await
            .map_err(PlatformError::Win32)?;

        // Replace the old registration with the new one
        dns_registration.replace(registration);

        Ok(())
    }

    async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError> {
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
}
