use crate::pal::PlatformAbstractionError;
use std::net::SocketAddr;

#[async_trait::async_trait]
pub trait DiscoveryManager: Send + Sync + 'static {
    /// Starts advertising the service.
    async fn advertise_service(
        &self,
        socket_addr: SocketAddr,
    ) -> Result<(), PlatformAbstractionError>;

    /// Stops advertising the service.
    async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError>;
}
