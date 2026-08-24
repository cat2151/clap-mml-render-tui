use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};

use super::*;

pub(super) struct OwnedHandle(HANDLE);

unsafe impl Send for OwnedHandle {}

impl OwnedHandle {
    pub(super) fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    pub(super) fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

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
