use windows::core::{Error as Win32Error, PCWSTR};
use windows::imp::CloseHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[derive(Debug)]
pub struct OwnProcess {
    own_process: HANDLE,
    process_token: HANDLE,
}

impl OwnProcess {
    /// Opens the currently running process and acquires handles to it.
    pub fn open() -> Result<Self, Win32Error> {
        unsafe {
            let own_process = GetCurrentProcess();

            let mut process_token_handle = HANDLE::default();
            if !OpenProcessToken(
                own_process,
                TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES,
                &mut process_token_handle,
            )
            .as_bool()
            {
                // Not sure when even this would happen, but let's be safe
                return Err(Win32Error::from_win32());
            }

            Ok(Self {
                own_process,
                process_token: process_token_handle,
            })
        }
    }

    /// Enables the specified privileges for the current process.
    pub fn enable_privileges(&self, privileges: &[PCWSTR]) -> Result<(), Win32Error> {
        unsafe {
            // Technically this loop could adjust multiple privileges with a single call to
            // AdjustTokenPrivileges - but since the TOKEN_PRIVILEGE structure is defined in a very
            // idiotic way by MS (array of size 1), we just iterator over the privileges and call
            // the function foreach privilege separately
            for to_acquire in privileges {
                let mut privilege = TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [LUID_AND_ATTRIBUTES {
                        Luid: Default::default(),
                        Attributes: SE_PRIVILEGE_ENABLED,
                    }],
                };

                // Attempt to look up the privilege LUID
                if !LookupPrivilegeValueW(
                    PCWSTR::null(),
                    *to_acquire,
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
                if !AdjustTokenPrivileges(
                    self.process_token,
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
                    tracing::trace!("Enabled privilege {}", to_acquire.display());
                }
            }

            tracing::debug!("Adjusted privileges!");
        }

        Ok(())
    }

    /// Returns the process handle of the current process.
    pub fn process_handle(&self) -> HANDLE {
        self.own_process
    }

    /// Returns the process token of the current process.
    pub fn process_token(&self) -> HANDLE {
        self.process_token
    }
}

impl Drop for OwnProcess {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.process_token.0);

            // Not required, since its a pseudo-handle, but it doesn't hurt
            CloseHandle(self.own_process.0);
        }
    }
}
