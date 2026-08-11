use std::{
    ffi::c_void,
    mem::size_of,
    ptr::{self, NonNull},
    sync::atomic::Ordering,
    time::Instant,
};

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, HANDLE, STILL_ACTIVE, WAIT_OBJECT_0,
        WAIT_TIMEOUT,
    },
    System::{
        Memory::{
            MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
            MEMORY_MAPPED_VIEW_ADDRESS,
        },
        SystemInformation::GetTickCount64,
        Threading::{
            GetCurrentProcessId, GetExitCodeProcess, OpenEventW, OpenProcess, SetEvent,
            WaitForSingleObject, EVENT_MODIFY_STATE, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

use super::*;

mod platform;
mod protocol;
mod timeline;

use platform::{last_os_error, wide_name};
use protocol::*;

struct Mapping {
    handle: HANDLE,
    view: NonNull<SharedRing>,
}

unsafe impl Send for Mapping {}

impl Mapping {
    fn ring(&self) -> &SharedRing {
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

struct OwnedHandle(HANDLE);

unsafe impl Send for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub struct FastMidiClient {
    mapping: Mapping,
    command_event: OwnedHandle,
    response_event: OwnedHandle,
    pid: u32,
    next_request_id: u32,
}

impl FastMidiClient {
    pub fn connect(port: u16) -> Result<Self, FastIpcError> {
        let mapping_name = wide_name(port, "map");
        let command_event_name = wide_name(port, "command-event");
        let response_event_name = wide_name(port, "response-event");
        let mapping_handle =
            unsafe { OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, mapping_name.as_ptr()) };
        if mapping_handle.is_null() {
            return Err(FastIpcError::NotAvailable);
        }
        let mapping = map_handle(mapping_handle)?;
        validate_ring(mapping.ring())?;
        let command_event = open_event(&command_event_name, EVENT_MODIFY_STATE)?;
        let response_event = open_event(&response_event_name, SYNCHRONIZE_ACCESS)?;
        let pid = unsafe { GetCurrentProcessId() };
        claim_client(mapping.ring(), pid)?;
        let write = mapping.ring().write_index.load(Ordering::Acquire);
        mapping.ring().read_index.store(write, Ordering::Release);
        Ok(Self {
            mapping,
            command_event,
            response_event,
            pid,
            next_request_id: 1,
        })
    }

    pub fn send_events(&mut self, events: &[FastMidiEvent]) -> Result<(), FastIpcError> {
        validate_events(events)?;
        let mut slot = zeroed_slot();
        slot.kind = KIND_MIDI;
        slot.message_count = events.len() as u32;
        for (index, event) in events.iter().enumerate() {
            slot.messages[index] = event.message;
            slot.offsets[index] = event.offset_frames;
            slot.instance_ids[index] = event.instance_id;
        }
        self.push(slot)
    }

    pub fn prepare_patch(
        &mut self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<(), FastIpcError> {
        self.patch_request(KIND_PREPARE_PATCH, instance_id, patch)
            .map(|_| ())
    }

    pub fn probe_patch(
        &mut self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<Vec<u8>, FastIpcError> {
        self.patch_request(KIND_PROBE_PATCH, instance_id, patch)
    }

    pub fn stop(&mut self, instance_id: InstanceId) -> Result<(), FastIpcError> {
        validate_instance(instance_id)?;
        let mut slot = zeroed_slot();
        slot.kind = KIND_STOP;
        slot.instance_id = u32::from(instance_id);
        self.push(slot)
    }

    pub fn stop_all(&mut self) -> Result<(), FastIpcError> {
        let mut slot = zeroed_slot();
        slot.kind = KIND_STOP_ALL;
        self.push(slot)
    }

    pub fn set_buffer_multiplier(&mut self, multiplier: u16) -> Result<(), FastIpcError> {
        if !crate::is_valid_buffer_multiplier(multiplier) {
            return Err(FastIpcError::InvalidPayload(format!(
                "buffer multiplier must be a power of two up to {}",
                crate::MAX_LIVE_BUFFER_MULTIPLIER
            )));
        }
        let mut slot = zeroed_slot();
        slot.kind = KIND_SET_BUFFER_MULTIPLIER;
        slot.buffer_multiplier = u32::from(multiplier);
        self.push(slot)
    }

    /// live mix で instance へ掛ける振幅ゲインを設定する（1.0 が等倍）。
    pub fn set_instance_gain(
        &mut self,
        instance_id: InstanceId,
        gain: f32,
    ) -> Result<(), FastIpcError> {
        validate_instance(instance_id)?;
        if !gain.is_finite() || !(0.0..=MAX_INSTANCE_GAIN).contains(&gain) {
            return Err(FastIpcError::InvalidPayload(format!(
                "instance gain must be between 0.0 and {MAX_INSTANCE_GAIN} (got {gain})"
            )));
        }
        let mut slot = zeroed_slot();
        slot.kind = KIND_SET_INSTANCE_GAIN;
        slot.instance_id = u32::from(instance_id);
        slot.buffer_multiplier = (gain * 1000.0).round() as u32;
        self.push(slot)
    }

    pub fn set_auto_gain_enabled(&mut self, enabled: bool) -> Result<(), FastIpcError> {
        let mut slot = zeroed_slot();
        slot.kind = KIND_SET_AUTO_GAIN;
        slot.buffer_multiplier = u32::from(enabled);
        self.push(slot)
    }

    pub fn limiter_meter(&self) -> LimiterMeter {
        let ring = self.mapping.ring();
        LimiterMeter {
            current_reduction_db: f32::from_bits(ring.limiter_current_bits.load(Ordering::Acquire)),
            peak_reduction_db: f32::from_bits(ring.limiter_peak_bits.swap(0, Ordering::AcqRel)),
        }
    }

    /// instance ごとの auto-trim ゲイン（dB）。サーバーが公開している最新値。
    pub fn auto_gain_db(&self) -> [f32; INSTANCE_COUNT] {
        let ring = self.mapping.ring();
        let mut gains = [0.0; INSTANCE_COUNT];
        for (gain, slot) in gains.iter_mut().zip(ring.auto_gain_db_bits.iter()) {
            *gain = f32::from_bits(slot.load(Ordering::Acquire));
        }
        gains
    }

    pub fn underrun_frames(&self) -> u64 {
        self.mapping.ring().underrun_frames.load(Ordering::Acquire)
    }

    pub fn timing_metrics(&self) -> TimingMetrics {
        let ring = self.mapping.ring();
        loop {
            let before = ring.timing_sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }
            let metrics = TimingMetrics {
                events: ring.timing_events.load(Ordering::Relaxed),
                late_events: ring.timing_late_events.load(Ordering::Relaxed),
                late_events_total: ring.timing_late_events_total.load(Ordering::Relaxed),
                max_late_samples: ring.timing_max_late_samples.load(Ordering::Relaxed),
                max_late_us: f64::from_bits(ring.timing_max_late_us_bits.load(Ordering::Relaxed)),
                output_lead_min_frames: ring.timing_output_lead_min_frames.load(Ordering::Relaxed),
                output_lead_max_frames: ring.timing_output_lead_max_frames.load(Ordering::Relaxed),
                process_load_p95: f32::from_bits(
                    ring.timing_process_load_p95_bits.load(Ordering::Relaxed),
                ),
                process_load_max: f32::from_bits(
                    ring.timing_process_load_max_bits.load(Ordering::Relaxed),
                ),
            };
            if ring.timing_sequence.load(Ordering::Acquire) == before {
                return metrics;
            }
        }
    }

    fn patch_request(
        &mut self,
        kind: u32,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<Vec<u8>, FastIpcError> {
        validate_instance(instance_id)?;
        let patch_bytes = patch.map(str::as_bytes).unwrap_or_default();
        if patch_bytes.len() > MAX_PATCH_BYTES {
            return Err(FastIpcError::InvalidPayload(format!(
                "patch path is too long ({} bytes; max {MAX_PATCH_BYTES})",
                patch_bytes.len()
            )));
        }
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        let mut slot = zeroed_slot();
        slot.kind = kind;
        slot.request_id = request_id;
        slot.instance_id = u32::from(instance_id);
        if patch.is_some() {
            slot.has_patch = 1;
            slot.patch_len = patch_bytes.len() as u32;
            slot.patch[..patch_bytes.len()].copy_from_slice(patch_bytes);
        }
        self.push(slot)?;
        self.wait_for_response(request_id)
    }

    fn wait_for_response(&self, request_id: u32) -> Result<Vec<u8>, FastIpcError> {
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        loop {
            if let Some(result) = self.read_response(request_id)? {
                return result;
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(FastIpcError::ResponseTimeout);
            }
            let wait_ms = (deadline - now).as_millis().min(u32::MAX as u128) as u32;
            match unsafe { WaitForSingleObject(self.response_event.0, wait_ms) } {
                WAIT_OBJECT_0 => {}
                WAIT_TIMEOUT => return Err(FastIpcError::ResponseTimeout),
                _ => return Err(last_os_error("WaitForSingleObject")),
            }
        }
    }

    fn read_response(
        &self,
        request_id: u32,
    ) -> Result<Option<Result<Vec<u8>, FastIpcError>>, FastIpcError> {
        let ring = self.mapping.ring();
        if ring.response_sequence.load(Ordering::Acquire) == 0 {
            return Ok(None);
        }
        let response = unsafe { &*ring.response.get() };
        if response.request_id != request_id {
            return Ok(None);
        }
        let len = response.payload_len as usize;
        if len > MAX_RESPONSE_BYTES {
            return Err(FastIpcError::InvalidPayload(
                "response payload length is invalid".into(),
            ));
        }
        let payload = response.payload[..len].to_vec();
        Ok(Some(match response.status {
            RESPONSE_OK => Ok(payload),
            RESPONSE_ERROR => Err(FastIpcError::RequestFailed(
                String::from_utf8_lossy(&payload).into_owned(),
            )),
            _ => {
                return Err(FastIpcError::InvalidPayload(
                    "response status is invalid".into(),
                ))
            }
        }))
    }

    fn push(&mut self, slot: CommandSlot) -> Result<(), FastIpcError> {
        let ring = self.mapping.ring();
        validate_ring(ring)?;
        if ring.client_pid.load(Ordering::Acquire) != self.pid {
            return Err(FastIpcError::ServerStopped);
        }
        let elapsed =
            unsafe { GetTickCount64() }.saturating_sub(ring.heartbeat_ms.load(Ordering::Acquire));
        if elapsed > SERVER_STALE_MS {
            return Err(FastIpcError::ServerStopped);
        }
        let write = ring.write_index.load(Ordering::Relaxed);
        let read = ring.read_index.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= SLOT_COUNT as u32 {
            return Err(FastIpcError::QueueFull);
        }
        let index = write as usize % SLOT_COUNT;
        unsafe { ptr::write(ring.slots[index].get(), slot) };
        ring.write_index
            .store(write.wrapping_add(1), Ordering::Release);
        if unsafe { SetEvent(self.command_event.0) } == 0 {
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

fn validate_events(events: &[FastMidiEvent]) -> Result<(), FastIpcError> {
    if events.is_empty() || events.len() > MAX_MIDI_MESSAGES {
        return Err(FastIpcError::InvalidPayload(format!(
            "MIDI batch must contain 1..={MAX_MIDI_MESSAGES} events"
        )));
    }
    for event in events {
        validate_instance(event.instance_id)?;
        let message = event.message;
        if !(0x80..=0xef).contains(&message[0]) || message[1] > 0x7f || message[2] > 0x7f {
            return Err(FastIpcError::InvalidPayload(format!(
                "invalid MIDI channel voice message: [{}, {}, {}]",
                message[0], message[1], message[2]
            )));
        }
    }
    Ok(())
}

fn validate_instance(instance_id: InstanceId) -> Result<(), FastIpcError> {
    if usize::from(instance_id) >= INSTANCE_COUNT {
        return Err(FastIpcError::InvalidPayload(format!(
            "instance {instance_id} is outside 0..{INSTANCE_COUNT}"
        )));
    }
    Ok(())
}

fn zeroed_slot() -> CommandSlot {
    unsafe { std::mem::zeroed() }
}

fn validate_ring(ring: &SharedRing) -> Result<(), FastIpcError> {
    if ring.magic != MAGIC || ring.version != VERSION {
        return Err(FastIpcError::ProtocolMismatch);
    }
    Ok(())
}

fn claim_client(ring: &SharedRing, pid: u32) -> Result<(), FastIpcError> {
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

fn map_handle(handle: HANDLE) -> Result<Mapping, FastIpcError> {
    let view = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size_of::<SharedRing>()) };
    let Some(view) = NonNull::new(view.Value.cast::<SharedRing>()) else {
        unsafe { CloseHandle(handle) };
        return Err(last_os_error("MapViewOfFile"));
    };
    Ok(Mapping { handle, view })
}

fn open_event(name: &[u16], access: u32) -> Result<OwnedHandle, FastIpcError> {
    let handle = unsafe { OpenEventW(access, 0, name.as_ptr()) };
    if handle.is_null() {
        Err(FastIpcError::NotAvailable)
    } else {
        Ok(OwnedHandle(handle))
    }
}
