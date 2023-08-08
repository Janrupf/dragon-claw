use thiserror::Error;

#[derive(Debug, Error)]
pub enum DragonClawAgentError {
    #[error("rpc failed: {0}")]
    RpcError(#[from] tonic::Status),

    #[error("rpc failed: {0}")]
    Tonic(Box<dyn std::error::Error + Send + Sync>),
    
    #[error("rpc transport failed: {0}")]
    TonicTransport(#[from] tonic::transport::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
