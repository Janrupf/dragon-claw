use crate::pal::platform::name::ComputerName;
use std::net::SocketAddr;
use std::sync::Mutex;
use tokio::sync::oneshot::{Receiver as OneshotReceiver, Sender as OneshotSender};
use windows::core::{Error as Win32Error, PCWSTR};
use windows::Win32::Foundation::{DNS_REQUEST_PENDING, ERROR_SUCCESS, HANDLE, WIN32_ERROR};
use windows::Win32::NetworkManagement::Dns::{
    DnsServiceConstructInstance, DnsServiceDeRegister, DnsServiceFreeInstance, DnsServiceRegister,
    DNS_QUERY_REQUEST_VERSION1, DNS_SERVICE_INSTANCE, DNS_SERVICE_REGISTER_REQUEST, IP6_ADDRESS,
};

#[derive(Debug)]
struct DnsRegistrationContextInner {
    complete_sender: Option<OneshotSender<u32>>,
}

type DnsRegistrationContext = Mutex<DnsRegistrationContextInner>;

#[derive(Debug)]
pub struct ServiceDnsRegistration {
    register_request: DNS_SERVICE_REGISTER_REQUEST,
    service_instance: *mut DNS_SERVICE_INSTANCE,
    context: Box<DnsRegistrationContext>,
}

impl ServiceDnsRegistration {
    pub fn create(
        addr: SocketAddr,
        computer_name: ComputerName,
        service_name: &str,
    ) -> Result<Self, Win32Error> {
        let mut host_name = computer_name.into_dns_host_name();

        // Append .local to host name
        host_name.extend(".local".encode_utf16());
        host_name.push(0);

        // Format and encode to UTF-16
        let service_instance_name = format!("{service_name}._dragon-claw._tcp.local");
        tracing::trace!("Service name: {}", service_instance_name);

        let service_instance_name = service_instance_name
            .encode_utf16()
            .chain([0u16])
            .collect::<Vec<u16>>();

        // Translate the rust address representation to the Win32 representation
        let (ipv4, ipv6) = match addr {
            SocketAddr::V4(addr) => {
                let ipv4 = u32::from_be_bytes(addr.ip().octets());

                (Some(ipv4), None)
            }
            SocketAddr::V6(addr) => {
                let ipv6 = IP6_ADDRESS {
                    IP6Byte: addr.ip().octets(),
                };

                (None, Some(ipv6))
            }
        };

        // Construct a DNS service instance to register
        let service_instance = unsafe {
            DnsServiceConstructInstance(
                PCWSTR::from_raw(service_instance_name.as_ptr()),
                PCWSTR::from_raw(host_name.as_ptr()),
                ipv4.as_ref().map(|v| v as *const u32),
                ipv6.as_ref().map(|v| v as *const IP6_ADDRESS),
                addr.port(),
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        let mut context = Box::new(Mutex::new(DnsRegistrationContextInner {
            complete_sender: None,
        }));

        let register_request = DNS_SERVICE_REGISTER_REQUEST {
            Version: DNS_QUERY_REQUEST_VERSION1.0,
            InterfaceIndex: 0,
            pServiceInstance: service_instance,
            pRegisterCompletionCallback: Some(Self::dns_registration_callback),
            pQueryContext: context.as_mut() as *mut _ as *mut _,
            hCredentials: HANDLE(0),
            unicastEnabled: false.into(),
        };

        Ok(Self {
            register_request,
            service_instance,
            context,
        })
    }

    pub async fn perform_registration(&self) -> Result<(), Win32Error> {
        let complete_receiver = self.update_callback_sender();

        // Dispatch the request
        //
        // De-registration of the service is automatically performed by windows when this process
        // exits.
        unsafe {
            let res = DnsServiceRegister(&self.register_request, None);
            if res != (DNS_REQUEST_PENDING as u32) {
                let err = Win32Error::from_win32();
                tracing::error!(
                    "Failed to submit DNS service registration: {}, {}",
                    res,
                    err
                );
                return Err(err);
            }
        }

        let status = complete_receiver.await.unwrap();
        let status = WIN32_ERROR(status);

        if status != ERROR_SUCCESS {
            let err = Win32Error::from(status);
            tracing::error!("DNS service registration failed: {}", err);
            return Err(err);
        }

        tracing::debug!("DNS service registered!");

        Ok(())
    }

    pub async fn perform_deregistration(&self) -> Result<(), Win32Error> {
        let complete_receiver = self.update_callback_sender();

        // Dispatch the request
        unsafe {
            let res = DnsServiceDeRegister(&self.register_request, None);
            if res != (DNS_REQUEST_PENDING as u32) {
                let err = Win32Error::from_win32();
                tracing::error!(
                    "Failed to submit DNS service de-registration: {}, {}",
                    res,
                    err
                );
                return Err(err);
            }
        }

        let status = complete_receiver.await.unwrap();
        let status = WIN32_ERROR(status);

        if status != ERROR_SUCCESS {
            let err = Win32Error::from(status);
            tracing::error!("DNS service de-registration failed: {}", err);
            return Err(err);
        }

        tracing::debug!("DNS service de-registered!");

        Ok(())
    }

    // Updates the callback sender in the context and returns the receiver
    fn update_callback_sender(&self) -> OneshotReceiver<u32> {
        let (complete_sender, complete_receiver) = tokio::sync::oneshot::channel();

        // Update the sender in the context
        let mut context = self
            .context
            .lock()
            .expect("Poisoned DNS registration context lock");
        context.complete_sender.replace(complete_sender);

        complete_receiver
    }

    unsafe extern "system" fn dns_registration_callback(
        status: u32,
        context: *const std::ffi::c_void,
        instance: *const DNS_SERVICE_INSTANCE,
    ) {
        tracing::trace!("DNS service operation callback invoked");

        if !instance.is_null() {
            DnsServiceFreeInstance(instance);
        }

        // Acquire the context
        let context = &*(context as *const DnsRegistrationContext);
        let mut context = context
            .lock()
            .expect("Poisoned DNS registration context lock");

        // Send the status back
        if let Some(sender) = context.complete_sender.take() {
            tracing::trace!("Sending DNS registration operation status back");
            let _ = sender.send(status);
        } else {
            tracing::error!("No sender to send DNS registration operation status back to")
        }
    }
}

impl Drop for ServiceDnsRegistration {
    fn drop(&mut self) {
        unsafe {
            DnsServiceFreeInstance(self.service_instance);
        }
    }
}

// The rust compiler complains about not being able to send pointers across threads -
// we don't hold pointers that are valid on only one thread.
unsafe impl Send for ServiceDnsRegistration {}
unsafe impl Sync for ServiceDnsRegistration {}
