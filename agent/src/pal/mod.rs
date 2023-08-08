use std::error::Error;
use std::future;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use thiserror::Error;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod platform;

/// Opaque type for platform-specific initialization data.
pub type PlatformInitData = platform::PlatformInitData;

/// Future type for shutdown requests.
pub type ShutdownRequestFut = Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

#[derive(Debug)]
pub enum ApplicationStatus {
    /// The application is starting up
    Starting,

    /// The application is running
    Running,

    /// The application is shutting down
    Stopping,

    /// The application has stopped
    Stopped,

    /// The application has failed with platform error
    PlatformError(PlatformAbstractionError),

    /// The application has failed with application error
    ApplicationError(Box<dyn Error>),
}

#[derive(Debug)]
pub struct PlatformAbstraction {
    platform: platform::PlatformAbstractionImpl,
}

impl PlatformAbstraction {
    pub fn dispatch_main<F, R>(main: F) -> Result<R, PlatformAbstractionError>
    where
        F: FnOnce(PlatformInitData, ShutdownRequestFut) -> R,
    {
        platform::PlatformAbstractionImpl::dispatch_main(main)
    }

    /// Creates a new platform abstraction layer.
    pub async fn new(data: PlatformInitData) -> Result<Self, PlatformAbstractionError> {
        let platform = platform::PlatformAbstractionImpl::new(data).await?;
        Ok(Self { platform })
    }

    /// Starts advertising the service.
    pub async fn advertise_service(
        &self,
        socket_addr: SocketAddr,
    ) -> Result<(), PlatformAbstractionError> {
        self.platform.advertise_service(socket_addr).await
    }

    /// Stops advertising the service.
    pub async fn stop_advertising_service(
        &self
    ) -> Result<(), PlatformAbstractionError> {
        self.platform.stop_advertising_service().await
    }
    
    /// Shuts down the system.
    pub async fn shutdown_system(&self) -> Result<(), PlatformAbstractionError> {
        self.platform.shutdown_system().await
    }

    /// Sets the application status.
    pub async fn set_status(&self, status: ApplicationStatus) {
        self.platform.set_status(status).await;
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

pub(in crate::pal) fn ctrl_c_shutdown_fut() -> ShutdownRequestFut {
    Box::pin(async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::warn!("Failed to listen for Ctrl-C: {}", err);
            future::pending::<()>().await; // Just hang then, the process probably will need to be killed
        }
    })
}
