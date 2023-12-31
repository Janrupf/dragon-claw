use crate::error::DragonClawAgentError;
use crate::pal::discovery::DiscoveryManager;
use crate::pal::status::{ApplicationStatus, StatusManager};
use crate::pal::{PlatformInitData, ShutdownRequestFut};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::server::TcpIncoming;
use tonic::transport::Server;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::proto::{DragonClawAgentImpl, DragonClawAgentServer};

mod error;
mod pal;
mod proto;
mod ssdp;

fn main() {
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

    let res = pal::PlatformAbstraction::dispatch_main(service_main);

    // Set exit code depending on run result
    match res {
        Ok(Ok(())) => {
            tracing::trace!("Exiting with code 0");
            std::process::exit(0)
        }
        Ok(Err(())) => {
            tracing::trace!("Exiting with code 2");
            std::process::exit(2)
        }
        Err(err) => {
            tracing::error!("Failed to start main: {}", err);
            std::process::exit(1)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn service_main(data: PlatformInitData, shutdown_fut: ShutdownRequestFut) -> Result<(), ()> {
    tracing::debug!("Creating platform abstraction layer...");
    let pal = match pal::PlatformAbstraction::new(data).await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to create platform abstraction layer: {}", err);
            return Err(());
        }
    };

    let pal = Arc::new(pal);

    pal.status_manager()
        .set_status(ApplicationStatus::Starting)
        .await;
    match runner(pal.clone(), shutdown_fut).await {
        Ok(()) => {
            tracing::info!("Service finished successfully!");
            pal.status_manager()
                .set_status(ApplicationStatus::Stopped)
                .await;

            Ok(())
        }
        Err(err) => {
            tracing::error!("Service failed: {}", err);
            pal.status_manager()
                .set_status(ApplicationStatus::ApplicationError(err.into()))
                .await;

            Err(())
        }
    }
}

async fn runner(
    pal: Arc<pal::PlatformAbstraction>,
    shutdown_fut: ShutdownRequestFut,
) -> Result<(), DragonClawAgentError> {
    tracing::debug!("Binding TCP listener...");
    let any_host = Ipv4Addr::new(0, 0, 0, 0);
    let socket_addr = SocketAddrV4::new(any_host, 37121);

    let listener = TcpListener::bind(socket_addr).await?;
    let local_addr = listener.local_addr()?;

    tracing::debug!("Listening on {}", local_addr);
    let discovery_manager = pal.discovery_manager();

    let service_advertised = if let Err(err) = discovery_manager.advertise_service(local_addr).await
    {
        tracing::warn!(
            "Failed to advertise service, discovery not available: {}",
            err
        );

        false
    } else {
        true
    };

    let incoming =
        TcpIncoming::from_listener(listener, true, None).map_err(DragonClawAgentError::Tonic)?;

    tracing::info!("Starting RPC...");
    let server_future = Server::builder()
        .add_service(DragonClawAgentServer::new(DragonClawAgentImpl::new(
            pal.clone(),
        )))
        .serve_with_incoming(incoming);

    pal.status_manager()
        .set_status(ApplicationStatus::Running)
        .await;

    tokio::select! {
        res = server_future => res?,
        _ = shutdown_fut => {
            tracing::info!("Received shutdown request, shutting down...");
        }
    }

    pal.status_manager()
        .set_status(ApplicationStatus::Stopping)
        .await;

    if service_advertised {
        if let Err(err) = discovery_manager.stop_advertising_service().await {
            tracing::warn!("Failed to stop advertising service: {}", err);
        }
    }

    Ok(())
}
