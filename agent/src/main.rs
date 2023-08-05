use tonic::transport::Server;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::net::discovery::DiscoveryServer;
use crate::proto::{DragonClawAgentImpl, DragonClawAgentServer};

mod net;
mod proto;

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

    tracing::info!("Starting RPC...");
    let server_future = Server::builder()
        .add_service(DragonClawAgentServer::new(DragonClawAgentImpl::new()))
        .serve("0.0.0.0:4455".parse().unwrap());

    let ctrl_c_future = tokio::signal::ctrl_c();

    tokio::select! {
        res = server_future => {
            tracing::error!("RPC server stopped unexpectedly: {:?}", res);
        }
        _ = ctrl_c_future => {
            tracing::info!("Received Ctrl+C, shutting down...");
        }
    }
}
