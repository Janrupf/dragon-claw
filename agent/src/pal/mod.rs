use std::net::SocketAddr;
use thiserror::Error;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod platform;

#[derive(Debug)]
pub struct PlatformAbstraction {
    platform: platform::PlatformAbstractionImpl,
}

impl PlatformAbstraction {
    /// Creates a new platform abstraction layer.
    pub async fn new() -> Result<Self, PlatformAbstractionError> {
        let platform = platform::PlatformAbstractionImpl::new().await?;
        Ok(Self { platform })
    }

    /// Starts advertising the service.
    pub async fn advertise_service(
        &self,
        socket_addr: SocketAddr,
    ) -> Result<(), PlatformAbstractionError> {
        self.platform.advertise_service(socket_addr).await
    }

    /// Shuts down the system.
    pub async fn shutdown_system(&self) -> Result<(), PlatformAbstractionError> {
        self.platform.shutdown_system().await
    }
}

#[derive(Debug, Error)]
pub enum PlatformAbstractionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Platform(#[from] platform::PlatformError),

    #[error("operation not supported")]
    Unsupported,
}
