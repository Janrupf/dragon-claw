use crate::pal::discovery::DiscoveryManager;
use crate::pal::power::PowerManager;
use crate::pal::status::StatusManager;
use std::future;
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod platform;

pub mod discovery;
pub mod power;
pub mod status;

/// Opaque type for platform-specific initialization data.
pub type PlatformInitData = platform::PlatformInitData;

/// Future type for shutdown requests.
pub type ShutdownRequestFut = Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

#[derive(Debug)]
pub struct PlatformAbstraction {
    platform: platform::PlatformAbstractionImpl,
}

pub type PlatformPowerManager =
    <platform::PlatformAbstractionImpl as PlatformAbstractionLayer>::PowerManager;
pub type PlatformDiscoveryManager =
    <platform::PlatformAbstractionImpl as PlatformAbstractionLayer>::DiscoveryManager;
pub type PlatformStatusManager =
    <platform::PlatformAbstractionImpl as PlatformAbstractionLayer>::StatusManager;

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

    /// Retrieves the power manager.
    pub fn power_manager(&self) -> Option<&PlatformPowerManager> {
        self.platform.power_manager()
    }

    /// Retrieves the discovery manager.
    pub fn discovery_manager(&self) -> &PlatformDiscoveryManager {
        self.platform.discovery_manager()
    }

    /// Retrieves the status manager.
    pub fn status_manager(&self) -> &PlatformStatusManager {
        self.platform.status_manager()
    }
}

pub trait PlatformAbstractionLayer: Send + Sync + 'static {
    /// The type of the power manager.
    type PowerManager: PowerManager;

    /// Retrieves the power manager.
    fn power_manager(&self) -> Option<&Self::PowerManager>;

    /// The type of the discovery manager.
    type DiscoveryManager: DiscoveryManager;

    /// Retrieves the discovery manager.
    fn discovery_manager(&self) -> &Self::DiscoveryManager;

    /// The type of the status manager.
    type StatusManager: StatusManager;

    /// Retrieves the status manager.
    fn status_manager(&self) -> &Self::StatusManager;
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
