mod power;

tonic::include_proto!("net.janrupf.dc");

use crate::pal::PlatformAbstraction;
use dragon_claw_agent_server::*;
use std::sync::Arc;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct DragonClawAgentImpl {
    pal: Arc<PlatformAbstraction>,
}

impl DragonClawAgentImpl {
    pub fn new(pal: Arc<PlatformAbstraction>) -> Self {
        Self { pal }
    }
}

#[tonic::async_trait]
impl DragonClawAgent for DragonClawAgentImpl {
    async fn get_agent_version(
        &self,
        _request: Request<()>,
    ) -> Result<Response<AgentVersion>, Status> {
        // Get the version from Cargo.toml
        let major = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
        let minor = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
        let patch = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();
        let pre = env!("CARGO_PKG_VERSION_PRE");

        let version = AgentVersion {
            major,
            minor,
            patch,
            pre_release: if pre.is_empty() {
                None
            } else {
                Some(pre.to_string())
            },
        };

        Ok(Response::new(version))
    }

    async fn get_supported_power_actions(
        &self,
        _request: Request<()>,
    ) -> Result<Response<SupportedPowerActions>, Status> {
        let Some(power) = self.pal.power_manager() else {
            // Power management is not supported
            return Ok(Response::new(SupportedPowerActions { actions: vec![] }));
        };

        // Collect supported actions and translate to RPC
        let actions = power
            .get_supported_power_actions()
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .into_iter()
            .map(PowerAction::from)
            .map(|v| v as i32)
            .collect();
        Ok(Response::new(SupportedPowerActions { actions }))
    }

    async fn perform_power_action(
        &self,
        request: Request<PowerActionRequest>,
    ) -> Result<Response<()>, Status> {
        let Some(power) = self.pal.power_manager() else {
            // Power management is not supported
            return Err(Status::unimplemented("Power management is not supported"));
        };

        let action = match PowerAction::from_i32(request.into_inner().action) {
            Some(v) => v,
            None => return Err(Status::invalid_argument("Invalid power action")),
        };

        power
            .perform_power_action(action.into())
            .await
            .map_err(|err| Status::internal(err.to_string()))
            .map(Response::new)
    }
}

use crate::pal::power::PowerManager;
pub use dragon_claw_agent_server::DragonClawAgentServer;
