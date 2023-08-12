use crate::pal::platform::dbus::dbus_call;
use crate::pal::platform::dbus::login1::Login1ManagerProxy;
use crate::pal::platform::PlatformError;
use crate::pal::power::{PowerAction, PowerManager};
use crate::pal::PlatformAbstractionError;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct LinuxPowerManager {
    login1: Login1ManagerProxy<'static>,
}

impl LinuxPowerManager {
    pub async fn try_connect(dbus_connection: &zbus::Connection) -> Option<Self> {
        let login1 = match dbus_call!(Login1ManagerProxy::new(dbus_connection)).await {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(
                    "Failed to connect to Login1 Manager, shutdown will be unavailable: {}",
                    err
                );
                return None;
            }
        };

        Some(Self { login1 })
    }
}

macro_rules! test_power_action {
    ($actions:expr, $action:expr, $call:expr) => {{
        let call_fut = $call;
        let can_fut = ::futures_util::TryFutureExt::map_ok(call_fut, |v| v == "yes");
        let final_fut = ::futures_util::TryFutureExt::map_ok(can_fut, |v| {
            if v {
                $actions.push($action);
            }
        });

        ::futures_util::TryFutureExt::map_err(final_fut, $crate::pal::platform::PlatformError::from)
    }};
}

#[async_trait::async_trait]
impl PowerManager for LinuxPowerManager {
    async fn get_supported_power_actions(
        &self,
    ) -> Result<Vec<PowerAction>, PlatformAbstractionError> {
        let mut actions = Vec::new();

        test_power_action!(
            &mut actions,
            PowerAction::PowerOff,
            dbus_call!(self.login1.can_power_off())
        )
        .await?;
        test_power_action!(&mut actions, PowerAction::Reboot, self.login1.can_reboot()).await?;
        test_power_action!(
            &mut actions,
            PowerAction::Suspend,
            dbus_call!(self.login1.can_suspend())
        )
        .await?;
        test_power_action!(
            &mut actions,
            PowerAction::Hibernate,
            dbus_call!(self.login1.can_hibernate())
        )
        .await?;
        test_power_action!(
            &mut actions,
            PowerAction::HybridSuspend,
            dbus_call!(self.login1.can_hybrid_sleep())
        )
        .await?;
        test_power_action!(
            &mut actions,
            PowerAction::RebootToFirmware,
            dbus_call!(self.login1.can_reboot_to_firmware_setup())
        )
        .await?;

        Ok(actions)
    }

    async fn perform_power_action(
        &self,
        action: PowerAction,
    ) -> Result<(), PlatformAbstractionError> {
        tracing::trace!("Performing power action {:?}", action);

        let reboot_to_firmware = action == PowerAction::RebootToFirmware;
        self.login1
            .set_reboot_to_firmware_setup(reboot_to_firmware)
            .await
            .map_err(PlatformError::from)?;

        let login1 = self.login1.clone();

        // Translate the action to a future we can run
        let action_fut: Pin<Box<dyn Future<Output = zbus::Result<()>> + Send>> = match action {
            PowerAction::PowerOff => Box::pin(async move { login1.power_off(false).await }),
            PowerAction::Reboot => Box::pin(async move { login1.reboot(false).await }),
            PowerAction::Suspend => Box::pin(async move { login1.suspend(false).await }),
            PowerAction::Hibernate => Box::pin(async move { login1.hibernate(false).await }),
            PowerAction::HybridSuspend => Box::pin(async move { login1.hibernate(false).await }),
            PowerAction::RebootToFirmware => Box::pin(async move { login1.reboot(false).await }),
            PowerAction::Lock => return Err(PlatformAbstractionError::Unsupported),
            PowerAction::LogOut => return Err(PlatformAbstractionError::Unsupported),
        };

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if let Err(err) = action_fut.await {
                tracing::error!("Failed to perform power action: {}", err);
            }
        });

        Ok(())
    }
}
