use windows::core::{Error as Win32Error, HRESULT};
use windows::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, ERROR_SUCCESS, SEVERITY_ERROR, WIN32_ERROR,
};
use windows::Win32::System::Diagnostics::Debug::FACILITY_WIN32;

pub trait ToWin32ErrorCode {
    /// Converts self to a Win32 error code
    fn to_win32_error_code(&self) -> WIN32_ERROR;
}

const WIN32_HRESULT_MASK: u32 = (SEVERITY_ERROR << 31) | (FACILITY_WIN32.0 << 16);

impl ToWin32ErrorCode for HRESULT {
    fn to_win32_error_code(&self) -> WIN32_ERROR {
        let h = self.0 as u32;

        if (h & 0xFFFF0000) == WIN32_HRESULT_MASK {
            WIN32_ERROR(h & 0xFFFF)
        } else if self.is_ok() {
            ERROR_SUCCESS
        } else {
            // This is not correct, but the best we can do
            ERROR_INVALID_PARAMETER
        }
    }
}

impl ToWin32ErrorCode for Win32Error {
    fn to_win32_error_code(&self) -> WIN32_ERROR {
        self.code().to_win32_error_code()
    }
}
