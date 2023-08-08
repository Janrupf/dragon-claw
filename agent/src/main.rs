use crate::error::DragonClawAgentError;
use crate::pal::{ApplicationStatus, PlatformInitData, ShutdownRequestFut};
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

fn main() {
    let file_appender = tracing_appender::rolling::daily("C:/Temp", "dragon-claw-agent.log");

    // Set up logging using tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_appender),
        )
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

    pal.set_status(ApplicationStatus::Starting).await;
    match runner(pal.clone(), shutdown_fut).await {
        Ok(()) => {
            tracing::info!("Service finished successfully!");
            pal.set_status(ApplicationStatus::Stopped).await;

            Ok(())
        }
        Err(err) => {
            tracing::error!("Service failed: {}", err);
            pal.set_status(ApplicationStatus::ApplicationError(err.into()))
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
    let socket_addr = SocketAddrV4::new(any_host, 0);

    let listener = TcpListener::bind(socket_addr).await?;
    let local_addr = listener.local_addr()?;

    tracing::debug!("Listening on {}", local_addr);

    if let Err(err) = pal.advertise_service(local_addr).await {
        tracing::warn!(
            "Failed to advertise service, discovery not available: {}",
            err
        );
    }

    let incoming =
        TcpIncoming::from_listener(listener, true, None).map_err(DragonClawAgentError::Tonic)?;

    tracing::info!("Starting RPC...");
    let server_future = Server::builder()
        .add_service(DragonClawAgentServer::new(DragonClawAgentImpl::new(
            pal.clone(),
        )))
        .serve_with_incoming(incoming);

    pal.set_status(ApplicationStatus::Running).await;

    tokio::select! {
        res = server_future => res?,
        _ = shutdown_fut => {
            tracing::info!("Received shutdown request, shutting down...");
        }
    }

    pal.set_status(ApplicationStatus::Stopping).await;
    
    if let Err(err) = pal.stop_advertising_service().await {
        tracing::warn!("Failed to stop advertising service: {}", err);
    }

    Ok(())
}
