use std::fmt;

pub const INSTANCE_COUNT: usize = 16;
pub const MAX_MIDI_MESSAGES: usize = 128;
pub const MAX_PATCH_BYTES: usize = 4096;
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024;
pub type InstanceId = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FastMidiEvent {
    pub instance_id: InstanceId,
    pub offset_frames: u32,
    pub message: [u8; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LimiterMeter {
    pub current_reduction_db: f32,
    pub peak_reduction_db: f32,
}

#[derive(Debug)]
pub enum FastIpcError {
    UnsupportedPlatform,
    NotAvailable,
    AlreadyConnected,
    ProtocolMismatch,
    ServerStopped,
    QueueFull,
    ResponseTimeout,
    RequestFailed(String),
    InvalidPayload(String),
    Os { operation: &'static str, code: u32 },
}

impl fmt::Display for FastIpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "shared-memory MIDI is only supported on Windows")
            }
            Self::NotAvailable => write!(f, "shared-memory MIDI server is not available"),
            Self::AlreadyConnected => write!(f, "another shared-memory MIDI client is connected"),
            Self::ProtocolMismatch => write!(f, "shared-memory MIDI protocol mismatch"),
            Self::ServerStopped => write!(f, "shared-memory MIDI server stopped responding"),
            Self::QueueFull => write!(f, "shared-memory MIDI queue is full"),
            Self::ResponseTimeout => write!(f, "shared-memory MIDI response timed out"),
            Self::RequestFailed(message) => write!(f, "shared-memory request failed: {message}"),
            Self::InvalidPayload(message) => write!(f, "invalid shared-memory payload: {message}"),
            Self::Os { operation, code } => {
                write!(f, "{operation} failed with Windows error {code}")
            }
        }
    }
}

impl std::error::Error for FastIpcError {}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::FastMidiClient;

#[cfg(not(windows))]
pub struct FastMidiClient;

#[cfg(not(windows))]
impl FastMidiClient {
    pub fn connect(_port: u16) -> Result<Self, FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn send_events(&mut self, _events: &[FastMidiEvent]) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn prepare_patch(
        &mut self,
        _instance_id: InstanceId,
        _patch: Option<&str>,
    ) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn probe_patch(
        &mut self,
        _instance_id: InstanceId,
        _patch: Option<&str>,
    ) -> Result<Vec<u8>, FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn stop(&mut self, _instance_id: InstanceId) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn stop_all(&mut self) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn set_buffer_multiplier(&mut self, _multiplier: u8) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn limiter_meter(&self) -> LimiterMeter {
        LimiterMeter::default()
    }

    pub fn underrun_frames(&self) -> u64 {
        0
    }
}
