use std::net::SocketAddr;
use thiserror::Error;

#[path = "linux/mod.rs"]
mod platform;

#[derive(Debug)]
pub struct DiscoveryData {
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub struct DiscoveryServer {
    platform: platform::DiscoveryServer,
}

impl DiscoveryServer {
    /// Starts a new discovery server.
    pub async fn start(data: DiscoveryData) -> Result<Self, DiscoveryError> {
        let platform = platform::DiscoveryServer::start(data).await?;

        Ok(Self { platform })
    }
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("the current system configuration does not support discovery")]
    Unavailable,
}
