use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum GridConnectionPhase {
    Idle,
    Connecting,
    WaitingForPatches,
    PatchSetting,
    Ready,
    Error(String),
}

impl GridConnectionPhase {
    pub fn accepts_notes(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridProgress {
    pub completed: usize,
    pub total: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridConnectionStatus {
    pub phase: GridConnectionPhase,
    pub last_send: Option<Duration>,
    pub limiter_reduction_db: f32,
    pub buffer_multiplier: u8,
    pub underrun_frames: u64,
    pub server_startup: Option<GridProgress>,
    pub patch_setting: Option<GridProgress>,
}

impl Default for GridConnectionStatus {
    fn default() -> Self {
        Self {
            phase: GridConnectionPhase::Idle,
            last_send: None,
            limiter_reduction_db: 0.0,
            buffer_multiplier: super::adaptive_buffer::INITIAL_BUFFER_MULTIPLIER,
            underrun_frames: 0,
            server_startup: None,
            patch_setting: None,
        }
    }
}

impl GridConnectionStatus {
    pub fn label(&self) -> String {
        match &self.phase {
            GridConnectionPhase::Idle => "idle".to_string(),
            GridConnectionPhase::Connecting => self
                .server_startup
                .map(|progress| {
                    format!("starting server {}/{}", progress.completed, progress.total)
                })
                .unwrap_or_else(|| "starting server".to_string()),
            GridConnectionPhase::WaitingForPatches => "waiting for patches".to_string(),
            GridConnectionPhase::PatchSetting => self
                .patch_setting
                .map(|progress| format!("patches {}/{}", progress.completed, progress.total))
                .unwrap_or_else(|| "patches".to_string()),
            GridConnectionPhase::Ready => "ready".to_string(),
            GridConnectionPhase::Error(message) => message.clone(),
        }
    }

    pub(super) fn begin_connecting(&mut self) {
        self.phase = GridConnectionPhase::Connecting;
        self.last_send = None;
        self.limiter_reduction_db = 0.0;
        self.buffer_multiplier = super::adaptive_buffer::INITIAL_BUFFER_MULTIPLIER;
        self.underrun_frames = 0;
        self.server_startup = None;
        self.patch_setting = None;
    }

    pub(super) fn update_server_startup(&mut self, completed: usize, total: usize) {
        self.server_startup = Some(GridProgress { completed, total });
    }

    pub(super) fn wait_for_patches(&mut self, elapsed: Duration) {
        self.phase = GridConnectionPhase::WaitingForPatches;
        self.last_send = Some(elapsed);
    }

    pub(super) fn begin_patch_setting(&mut self, total: usize) {
        self.phase = GridConnectionPhase::PatchSetting;
        self.patch_setting = Some(GridProgress {
            completed: 0,
            total,
        });
    }

    pub(super) fn update_patch_setting(&mut self, completed: usize, total: usize) {
        self.patch_setting = Some(GridProgress { completed, total });
    }

    pub(super) fn apply_result(
        &mut self,
        result: anyhow::Result<cmrt_realtime_play::LimiterMeter>,
        elapsed: Option<Duration>,
        idle_on_success: bool,
    ) {
        self.phase = match &result {
            Ok(_) if idle_on_success => GridConnectionPhase::Idle,
            Ok(_) => GridConnectionPhase::Ready,
            Err(error) => GridConnectionPhase::Error(error.to_string()),
        };
        if let Ok(meter) = result {
            self.update_limiter_meter(meter);
        }
        self.last_send = elapsed;
    }

    pub(super) fn update_limiter_meter(&mut self, meter: cmrt_realtime_play::LimiterMeter) {
        self.limiter_reduction_db = meter.peak_reduction_db.max(meter.current_reduction_db);
    }

    pub(super) fn update_adaptive_buffer(&mut self, multiplier: u8, underrun_frames: u64) {
        self.buffer_multiplier = multiplier;
        self.underrun_frames = underrun_frames;
    }
}
