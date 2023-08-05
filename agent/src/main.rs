use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::net::discovery::DiscoveryServer;

mod net;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Set up logging using tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("Starting agent...");

    let discovery_server = DiscoveryServer::start().await.ok();
    if discovery_server.is_none() {
        tracing::warn!("failed to start discovery server, discovery will be unavailable");
    }
}
