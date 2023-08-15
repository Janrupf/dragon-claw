use std::string::FromUtf16Error;
use windows::core::{Error as Win32Error, PWSTR};
use windows::Win32::Foundation::ERROR_MORE_DATA;
use windows::Win32::System::SystemInformation::{ComputerNameDnsHostname, GetComputerNameExW};

#[derive(Debug, Clone)]
pub struct ComputerName {
    dns_host_name: Vec<u16>,
}

impl ComputerName {
    pub fn determine() -> Result<Self, Win32Error> {
        let dns_host_name = unsafe {
            // Retrieve required buffer length
            let mut buffer_size = 0;
            if !GetComputerNameExW(ComputerNameDnsHostname, PWSTR::null(), &mut buffer_size)
                .as_bool()
            {
                let err = Win32Error::from_win32();
                // ERROR_MORE_DATA is ok, everything else is fatal
                if err != Win32Error::from(ERROR_MORE_DATA) {
                    tracing::error!("Failed to retrieve buffer length for DNS hostname: {}", err);
                    return Err(err);
                }
            }

            // Use a Vec as a memory buffer for a PWSTR
            // We also directly reserve 6 bytes more so we later can append .local without
            // reallocation
            let mut buffer = Vec::with_capacity((buffer_size as usize) + 6);

            if !GetComputerNameExW(
                ComputerNameDnsHostname,
                PWSTR::from_raw(buffer.as_mut_ptr()),
                &mut buffer_size,
            )
            .as_bool()
            {
                let err = Win32Error::from_win32();
                tracing::error!("Failed to retrieve DNS hostname: {}", err);
                return Err(err);
            }

            // Set the real length, this will cut the null terminator
            buffer.set_len(buffer_size as usize);

            buffer
        };

        Ok(Self { dns_host_name })
    }

    /// Attempts to convert the DNS host name to a UTF8 Rust string.
    pub fn dns_host_name_to_string(&self) -> Result<String, FromUtf16Error> {
        String::from_utf16(&self.dns_host_name)
    }

    /// Deconstructs this name into a UTF16 string that contains the DNS host name.
    pub fn into_dns_host_name(self) -> Vec<u16> {
        self.dns_host_name
    }
}
