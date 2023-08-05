tonic::include_proto!("net.janrupf.dc");

use dragon_claw_agent_server::*;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct DragonClawAgentImpl {}

impl DragonClawAgentImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[tonic::async_trait]
impl DragonClawAgent for DragonClawAgentImpl {
    async fn shutdown_system(&self, _: Request<()>) -> Result<Response<()>, Status> {
        tracing::trace!("Received shutdown request");

        Ok(Response::new(()))
    }
}

pub use dragon_claw_agent_server::DragonClawAgentServer;
