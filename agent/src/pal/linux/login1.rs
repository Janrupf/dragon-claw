#[zbus::dbus_proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait Login1Manager {
    async fn can_power_off(&self) -> zbus::Result<String>;

    async fn power_off(&self, interactive: bool) -> zbus::Result<()>;
}
