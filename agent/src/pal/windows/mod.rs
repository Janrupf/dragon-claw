use std::borrow::Cow;
use std::net::SocketAddr;
use thiserror::Error;
use windows::core::{Error as Win32Error, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    DNS_REQUEST_PENDING, ERROR_MORE_DATA, ERROR_SUCCESS, HANDLE, WIN32_ERROR,
};

use crate::pal::PlatformAbstractionError;

#[derive(Debug)]
pub struct PlatformAbstractionImpl;

impl PlatformAbstractionImpl {
    pub async fn new() -> Result<Self, PlatformAbstractionError> {
        Self::acquire_privileges();
        Ok(Self)
    }

    fn acquire_privileges() {
        use windows::Win32::Security as security;
        use windows::Win32::System::Threading as threading;

        unsafe {
            let current_process = threading::GetCurrentProcess();
            let mut current_token = HANDLE(0);

            // Get our own process token so we can adjust privileges on it
            if !threading::OpenProcessToken(
                current_process,
                security::TOKEN_ADJUST_PRIVILEGES | security::TOKEN_QUERY,
                &mut current_token,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to open own process token: {}", err);
                return;
            }

            tracing::trace!("Opened own process token: {:?}", current_token);

            // Technically this loop could adjust multiple privileges with a single call to
            // AdjustTokenPrivileges - but since the TOKEN_PRIVILEGE structure is defined in a very
            // idiotic way by MS (array of size 1), we just iterator over the privileges and call
            // the function foreach privilege separately
            let privileges_to_acquire = [security::SE_SHUTDOWN_NAME];
            for to_acquire in privileges_to_acquire {
                let mut privilege = security::TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [security::LUID_AND_ATTRIBUTES {
                        Luid: Default::default(),
                        Attributes: security::SE_PRIVILEGE_ENABLED,
                    }],
                };

                // Attempt to look up the privilege LUID
                if !security::LookupPrivilegeValueW(
                    PCWSTR::null(),
                    to_acquire,
                    &mut privilege.Privileges[0].Luid,
                )
                .as_bool()
                {
                    let err = Win32Error::from_win32();
                    tracing::warn!(
                        "Failed to look up privilege {}: {}",
                        to_acquire.display(),
                        err
                    );
                    continue;
                }
                // Now adjust the privilege
                if !security::AdjustTokenPrivileges(
                    current_token,
                    false,
                    Some(&privilege),
                    0,
                    None,
                    None,
                )
                .as_bool()
                {
                    let err = Win32Error::from_win32();
                    tracing::warn!(
                        "Failed to adjust privilege {}: {}",
                        to_acquire.display(),
                        err
                    );
                } else {
                    tracing::trace!("Acquired privilege {}", to_acquire.display());
                }
            }

            tracing::debug!("Adjusted privileges!");
        }
    }

    pub async fn advertise_service(
        &self,
        addr: SocketAddr,
    ) -> Result<(), PlatformAbstractionError> {
        use windows::Win32::NetworkManagement::Dns as dns;
        use windows::Win32::System::SystemInformation as sysinfo;

        let mut host_name = unsafe {
            // Retrieve required buffer length
            let mut buffer_size = 0;
            if !sysinfo::GetComputerNameExW(
                sysinfo::ComputerNameDnsHostname,
                PWSTR::null(),
                &mut buffer_size,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                // ERROR_MORE_DATA is ok, everything else is fatal
                if err != Win32Error::from(ERROR_MORE_DATA) {
                    tracing::error!("Failed to retrieve buffer length for DNS hostname: {}", err);
                    return Err(PlatformError::Win32(err).into());
                }
            }

            // Use a Vec as a memory buffer for a PWSTR
            // We also directly reserve 6 bytes more so we later can append .local without
            // reallocation
            let mut buffer = Vec::with_capacity((buffer_size as usize) + 6);

            if !sysinfo::GetComputerNameExW(
                sysinfo::ComputerNameDnsHostname,
                PWSTR::from_raw(buffer.as_mut_ptr()),
                &mut buffer_size,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to retrieve DNS hostname: {}", err);
                return Err(PlatformError::Win32(err).into());
            }

            // Set the real length, this will cut the null terminator, but since
            // we are first using this buffer as a Rust slice and then later manually
            // append .local\0, this is not a problem
            buffer.set_len(buffer_size as usize);

            buffer
        };

        // Prepare a buffer to store the name into
        let computer_name = match String::from_utf16(&host_name) {
            Ok(v) => Cow::Owned(v),
            Err(err) => {
                tracing::warn!("Failed to convert computer name to UTF-8: {}", err);
                Cow::Borrowed("Dragon Claw Computer")
            }
        };

        // Append .local to host name
        host_name.extend(".local".encode_utf16());
        host_name.push(0);

        // Format and encode to UTF-16
        let service_instance_name = format!("{computer_name}._dragon-claw._tcp.local");
        tracing::trace!("Service name: {}", service_instance_name);

        let mut service_instance_name = service_instance_name
            .encode_utf16()
            .chain([0u16])
            .collect::<Vec<u16>>();

        // Translate the rust address representation to the Win32 representation
        let (mut ipv4, mut ipv6) = match addr {
            SocketAddr::V4(addr) => {
                let ipv4 = u32::from_be_bytes(addr.ip().octets());

                (Some(ipv4), None)
            }
            SocketAddr::V6(addr) => {
                let ipv6 = dns::IP6_ADDRESS {
                    IP6Byte: addr.ip().octets(),
                };

                (None, Some(ipv6))
            }
        };

        // Construct a DNS service instance to register
        let mut service_instance = dns::DNS_SERVICE_INSTANCE {
            pszInstanceName: PWSTR::from_raw(service_instance_name.as_mut_ptr()),
            pszHostName: PWSTR::from_raw(host_name.as_mut_ptr()),
            ip4Address: ipv4
                .as_mut()
                .map(|v| v as *mut u32)
                .unwrap_or(std::ptr::null_mut()),
            ip6Address: ipv6
                .as_mut()
                .map(|v| v as *mut dns::IP6_ADDRESS)
                .unwrap_or(std::ptr::null_mut()),
            wPort: addr.port(),
            wPriority: 0,
            wWeight: 0,
            dwPropertyCount: 0,
            keys: std::ptr::null_mut(),
            values: std::ptr::null_mut(),
            dwInterfaceIndex: 0,
        };

        let (mut complete_sender, complete_receiver) = tokio::sync::oneshot::channel::<u32>();

        #[tracing::instrument]
        unsafe extern "system" fn dns_registration_complete(
            status: u32,
            context: *const std::ffi::c_void,
            instance: *const dns::DNS_SERVICE_INSTANCE,
        ) {
            tracing::trace!("DNS service registration complete!");

            if !instance.is_null() {
                dns::DnsServiceFreeInstance(instance);
            }

            // Send the status back
            let complete_sender =
                std::ptr::read::<tokio::sync::oneshot::Sender<u32>>(context as *const _);
            complete_sender.send(status).unwrap();
        }

        let register_request = dns::DNS_SERVICE_REGISTER_REQUEST {
            Version: dns::DNS_QUERY_REQUEST_VERSION1.0,
            InterfaceIndex: 0,
            pServiceInstance: &mut service_instance,
            pRegisterCompletionCallback: Some(dns_registration_complete),
            pQueryContext: &mut complete_sender as *mut _ as *mut _,
            hCredentials: HANDLE(0),
            unicastEnabled: false.into(),
        };

        // Dispatch the request
        //
        // De-registration of the service is automatically performed by windows when this process
        // exits.
        unsafe {
            let res = dns::DnsServiceRegister(&register_request, None);
            if res != (DNS_REQUEST_PENDING as u32) {
                let err = Win32Error::from_win32();
                tracing::error!(
                    "Failed to submit DNS service registration: {}, {}",
                    res,
                    err
                );
                return Err(PlatformError::Win32(err).into());
            }
        }

        // The sender is now owned by the callback
        std::mem::forget(complete_sender);
        let status = complete_receiver.await.unwrap();
        let status = WIN32_ERROR(status);

        if status != ERROR_SUCCESS {
            let err = Win32Error::from(status);
            tracing::error!("DNS service registration failed: {}", err);
            return Err(PlatformError::Win32(err).into());
        }

        tracing::debug!("DNS service registered!");

        Ok(())
    }

    pub async fn shutdown_system(&self) -> Result<(), PlatformAbstractionError> {
        unsafe {
            use windows::Win32::System::Shutdown as shtdn;

            if !shtdn::InitiateSystemShutdownExW(
                PCWSTR::null(),
                PCWSTR::null(),
                5, // This should give us a chance to send the response over RPC
                true,
                false,
                shtdn::SHTDN_REASON_MAJOR_OTHER
                    | shtdn::SHTDN_REASON_MINOR_OTHER
                    | shtdn::SHTDN_REASON_FLAG_PLANNED,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to initiate a system shutdown: {}", err);
                return Err(PlatformError::Win32(err).into());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error(transparent)]
    Win32(Win32Error),
}
