//! 共有メモリへの接続と、クライアント枠の確保。
//!
//! 「どのコマンドをどう積むか」（[`super`]）とは別の関心事。map / unmap、
//! イベントの open、**同時に 1 クライアントだけ**という所有権の取り合いをここへ集める。

use std::{ffi::c_void, mem::size_of, ptr::NonNull};

use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, HANDLE, STILL_ACTIVE},
    System::{
        Memory::{MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS},
        Threading::{
            GetExitCodeProcess, OpenEventW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

use std::sync::atomic::Ordering;

use super::{last_os_error, FastIpcError, OwnedHandle, SharedRing, MAGIC, VERSION};

pub(super) struct Mapping {
    handle: HANDLE,
    view: NonNull<SharedRing>,
}

unsafe impl Send for Mapping {}
// SharedRing の並行アクセス対象は atomic と、protocol で同期された response だけ。
// 読み取り専用 meter handle は command の応答待ちと並行して使う。
unsafe impl Sync for Mapping {}

impl Mapping {
    pub(super) fn ring(&self) -> &SharedRing {
        unsafe { self.view.as_ref() }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view.as_ptr().cast::<c_void>(),
            });
            CloseHandle(self.handle);
        }
    }
}

pub(super) fn validate_ring(ring: &SharedRing) -> Result<(), FastIpcError> {
    if ring.magic != MAGIC || ring.version != VERSION {
        return Err(FastIpcError::ProtocolMismatch);
    }
    Ok(())
}

pub(super) fn claim_client(ring: &SharedRing, pid: u32) -> Result<(), FastIpcError> {
    loop {
        let owner = ring.client_pid.load(Ordering::Acquire);
        if owner == 0 {
            if ring
                .client_pid
                .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(());
            }
            continue;
        }
        if process_is_running(owner) {
            return Err(FastIpcError::AlreadyConnected);
        }
        let _ = ring
            .client_pid
            .compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire);
    }
}

pub(super) fn process_is_running(pid: u32) -> bool {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return unsafe { GetLastError() } != ERROR_INVALID_PARAMETER;
    }
    let mut exit_code = 0;
    let read_ok = unsafe { GetExitCodeProcess(process, &mut exit_code) } != 0;
    unsafe { CloseHandle(process) };
    read_ok && exit_code == STILL_ACTIVE as u32
}

pub(super) fn map_handle(handle: HANDLE) -> Result<Mapping, FastIpcError> {
    let view = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size_of::<SharedRing>()) };
    let Some(view) = NonNull::new(view.Value.cast::<SharedRing>()) else {
        unsafe { CloseHandle(handle) };
        return Err(last_os_error("MapViewOfFile"));
    };
    Ok(Mapping { handle, view })
}

pub(super) fn open_event(name: &[u16], access: u32) -> Result<OwnedHandle, FastIpcError> {
    let handle = unsafe { OpenEventW(access, 0, name.as_ptr()) };
    if handle.is_null() {
        Err(FastIpcError::NotAvailable)
    } else {
        Ok(OwnedHandle::new(handle))
    }
}
