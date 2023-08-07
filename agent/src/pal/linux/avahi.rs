#![allow(clippy::too_many_arguments)]

use zbus::zvariant::Optional;

#[zbus::dbus_proxy(
    interface = "org.freedesktop.Avahi.Server2",
    default_service = "org.freedesktop.Avahi",
    default_path = "/"
)]
trait AvahiServer2 {
    async fn get_version_string(&self) -> zbus::Result<String>;

    async fn get_host_name(&self) -> zbus::Result<String>;

    #[dbus_proxy(object = "AvahiEntryGroup")]
    async fn entry_group_new(&self);
}

#[zbus::dbus_proxy(
    interface = "org.freedesktop.Avahi.EntryGroup",
    default_service = "org.freedesktop.Avahi",
    assume_defaults = false
)]
trait AvahiEntryGroup {
    async fn add_service(
        &self,
        interface: i32,
        protocol: i32,
        flags: u32,
        name: &str,
        ty: &str,
        domain: Optional<&str>,
        host: Optional<&str>,
        port: u16,
        txt: &[&[u8]],
    ) -> zbus::Result<()>;

    async fn commit(&self) -> zbus::Result<()>;
}
