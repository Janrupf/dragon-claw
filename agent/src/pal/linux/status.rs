use crate::pal::status::{ApplicationStatus, StatusManager};

#[derive(Debug)]
pub struct LinuxStatusManager;

#[async_trait::async_trait]
impl StatusManager for LinuxStatusManager {
    async fn set_status(&self, _: ApplicationStatus) {
        // no-op
    }
}
