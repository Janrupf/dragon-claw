use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::TcpListener;
use tonic::transport::server::TcpIncoming;
use tonic::transport::Server;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::proto::{DragonClawAgentImpl, DragonClawAgentServer};

mod pal;
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

    tracing::debug!("Creating platform abstraction layer...");
    let pal = match pal::PlatformAbstraction::new().await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to create platform abstraction layer: {}", err);
            return;
        }
    };

    tracing::debug!("Binding TCP listener...");
    let any_host = Ipv4Addr::new(0, 0, 0, 0);
    let socket_addr = SocketAddrV4::new(any_host, 0);

    let listener = match TcpListener::bind(socket_addr).await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to bind TCP listener: {}", err);
            return;
        }
    };

    let local_addr = match listener.local_addr() {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to get local address: {}", err);
            return;
        }
    };

    tracing::debug!("Listening on {}", local_addr);

    if let Err(err) = pal.advertise_service(local_addr).await {
        tracing::warn!(
            "Failed to advertise service, discovery not available: {}",
            err
        );
    }

    let incoming = match TcpIncoming::from_listener(listener, true, None) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to configure incoming listener: {}", err);
            return;
        }
    };

    tracing::info!("Starting RPC...");
    let server_future = Server::builder()
        .add_service(DragonClawAgentServer::new(DragonClawAgentImpl::new(pal)))
        .serve_with_incoming(incoming);

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
