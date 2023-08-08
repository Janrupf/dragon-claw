use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::Mutex;
use tokio::sync::oneshot::Sender as OneshotSender;
use windows::core::{w as wide_str, Error as Win32Error, PWSTR};
use windows::Win32::Foundation::{
    ERROR_CALL_NOT_IMPLEMENTED, ERROR_INVALID_STATE, ERROR_SERVICE_SPECIFIC_ERROR, NO_ERROR,
    WIN32_ERROR,
};
use windows::Win32::System::Services::{
    RegisterServiceCtrlHandlerExW, SetServiceStatus, StartServiceCtrlDispatcherW,
    SERVICE_ACCEPT_STOP, SERVICE_CONTROL_INTERROGATE, SERVICE_CONTROL_STOP, SERVICE_RUNNING,
    SERVICE_START_PENDING, SERVICE_STATUS, SERVICE_STATUS_CURRENT_STATE, SERVICE_STATUS_HANDLE,
    SERVICE_STOPPED, SERVICE_STOP_PENDING, SERVICE_TABLE_ENTRYW, SERVICE_WIN32_OWN_PROCESS,
};

use crate::pal::platform::util::ToWin32ErrorCode;
use crate::pal::ShutdownRequestFut;

enum ServiceData<F, R>
where
    F: FnOnce(ServiceDispatcher, ShutdownRequestFut) -> R,
{
    /// The main function to launch
    Launch(F),

    /// The result of the main function
    Return(R),

    /// The dispatcher failed
    Failed(Win32Error),
}
static SERVICE_DATA: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

#[derive(Debug)]
enum ServiceExitCode {
    Success,
    Win32(WIN32_ERROR),
    ServiceSpecific(u32),
}

#[derive(Debug)]
struct ServiceCtrlContext {
    shutdown_sender: Mutex<Option<OneshotSender<()>>>,
    check_point: AtomicU32,
    status_handle: SERVICE_STATUS_HANDLE,
}

impl ServiceCtrlContext {
    pub fn new(shutdown_sender: OneshotSender<()>) -> Self {
        Self {
            shutdown_sender: Mutex::new(Some(shutdown_sender)),
            check_point: AtomicU32::new(1),
            status_handle: SERVICE_STATUS_HANDLE::default(),
        }
    }

    pub fn report_status(
        &self,
        status: SERVICE_STATUS_CURRENT_STATE,
        exit_code: ServiceExitCode,
        wait_hint: u32,
    ) -> Result<(), Win32Error> {
        let accepted_controls = match status {
            SERVICE_START_PENDING => 0,

            // We only ever accept stop requests
            _ => SERVICE_ACCEPT_STOP,
        };

        // Translate the wanted exit code
        let (win32_exit_code, service_specific_exit_code) = match exit_code {
            ServiceExitCode::Success => (NO_ERROR.0, 0),
            ServiceExitCode::Win32(code) => (code.0, 0),
            ServiceExitCode::ServiceSpecific(code) => (ERROR_SERVICE_SPECIFIC_ERROR.0, code),
        };

        let status = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: status,
            dwControlsAccepted: accepted_controls,
            dwWin32ExitCode: win32_exit_code,
            dwServiceSpecificExitCode: service_specific_exit_code,
            dwCheckPoint: self.check_point.fetch_add(1, Ordering::AcqRel),
            dwWaitHint: wait_hint,
        };

        tracing::trace!("Reporting service status: {:?}", status);

        let res = unsafe { SetServiceStatus(self.status_handle, &status).as_bool() };

        if res {
            Ok(())
        } else {
            Err(Win32Error::from_win32())
        }
    }
}

#[derive(Debug)]
pub struct ServiceDispatcher {
    ctx: Box<ServiceCtrlContext>,
}

impl ServiceDispatcher {
    pub fn dispatch_service_main<F, R>(main: F) -> Result<R, Win32Error>
    where
        F: FnOnce(ServiceDispatcher, ShutdownRequestFut) -> R,
    {
        // Wrap the main function into a box
        let launch_data =
            Box::into_raw(Box::new(ServiceData::Launch(main))) as *mut std::ffi::c_void;

        // Store the launch data in a global variable
        if SERVICE_DATA
            .compare_exchange(
                std::ptr::null_mut(),
                launch_data,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_err()
        {
            // Drop the launch data, it has not been stored
            drop(unsafe { Box::from_raw(launch_data as *mut F) });

            tracing::error!("Attempted to dispatch service main function more than once");
            return Err(Win32Error::from(ERROR_INVALID_STATE));
        }

        // For some reason the service name must be a **mutable** wide string
        // I doubt it is ever modified, but who knows, its Windows
        let mut service_name = "DragonClawAgent"
            .encode_utf16()
            .chain([0u16])
            .collect::<Vec<u16>>();

        let dispatch_table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR(service_name.as_mut_ptr()),
                lpServiceProc: Some(Self::service_main_function::<F, R>),
            },
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR::null(),
                lpServiceProc: None,
            },
        ];

        // Start the service dispatcher
        tracing::trace!("Starting service dispatcher");
        let res = unsafe { StartServiceCtrlDispatcherW(dispatch_table.as_ptr()).as_bool() };

        // If we get here either the service main returned or the call above failed,
        // so we can in both cases safely load the data back
        let data = *unsafe {
            Box::from_raw(
                SERVICE_DATA.swap(std::ptr::null_mut(), Ordering::AcqRel) as *mut ServiceData<F, R>
            )
        };

        if !res {
            // Dispatch failed
            return Err(Win32Error::from_win32());
        }

        match data {
            ServiceData::Return(ret) => Ok(ret),
            ServiceData::Failed(err) => Err(err),
            _ => unreachable!("Service data was not Return or Failed when service main returned"),
        }
    }

    extern "system" fn service_main_function<F, R>(_argc: u32, _argv: *mut PWSTR)
    where
        F: FnOnce(ServiceDispatcher, ShutdownRequestFut) -> R,
    {
        // Acquire the launch data
        // We do not reset the pointer, since we also use it to indicate that we already
        // dispatched the service. Since the function above never attempts to deallocate the data
        // or overwrite it when its not null, this is perfectly safe and doesn't leak memory.
        let f = *unsafe {
            Box::from_raw(SERVICE_DATA.load(Ordering::Acquire) as *mut ServiceData<F, R>)
        };
        let main = match f {
            ServiceData::Launch(main) => main,
            _ => unreachable!("Service data was not Launch when service main was invoked"),
        };

        let return_data = Self::dispatcher_proxy(main);

        // Store the return data in the global variable
        let return_data = Box::into_raw(Box::new(return_data)) as *mut std::ffi::c_void;
        SERVICE_DATA.store(return_data, Ordering::Release);
    }

    fn dispatcher_proxy<F, R>(main: F) -> ServiceData<F, R>
    where
        F: FnOnce(ServiceDispatcher, ShutdownRequestFut) -> R,
    {
        let (dispatcher, shutdown_fut) = match Self::instantiate_in_service() {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("Failed to instantiate service dispatcher: {}", err);
                return ServiceData::Failed(err);
            }
        };

        tracing::trace!("Proxying service main function");

        // invoke the main function
        let ret = (main)(dispatcher, shutdown_fut);
        ServiceData::Return(ret)
    }

    /// Instantiates the service dispatcher in the current process after the service main has been
    /// dispatched.
    fn instantiate_in_service() -> Result<(Self, ShutdownRequestFut), Win32Error> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();

        let mut ctx = Box::new(ServiceCtrlContext::new(sender));

        // We are running in a service main
        let status_handle = unsafe {
            RegisterServiceCtrlHandlerExW(
                wide_str!("DragonClawAgent"),
                Some(Self::service_ctrl_handler),
                Some(ctx.as_mut() as *mut _ as *mut std::ffi::c_void),
            )
        }?;

        let shutdown_fut = Box::pin(async move {
            let _ = receiver.await;
        });

        ctx.status_handle = status_handle;
        Ok((Self { ctx }, shutdown_fut))
    }

    extern "system" fn service_ctrl_handler(
        ctrl: u32,
        _event_type: u32,
        _event_data: *mut std::ffi::c_void,
        context: *mut std::ffi::c_void,
    ) -> u32 {
        tracing::trace!("Received service control request: {}", ctrl);
        let ctx = unsafe { &*(context as *mut ServiceCtrlContext) };

        let res = match ctrl {
            SERVICE_CONTROL_STOP => {
                tracing::info!("Received stop request!");
                let _ = ctx.report_status(SERVICE_STOP_PENDING, ServiceExitCode::Success, 5000);

                // Lock the mutex and send the shutdown request
                match ctx.shutdown_sender.lock() {
                    Ok(mut v) => {
                        if let Some(sender) = v.take() {
                            let res = sender.send(());
                            tracing::trace!("Dispatched shutdown request: {:?}", res);
                        }

                        NO_ERROR
                    }
                    Err(err) => {
                        tracing::error!("Shutdown lock poisoned: {}", err);
                        ERROR_INVALID_STATE
                    }
                }
            }
            // Interrogation always needs to be handled
            SERVICE_CONTROL_INTERROGATE => NO_ERROR,

            // Unimplemented call, we should never get here but who knows what
            // other components on the system do
            _ => ERROR_CALL_NOT_IMPLEMENTED,
        };

        res.0
    }

    pub fn report_start_pending(&self) -> Result<(), Win32Error> {
        self.ctx
            .report_status(SERVICE_START_PENDING, ServiceExitCode::Success, 5000)
    }

    pub fn report_running(&self) -> Result<(), Win32Error> {
        self.ctx
            .report_status(SERVICE_RUNNING, ServiceExitCode::Success, 0)
    }

    pub fn report_stopping(&self) -> Result<(), Win32Error> {
        self.ctx
            .report_status(SERVICE_STOP_PENDING, ServiceExitCode::Success, 5000)
    }

    pub fn report_stopped_ok(&self) -> Result<(), Win32Error> {
        self.ctx
            .report_status(SERVICE_STOPPED, ServiceExitCode::Success, 0)
    }

    pub fn report_stopped_win32(&self, err: Win32Error) -> Result<(), Win32Error> {
        self.ctx.report_status(
            SERVICE_STOPPED,
            ServiceExitCode::Win32(err.to_win32_error_code()),
            0,
        )
    }

    pub fn report_stopped_application_err(&self, err: u32) -> Result<(), Win32Error> {
        self.ctx
            .report_status(SERVICE_STOPPED, ServiceExitCode::ServiceSpecific(err), 0)
    }
}
