pub mod dispatcher;

use crate::pal::platform::process::OwnProcess;
use windows::core::Error as Win32Error;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, PSID};
use windows::Win32::Security::{
    AllocateAndInitializeSid, EqualSid, FreeSid, GetTokenInformation, TokenGroups,
    SECURITY_NT_AUTHORITY, TOKEN_GROUPS,
};
use windows::Win32::System::SystemServices::{SECURITY_INTERACTIVE_RID, SECURITY_SERVICE_RID};

#[derive(Debug, Eq, PartialEq)]
pub enum ServiceEnvironment {
    /// Running in an interactive session
    None,

    /// Running as a user service
    User,

    /// Running as a system service
    System,
}

impl ServiceEnvironment {
    pub fn detect(process: &OwnProcess) -> Result<Self, Win32Error> {
        let token = process.process_token();

        let token_groups = unsafe {
            let mut buffer_size = 0;

            // Check how big the buffer needs to be
            if !GetTokenInformation(token, TokenGroups, None, 0, &mut buffer_size).as_bool() {
                let err = Win32Error::from_win32();
                if err != Win32Error::from(ERROR_INSUFFICIENT_BUFFER) {
                    tracing::error!("Failed to retrieve buffer length for token groups: {}", err);
                    return Err(err);
                }
            }

            tracing::trace!("Buffer size for token groups: {}", buffer_size);

            // Allocate a buffer and load the data into it
            let mut buffer = Vec::<u8>::with_capacity(buffer_size as usize);
            if !GetTokenInformation(
                token,
                TokenGroups,
                Some(buffer.as_mut_ptr() as _),
                buffer.capacity() as _,
                &mut buffer_size,
            )
            .as_bool()
            {
                return Err(Win32Error::from_win32());
            }

            // Adjust the length of the buffer to the actual number of entries
            buffer.set_len(buffer_size as usize);

            buffer
        };

        // Cast the buffer to a TOKEN_GROUPS struct
        let token_groups = unsafe { &*(token_groups.as_ptr() as *const TOKEN_GROUPS) };

        let mut interactive_sid = PSID::default();
        let mut service_sid = PSID::default();

        // Retrieve the interactive and service SID
        unsafe {
            if !AllocateAndInitializeSid(
                &SECURITY_NT_AUTHORITY,
                1,
                SECURITY_INTERACTIVE_RID as _,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                &mut interactive_sid,
            )
            .as_bool()
            {
                return Err(Win32Error::from_win32());
            }

            if !AllocateAndInitializeSid(
                &SECURITY_NT_AUTHORITY,
                1,
                SECURITY_SERVICE_RID as _,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                &mut service_sid,
            )
            .as_bool()
            {
                FreeSid(interactive_sid);
                return Err(Win32Error::from_win32());
            }
        };

        let mut is_interactive = false;
        let mut is_service = false;

        for i in 0..token_groups.GroupCount {
            // The Groups array is defined as an array of size 1, but in reality its an array of
            // size GroupCount, so we need to offset the pointer by i to get the correct entry
            // instead of directly accessing the index in order to avoid out-of-bounds checks.
            let sid_and_attributes = unsafe { &*token_groups.Groups.as_ptr().offset(i as _) };

            unsafe {
                // Check if the SID matches the interactive SID
                if EqualSid(sid_and_attributes.Sid, interactive_sid).as_bool() {
                    is_interactive = true;
                    break;
                }

                // Check if the SID matches the service SID
                if EqualSid(sid_and_attributes.Sid, service_sid).as_bool() {
                    is_service = true;
                    break;
                }
            }
        }

        // Free all the things
        unsafe {
            FreeSid(service_sid);
            FreeSid(interactive_sid);
        }

        tracing::trace!(
            "Is service: {}, is interactive: {}",
            is_service,
            is_interactive
        );

        match (is_service, is_interactive) {
            // Running interactively
            (_, true) => Ok(Self::None),

            // Service is set, running as a user service
            (true, _) => Ok(Self::User),

            // Service and interactive are not set, running as a system service
            (false, _) => Ok(Self::System),
        }
    }
}
