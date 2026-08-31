use std::fmt;

/// 共有メモリプロトコルが表現できる最大 instance 数。
///
/// grid sequencer の chord mode は N トラックを 2 つの bank（= 2N instance）へ割り当て、
/// 鳴っている bank の裏でもう一方へ次の patch を先読みする。16 トラックぶんを
/// ダブルバッファにすると 32 必要なのでここが上限になる。
/// `instance_id` は wire format 上 `u32` / `u8` なので、この定数を上げても
/// 共有メモリのレイアウトは変わらない（`windows/protocol.rs` の `VERSION` は据え置き）。
/// サーバー側の `cmrt_realtime_ipc::MAX_INSTANCE_COUNT` と必ず揃えること。
pub const INSTANCE_COUNT: usize = 32;
pub const MAX_MIDI_MESSAGES: usize = 128;
pub const MAX_PATCH_BYTES: usize = 4096;
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024;
/// standby 完了通知が運べるエラーメッセージの最大バイト数。
///
/// 汎用応答と違い、この slot は共有メモリに常設される固定長領域なので小さく取る。
/// これを超えるメッセージはサーバー側の publish 時に UTF-8 境界で切り詰められる。
/// 完了通知を落とすと先読みが永久に Loading のまま残るので、長すぎることを理由に
/// publish を失敗させない。サーバー側の
/// `cmrt_realtime_ipc::MAX_STANDBY_ERROR_BYTES` と必ず揃えること。
pub const MAX_STANDBY_ERROR_BYTES: usize = 1024;
pub type InstanceId = u8;
pub type TimelineId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FastMidiEvent {
    pub instance_id: InstanceId,
    pub offset_frames: u32,
    pub message: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineMidiEvent {
    pub timeline_id: TimelineId,
    pub instance_id: InstanceId,
    pub timeline_seconds: f64,
    pub message: [u8; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveTimelineConfig {
    pub timeline_id: TimelineId,
    pub sample_rate_hz: f64,
    pub tempo_bpm: f64,
    pub time_signature_numerator: u16,
    pub time_signature_denominator: u16,
}

/// live timeline の tempo map へ積む変化点。
///
/// テンポは timeline の属性ではなく timeline 上のデータなので、これを送っても
/// timeline は作り直されない（`begin_live_timeline` と違い、サーバー側の
/// プラグイン状態もサンプルクロックの原点も動かない）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveTempoChange {
    pub timeline_id: TimelineId,
    /// この絶対秒（timeline 原点から）から新しいテンポにする。
    pub at_seconds: f64,
    pub tempo_bpm: f64,
    pub time_signature_numerator: u16,
    pub time_signature_denominator: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimingMetrics {
    pub events: u64,
    pub late_events: u64,
    pub late_events_total: u64,
    pub max_late_samples: u64,
    pub max_late_us: f64,
    pub output_lead_min_frames: u64,
    pub output_lead_max_frames: u64,
    pub process_load_p95: f32,
    pub process_load_max: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LimiterMeter {
    pub current_reduction_db: f32,
    pub peak_reduction_db: f32,
}

/// サーバー側の `cmrt_realtime_ipc::FastIpcError` と同じ derive にしてある。
/// 完了通知の成否を `Option<Result<(), FastIpcError>>` のまま比較したいため。
#[derive(Clone, Debug, PartialEq, Eq)]
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
pub use windows::{FastMidiClient, FastMidiUnderrunReader};

#[cfg(not(windows))]
pub struct FastMidiClient;

#[cfg(not(windows))]
pub struct FastMidiUnderrunReader;

#[cfg(not(windows))]
impl FastMidiUnderrunReader {
    pub fn underrun_frames(&self) -> u64 {
        0
    }
}

#[cfg(not(windows))]
impl FastMidiClient {
    pub fn connect(_port: u16) -> Result<Self, FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn underrun_reader(&self) -> FastMidiUnderrunReader {
        FastMidiUnderrunReader
    }

    pub fn send_events(&mut self, _events: &[FastMidiEvent]) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn begin_live_timeline(&mut self, _config: LiveTimelineConfig) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn set_live_tempo(&mut self, _change: LiveTempoChange) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn send_timeline_events(
        &mut self,
        _events: &[TimelineMidiEvent],
    ) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn prepare_patch(
        &mut self,
        _instance_id: InstanceId,
        _patch: Option<&str>,
    ) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn begin_standby_patch(
        &mut self,
        _instance_id: InstanceId,
        _patch: Option<&str>,
    ) -> Result<(u32, u64), FastIpcError> {
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

    pub fn set_buffer_multiplier(&mut self, _multiplier: u16) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn set_instance_gain(
        &mut self,
        _instance_id: InstanceId,
        _gain: f32,
    ) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn set_auto_gain_enabled(&mut self, _enabled: bool) -> Result<(), FastIpcError> {
        Err(FastIpcError::UnsupportedPlatform)
    }

    pub fn limiter_meter(&self) -> LimiterMeter {
        LimiterMeter::default()
    }

    pub fn auto_gain_db(&self) -> [f32; INSTANCE_COUNT] {
        [0.0; INSTANCE_COUNT]
    }

    pub fn underrun_frames(&self) -> u64 {
        0
    }

    pub fn timing_metrics(&self) -> TimingMetrics {
        TimingMetrics::default()
    }

    pub fn poll_standby_completion(
        &mut self,
        _request_id: u32,
        _since_sequence: u64,
    ) -> Option<Result<(), FastIpcError>> {
        Some(Err(FastIpcError::UnsupportedPlatform))
    }

    pub fn abandon_standby_patch(&mut self, _request_id: u32) {}

    pub fn standby_in_flight(&self) -> Option<u32> {
        None
    }
}
