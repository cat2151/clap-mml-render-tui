use windows_sys::Win32::Foundation::GetLastError;

use super::*;

pub(super) fn wide_name(port: u16, suffix: &str) -> Vec<u16> {
    format!("Local\\cmrt-realtime-midi-v{VERSION}-{port}-{suffix}\0")
        .encode_utf16()
        .collect()
}

pub(super) fn last_os_error(operation: &'static str) -> FastIpcError {
    FastIpcError::Os {
        operation,
        code: unsafe { GetLastError() },
    }
}
