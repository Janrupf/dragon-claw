use crate::pal::PlatformAbstractionError;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PowerAction {
    /// Power-off the system
    PowerOff,

    /// Reboot the system
    Reboot,

    /// Reboot the system into the firmware setup
    RebootToFirmware,

    /// Lock the screen
    Lock,

    /// Log out the current user
    LogOut,

    /// Suspend the system
    Suspend,

    /// Hibernate the system
    Hibernate,

    /// Hybrid-suspend the system
    HybridSuspend,
}

#[async_trait::async_trait]
pub trait PowerManager: Send + Sync + 'static {
    /// Retrieves the power actions that are supported by the system
    async fn get_supported_power_actions(
        &self,
    ) -> Result<Vec<PowerAction>, PlatformAbstractionError>;

    /// Performs a power action
    async fn perform_power_action(
        &self,
        action: PowerAction,
    ) -> Result<(), PlatformAbstractionError>;
}
