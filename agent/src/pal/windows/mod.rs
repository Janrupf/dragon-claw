mod process;
mod service;
mod util;

use crate::pal::platform::process::OwnProcess;
use crate::pal::platform::service::dispatcher::ServiceDispatcher;
use crate::pal::platform::service::ServiceEnvironment;
use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use windows::core::{Error as Win32Error, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    DNS_REQUEST_PENDING, ERROR_MORE_DATA, ERROR_SUCCESS, HANDLE, WIN32_ERROR,
};

use crate::pal::{ApplicationStatus, PlatformAbstractionError, ShutdownRequestFut};

#[derive(Debug)]
pub struct PlatformInitData {
    process: OwnProcess,
    service_environment: ServiceEnvironment,
    service_dispatcher: Option<Arc<ServiceDispatcher>>,
}

#[derive(Debug)]
pub struct PlatformAbstractionImpl {
    process: OwnProcess,
    service_environment: ServiceEnvironment,
    service_dispatcher: Option<Arc<ServiceDispatcher>>,
}

impl PlatformAbstractionImpl {
    pub fn dispatch_main<F, R>(main: F) -> Result<R, PlatformAbstractionError>
    where
        F: FnOnce(PlatformInitData, ShutdownRequestFut) -> R,
    {
        // Initialize process data
        let mut init_data = Self::perform_pre_init()?;

        if init_data.service_environment != ServiceEnvironment::None {
            // We need to perform service specific initialization
            ServiceDispatcher::dispatch_service_main(move |dispatcher, shutdown_fut| {
                // We are now running as a real windows service
                let dispatcher = Arc::new(dispatcher);
                init_data.service_dispatcher = Some(dispatcher);

                main(init_data, shutdown_fut)
            })
            .map_err(PlatformError::Win32)
            .map_err(PlatformAbstractionError::Platform)
        } else {
            // Not a service, run the main without a wrapper
            Ok(main(init_data, crate::pal::ctrl_c_shutdown_fut()))
        }
    }

    fn perform_pre_init() -> Result<PlatformInitData, PlatformError> {
        let process = OwnProcess::open().map_err(PlatformError::Win32)?;

        // Make sure we can shutdown the system
        if let Err(err) = process.enable_privileges(&[windows::Win32::Security::SE_SHUTDOWN_NAME]) {
            tracing::warn!("Failed to enable shutdown privilege: {}", err);
        }

        let service_environment = match ServiceEnvironment::detect(&process) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("Failed to detect service environment: {}", err);
                tracing::warn!("Assuming to be running as a normal application");
                ServiceEnvironment::None
            }
        };

        tracing::trace!("Service environment: {:?}", service_environment);

        Ok(PlatformInitData {
            process,
            service_environment,
            service_dispatcher: None,
        })
    }

    pub async fn new(data: PlatformInitData) -> Result<Self, PlatformAbstractionError> {
        Ok(Self {
            process: data.process,
            service_environment: data.service_environment,
            service_dispatcher: data.service_dispatcher,
        })
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

    pub async fn set_status(&self, status: ApplicationStatus) {
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

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error(transparent)]
    Win32(Win32Error),
}
