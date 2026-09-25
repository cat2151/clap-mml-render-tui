//! server の代わりに、sender が送った内容を記録する [`SoundSink`]。
//!
//! sender の外（画面 crate）のテストが「この操作で LIVE へ何を送ったか」を数えるためのもの。
//! 本番の server と同じく bank は 2 つ（instance 0 と 1 が組）で、音色を変える行は先読みで準備する。

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

use cmrt_realtime_play::{LiveTimelineConfig, TimelineMidiEvent};

use super::live_patch::LivePatch;
use super::sink::{SinkResult, SoundSink};

/// sink が受けた操作（受けた順）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkOperation {
    Prepare,
    Timeline,
    Stop,
    FadeOut { instance_ids: Vec<u8>, fade_ms: u32 },
}

#[derive(Default)]
pub struct RecordingSink {
    prepare_error: Option<String>,
    prepared: Mutex<Vec<LivePatch>>,
    operations: Mutex<Vec<SinkOperation>>,
    timelines: AtomicUsize,
    stops: AtomicUsize,
}

impl RecordingSink {
    /// 音色の準備が必ず `error` で失敗する sink。
    pub fn failing_prepare(error: &str) -> Self {
        Self {
            prepare_error: Some(error.to_string()),
            ..Self::default()
        }
    }

    /// 準備を頼まれた音色と chain（順に）。
    pub fn prepared(&self) -> Vec<LivePatch> {
        self.prepared.lock().unwrap().clone()
    }

    /// 張った live timeline の数。行を 1 回鳴らすと 1 増える。
    pub fn timelines(&self) -> usize {
        self.timelines.load(Ordering::Acquire)
    }

    /// 受けた操作を受けた順に。
    pub fn operations(&self) -> Vec<SinkOperation> {
        self.operations.lock().unwrap().clone()
    }

    /// fadeout を頼まれた回数。
    pub fn fade_outs(&self) -> usize {
        self.operations
            .lock()
            .unwrap()
            .iter()
            .filter(|operation| matches!(operation, SinkOperation::FadeOut { .. }))
            .count()
    }

    fn push(&self, operation: SinkOperation) {
        self.operations.lock().unwrap().push(operation);
    }

    /// 鳴っているものを止めた回数。
    pub fn stops(&self) -> usize {
        self.stops.load(Ordering::Acquire)
    }
}

impl SoundSink for RecordingSink {
    fn prepare_patch(&self, _instance_id: u8, patch: &LivePatch) -> SinkResult {
        self.prepared.lock().unwrap().push(patch.clone());
        self.push(SinkOperation::Prepare);
        match &self.prepare_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn standby_instance_of(&self, instance_id: u8) -> Option<u8> {
        Some(1 - instance_id.min(1))
    }

    fn prepare_standby_patch(&self, instance_id: u8, patch: &LivePatch) -> SinkResult {
        self.prepare_patch(instance_id, patch)
    }

    fn send_midi(&self, _instance_id: u8, _messages: &[[u8; 3]]) -> SinkResult {
        Ok(())
    }

    fn stop_all(&self) -> SinkResult {
        self.stops.fetch_add(1, Ordering::AcqRel);
        self.push(SinkOperation::Stop);
        Ok(())
    }

    fn fade_out_instances(&self, instance_ids: &[u8], fade_ms: u32) -> SinkResult {
        self.push(SinkOperation::FadeOut {
            instance_ids: instance_ids.to_vec(),
            fade_ms,
        });
        Ok(())
    }

    fn begin_timeline(&self, _config: LiveTimelineConfig) -> SinkResult {
        self.timelines.fetch_add(1, Ordering::AcqRel);
        self.push(SinkOperation::Timeline);
        Ok(())
    }

    fn send_timeline_events(&self, _events: &[TimelineMidiEvent]) -> SinkResult {
        Ok(())
    }
}
