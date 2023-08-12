mod dns;
mod process;
mod service;
mod util;

use crate::pal::platform::discovery::dispatcher::ServiceDispatcher;
use crate::pal::platform::discovery::ServiceEnvironment;
use crate::pal::platform::process::OwnProcess;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use windows::core::{Error as Win32Error, PCWSTR};

use crate::pal::platform::dns::ServiceDnsRegistration;
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
    dns_registration: Mutex<Option<ServiceDnsRegistration>>,
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
            dns_registration: Mutex::new(None),
        })
    }

    pub async fn advertise_service(
        &self,
        addr: SocketAddr,
    ) -> Result<(), PlatformAbstractionError> {
        let mut dns_registration = self.dns_registration.lock().await;
        if let Some(registration) = dns_registration.take() {
            // If the service is registered we need to deregister it first
            registration
                .perform_deregistration()
                .await
                .map_err(PlatformError::Win32)?;
        }

        // Attempt to register the service
        let registration = ServiceDnsRegistration::create(addr).map_err(PlatformError::Win32)?;
        registration
            .perform_registration()
            .await
            .map_err(PlatformError::Win32)?;

        // Replace the old registration with the new one
        dns_registration.replace(registration);

        Ok(())
    }

    pub async fn stop_advertising_service(&self) -> Result<(), PlatformAbstractionError> {
        let mut dns_registration = self.dns_registration.lock().await;
        if let Some(registration) = dns_registration.take() {
            // If the service is registered we deregister it
            registration
                .perform_deregistration()
                .await
                .map_err(PlatformError::Win32)?;
        }

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
