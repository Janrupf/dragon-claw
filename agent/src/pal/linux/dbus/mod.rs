pub mod avahi;
pub mod login1;

pub const DBUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

macro_rules! dbus_call {
    ($e:expr) => {{
        ::futures_util::FutureExt::map(
            ::tokio::time::timeout($crate::pal::platform::dbus::DBUS_TIMEOUT, $e),
            |v| match v {
                Err(_) => Err($crate::pal::platform::PlatformError::DbusTimeout),
                Ok(Err(err)) => Err($crate::pal::platform::PlatformError::Dbus(err)),
                Ok(Ok(v)) => Ok(v),
            },
        )
    }};
}

pub(crate) use dbus_call;
