#[zbus::dbus_proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait Login1Manager {
    async fn can_power_off(&self) -> zbus::Result<String>;

    async fn can_reboot(&self) -> zbus::Result<String>;

    async fn can_suspend(&self) -> zbus::Result<String>;

    async fn can_hibernate(&self) -> zbus::Result<String>;

    async fn can_hybrid_sleep(&self) -> zbus::Result<String>;

    async fn can_reboot_to_firmware_setup(&self) -> zbus::Result<String>;

    async fn power_off(&self, interactive: bool) -> zbus::Result<()>;

    async fn reboot(&self, interactive: bool) -> zbus::Result<()>;

    async fn suspend(&self, interactive: bool) -> zbus::Result<()>;

    async fn hibernate(&self, interactive: bool) -> zbus::Result<()>;

    async fn hybrid_sleep(&self, interactive: bool) -> zbus::Result<()>;

    async fn set_reboot_to_firmware_setup(&self, enable: bool) -> zbus::Result<()>;
}
