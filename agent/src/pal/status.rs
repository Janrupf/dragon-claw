use crate::pal::PlatformAbstractionError;

#[derive(Debug)]
pub enum ApplicationStatus {
    /// The application is starting up
    Starting,

    /// The application is running
    Running,

    /// The application is shutting down
    Stopping,

    /// The application has stopped
    Stopped,

    /// The application has failed with platform error
    PlatformError(PlatformAbstractionError),

    /// The application has failed with application error
    ApplicationError(Box<dyn std::error::Error + Send + Sync>),
}

#[async_trait::async_trait]
pub trait StatusManager: Send + Sync + 'static {
    /// Sets the application status.
    async fn set_status(&self, status: ApplicationStatus);
}