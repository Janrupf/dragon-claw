use crate::pal::platform::service::dispatcher::ServiceDispatcher;
use crate::pal::platform::PlatformError;
use crate::pal::status::{ApplicationStatus, StatusManager};
use crate::pal::PlatformAbstractionError;
use std::sync::Arc;

#[derive(Debug)]
pub struct WindowsStatusManager {
    service_dispatcher: Option<Arc<ServiceDispatcher>>,
}

impl WindowsStatusManager {
    pub fn new(service_dispatcher: Option<Arc<ServiceDispatcher>>) -> Self {
        Self { service_dispatcher }
    }
}

#[async_trait::async_trait]
impl StatusManager for WindowsStatusManager {
    async fn set_status(&self, status: ApplicationStatus) {
        let Some(service_dispatcher) = self.service_dispatcher.as_ref() else {
            // If we don't have a service dispatcher we don't need to report the status
            return;
        };

        let res = match status {
            ApplicationStatus::Starting => service_dispatcher.report_start_pending(),
            ApplicationStatus::Running => service_dispatcher.report_running(),
            ApplicationStatus::Stopping => service_dispatcher.report_stopping(),
            ApplicationStatus::Stopped => service_dispatcher.report_stopped_ok(),
            ApplicationStatus::PlatformError(PlatformAbstractionError::Platform(
                PlatformError::Win32(err),
            )) => service_dispatcher.report_stopped_win32(err),
            ApplicationStatus::PlatformError(_) => {
                service_dispatcher.report_stopped_application_err(1)
            }
            ApplicationStatus::ApplicationError(_) => {
                service_dispatcher.report_stopped_application_err(2)
            }
        };

        if let Err(err) = res {
            tracing::warn!("Failed to report service status: {}", err);
        }
    }
}
