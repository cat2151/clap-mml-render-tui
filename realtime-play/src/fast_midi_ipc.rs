#[cfg(windows)]
mod windows {
    use std::{
        cell::UnsafeCell,
        ffi::c_void,
        mem::size_of,
        ptr::{self, NonNull},
        sync::atomic::{AtomicU32, AtomicU64, Ordering},
    };

    use anyhow::{anyhow, Result};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, HANDLE, STILL_ACTIVE},
        System::{
            Memory::{
                MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
                MEMORY_MAPPED_VIEW_ADDRESS,
            },
            SystemInformation::GetTickCount64,
            Threading::{
                GetCurrentProcessId, GetExitCodeProcess, OpenEventW, OpenProcess, SetEvent,
                EVENT_MODIFY_STATE, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    };

    const MAGIC: [u8; 8] = *b"CMRTMIDI";
    /// v3: `CommandSlot` に `offsets` を追加し、`MAX_MIDI_MESSAGES` を 128 へ拡張。
    /// clap-mml-play-server 側の `realtime-ipc/src/windows.rs` と1バイトも違わないこと。
    const VERSION: u32 = 3;
    const SLOT_COUNT: usize = 64;
    pub const MAX_MIDI_MESSAGES: usize = 128;
    const MAX_PATCH_BYTES: usize = 4096;
    const KIND_MIDI: u32 = 1;
    const KIND_STOP: u32 = 2;
    const KIND_SET_BUFFER_MULTIPLIER: u32 = 3;
    const SERVER_STALE_MS: u64 = 1_000;

    #[repr(C)]
    struct CommandSlot {
        kind: u32,
        message_count: u32,
        patch_len: u32,
        has_patch: u32,
        buffer_multiplier: u32,
        messages: [[u8; 3]; MAX_MIDI_MESSAGES],
        offsets: [u32; MAX_MIDI_MESSAGES],
        patch: [u8; MAX_PATCH_BYTES],
    }

    #[repr(C, align(64))]
    struct SharedRing {
        magic: [u8; 8],
        version: u32,
        _reserved: u32,
        server_pid: AtomicU32,
        client_pid: AtomicU32,
        write_index: AtomicU32,
        read_index: AtomicU32,
        heartbeat_ms: AtomicU64,
        slots: [UnsafeCell<CommandSlot>; SLOT_COUNT],
    }

    unsafe impl Sync for SharedRing {}

    // 共有メモリのレイアウトは2 repo に手書きで二重定義されている。片方だけ変えると
    // 実行時に ProtocolMismatch で落ちるため、サイズをここで固定してコンパイル時に検出する。
    // 変更するときは clap-mml-play-server 側の `realtime-ipc/src/windows.rs` も同じ値へ。
    const _: () = assert!(size_of::<CommandSlot>() == 5012);
    const _: () = assert!(size_of::<SharedRing>() == 320_832);

    struct Mapping {
        handle: HANDLE,
        view: NonNull<SharedRing>,
    }

    unsafe impl Send for Mapping {}

    impl Mapping {
        fn ring(&self) -> &SharedRing {
            // SAFETY: the mapping is valid while self is alive. The protocol synchronizes access.
            unsafe { self.view.as_ref() }
        }
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            // SAFETY: this object owns both resources.
            unsafe {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view.as_ptr().cast::<c_void>(),
                });
                CloseHandle(self.handle);
            }
        }
    }

    struct OwnedHandle(HANDLE);

    unsafe impl Send for OwnedHandle {}

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: this object owns the handle.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub struct FastMidiClient {
        mapping: Mapping,
        event: OwnedHandle,
        pid: u32,
    }

    impl FastMidiClient {
        pub fn connect(port: u16) -> Result<Self> {
            let mapping_name = wide_name(port, "map");
            let event_name = wide_name(port, "event");
            let handle = unsafe { OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, mapping_name.as_ptr()) };
            if handle.is_null() {
                return Err(last_os_error("OpenFileMappingW"));
            }
            let mapping = map_handle(handle)?;
            validate_ring(mapping.ring())?;
            let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr()) };
            if event.is_null() {
                return Err(last_os_error("OpenEventW"));
            }
            let event = OwnedHandle(event);
            let pid = unsafe { GetCurrentProcessId() };
            claim_client(mapping.ring(), pid)?;
            let write = mapping.ring().write_index.load(Ordering::Acquire);
            mapping.ring().read_index.store(write, Ordering::Release);
            Ok(Self {
                mapping,
                event,
                pid,
            })
        }

        /// 全メッセージをオフセット 0（次 chunk 先頭）で送る。
        pub fn send_midi(&mut self, messages: &[[u8; 3]], patch: Option<&str>) -> Result<()> {
            let events = messages
                .iter()
                .map(|message| (0, *message))
                .collect::<Vec<_>>();
            self.send_midi_with_offsets(&events, patch)
        }

        /// `(offset_frames, message)` の並びで送る。オフセットはサーバーの現在の live 位置から
        /// のフレーム数で、サーバー側でサンプル精度のスケジュールに載る。
        pub fn send_midi_with_offsets(
            &mut self,
            events: &[(u32, [u8; 3])],
            patch: Option<&str>,
        ) -> Result<()> {
            if events.is_empty() || events.len() > MAX_MIDI_MESSAGES {
                return Err(anyhow!(
                    "MIDI batch must contain 1..={MAX_MIDI_MESSAGES} messages"
                ));
            }
            if let Some((_, message)) = events.iter().find(|(_, message)| {
                !(0x80..=0xef).contains(&message[0]) || message[1] > 0x7f || message[2] > 0x7f
            }) {
                return Err(anyhow!(
                    "invalid MIDI channel voice message: [{}, {}, {}]",
                    message[0],
                    message[1],
                    message[2]
                ));
            }
            let patch_bytes = patch.map(str::as_bytes).unwrap_or_default();
            if patch_bytes.len() > MAX_PATCH_BYTES {
                return Err(anyhow!(
                    "patch path is too long ({} bytes; max {MAX_PATCH_BYTES})",
                    patch_bytes.len()
                ));
            }
            let mut slot = zeroed_slot();
            slot.kind = KIND_MIDI;
            slot.message_count = events.len() as u32;
            for (index, (offset, message)) in events.iter().enumerate() {
                slot.messages[index] = *message;
                slot.offsets[index] = *offset;
            }
            if patch.is_some() {
                slot.has_patch = 1;
                slot.patch_len = patch_bytes.len() as u32;
                slot.patch[..patch_bytes.len()].copy_from_slice(patch_bytes);
            }
            self.push(slot)
        }

        pub fn stop(&mut self) -> Result<()> {
            let mut slot = zeroed_slot();
            slot.kind = KIND_STOP;
            self.push(slot)
        }

        pub fn set_buffer_multiplier(&mut self, multiplier: u8) -> Result<()> {
            if !matches!(multiplier, 1 | 2 | 4 | 8) {
                return Err(anyhow!("buffer multiplier must be 1, 2, 4, or 8"));
            }
            let mut slot = zeroed_slot();
            slot.kind = KIND_SET_BUFFER_MULTIPLIER;
            slot.buffer_multiplier = u32::from(multiplier);
            self.push(slot)
        }

        fn push(&mut self, slot: CommandSlot) -> Result<()> {
            let ring = self.mapping.ring();
            validate_ring(ring)?;
            if ring.client_pid.load(Ordering::Acquire) != self.pid {
                return Err(anyhow!("shared-memory MIDI server restarted"));
            }
            let heartbeat = ring.heartbeat_ms.load(Ordering::Acquire);
            let elapsed = unsafe { GetTickCount64() }.saturating_sub(heartbeat);
            if elapsed > SERVER_STALE_MS {
                return Err(anyhow!("shared-memory MIDI server stopped responding"));
            }
            let write = ring.write_index.load(Ordering::Relaxed);
            let read = ring.read_index.load(Ordering::Acquire);
            if write.wrapping_sub(read) >= SLOT_COUNT as u32 {
                return Err(anyhow!("shared-memory MIDI queue is full"));
            }
            let index = write as usize % SLOT_COUNT;
            // SAFETY: this client is the sole producer. Release publishes the complete slot.
            unsafe { ptr::write(ring.slots[index].get(), slot) };
            ring.write_index
                .store(write.wrapping_add(1), Ordering::Release);
            if unsafe { SetEvent(self.event.0) } == 0 {
                return Err(last_os_error("SetEvent"));
            }
            Ok(())
        }
    }

    impl Drop for FastMidiClient {
        fn drop(&mut self) {
            let _ = self.mapping.ring().client_pid.compare_exchange(
                self.pid,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }

    fn map_handle(handle: HANDLE) -> Result<Mapping> {
        let view =
            unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size_of::<SharedRing>()) };
        let Some(view) = NonNull::new(view.Value.cast::<SharedRing>()) else {
            unsafe { CloseHandle(handle) };
            return Err(last_os_error("MapViewOfFile"));
        };
        Ok(Mapping { handle, view })
    }

    fn validate_ring(ring: &SharedRing) -> Result<()> {
        if ring.magic != MAGIC || ring.version != VERSION {
            return Err(anyhow!("shared-memory MIDI protocol mismatch"));
        }
        Ok(())
    }

    fn claim_client(ring: &SharedRing, pid: u32) -> Result<()> {
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
                return Err(anyhow!("another shared-memory MIDI client is connected"));
            }
            let _ = ring
                .client_pid
                .compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire);
        }
    }

    fn process_is_running(pid: u32) -> bool {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return unsafe { GetLastError() } != ERROR_INVALID_PARAMETER;
        }
        let mut exit_code = 0;
        let read_ok = unsafe { GetExitCodeProcess(process, &mut exit_code) } != 0;
        unsafe { CloseHandle(process) };
        read_ok && exit_code == STILL_ACTIVE as u32
    }

    fn zeroed_slot() -> CommandSlot {
        // SAFETY: the type contains only integers and byte arrays.
        unsafe { std::mem::zeroed() }
    }

    fn wide_name(port: u16, suffix: &str) -> Vec<u16> {
        format!("Local\\cmrt-realtime-midi-v{VERSION}-{port}-{suffix}\0")
            .encode_utf16()
            .collect()
    }

    fn last_os_error(operation: &str) -> anyhow::Error {
        anyhow!("{operation} failed with Windows error {}", unsafe {
            GetLastError()
        })
    }
}

#[cfg(windows)]
pub use windows::{FastMidiClient, MAX_MIDI_MESSAGES};

/// 1回の送信に詰められる MIDI メッセージ数の上限（共有メモリのスロット容量）。
/// 呼び出し側がバッチを切る目安に使う。
#[cfg(not(windows))]
pub const MAX_MIDI_MESSAGES: usize = 128;

#[cfg(not(windows))]
pub struct FastMidiClient;

#[cfg(not(windows))]
impl FastMidiClient {
    pub fn connect(_port: u16) -> anyhow::Result<Self> {
        anyhow::bail!("shared-memory MIDI is only supported on Windows")
    }

    pub fn send_midi(&mut self, _messages: &[[u8; 3]], _patch: Option<&str>) -> anyhow::Result<()> {
        anyhow::bail!("shared-memory MIDI is only supported on Windows")
    }

    pub fn send_midi_with_offsets(
        &mut self,
        _events: &[(u32, [u8; 3])],
        _patch: Option<&str>,
    ) -> anyhow::Result<()> {
        anyhow::bail!("shared-memory MIDI is only supported on Windows")
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        anyhow::bail!("shared-memory MIDI is only supported on Windows")
    }

    pub fn set_buffer_multiplier(&mut self, _multiplier: u8) -> anyhow::Result<()> {
        anyhow::bail!("shared-memory MIDI is only supported on Windows")
    }
}
