tonic::include_proto!("net.janrupf.dc");

use crate::pal::PlatformAbstraction;
use dragon_claw_agent_server::*;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct DragonClawAgentImpl {
    pal: PlatformAbstraction,
}

impl DragonClawAgentImpl {
    pub fn new(pal: PlatformAbstraction) -> Self {
        Self { pal }
    }
}

#[tonic::async_trait]
impl DragonClawAgent for DragonClawAgentImpl {
    async fn shutdown_system(&self, _: Request<()>) -> Result<Response<()>, Status> {
        tracing::trace!("Received shutdown request");

        match self.pal.shutdown_system().await {
            Ok(()) => Ok(Response::new(())),
            // If the shutdown fails this usually means that the system does not support it,
            // so we use the FailedPrecondition status code.
            Err(err) => Err(Status::failed_precondition(err.to_string())),
        }
    }
}

pub use dragon_claw_agent_server::DragonClawAgentServer;
