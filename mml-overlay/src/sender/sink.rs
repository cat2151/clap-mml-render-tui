//! [`super::voice::Voice`] がサーバーへ出す操作。
//!
//! 実体は realtime play server の supervisor 1 つしかない。それでも trait を切って
//! あるのは、**「どの操作でサーバーへ何を送ったか」をテストで数えるため**。
//! 鳴りっぱなしの正体は「サーバーへ 1 つもコマンドが飛ばない経路」だったので、
//! 送ったコマンド列そのものを見ないと再発を止められない。
//!
//! instance はここで [`super::MML_OVERLAY_INSTANCE`] に固定する。上位に instance を
//! 選ばせると「止めた instance と鳴らした instance が違う」が起こり得る。

use cmrt_realtime_play::{
    LiveTimelineConfig, RealtimePlayServerSupervisor, TimelineMidiEvent, MAX_MIDI_MESSAGES,
};

use super::MML_OVERLAY_INSTANCE;

/// 失敗の中身は log へ出すだけなので文字列で十分。
pub(super) type SinkResult = Result<(), String>;

pub(super) trait SoundSink {
    fn prepare_patch(&self, patch: Option<&str>) -> SinkResult;
    /// 生 MIDI を offset なしで即座に送る。
    fn send_midi(&self, messages: &[[u8; 3]]) -> SinkResult;
    /// 音源そのものをリセットして黙らせる。何が鳴っていたかを知らなくてよい。
    fn stop_all(&self) -> SinkResult;
    fn begin_timeline(&self, config: LiveTimelineConfig) -> SinkResult;
    fn send_timeline_events(&self, events: &[TimelineMidiEvent]) -> SinkResult;
    /// 1 バッチに載せられるイベント数。超えるとサーバーがバッチごと弾く。
    fn max_batch_events(&self) -> usize {
        MAX_MIDI_MESSAGES
    }
}

impl SoundSink for RealtimePlayServerSupervisor {
    fn prepare_patch(&self, patch: Option<&str>) -> SinkResult {
        self.prepare_live_patch(MML_OVERLAY_INSTANCE, patch)
            .map_err(|error| format!("{error:#}"))
    }

    fn send_midi(&self, messages: &[[u8; 3]]) -> SinkResult {
        RealtimePlayServerSupervisor::send_midi(self, MML_OVERLAY_INSTANCE, messages)
            .map(|_| ())
            .map_err(|error| format!("{error:#}"))
    }

    fn stop_all(&self) -> SinkResult {
        self.stop_live_all().map_err(|error| format!("{error:#}"))
    }

    fn begin_timeline(&self, config: LiveTimelineConfig) -> SinkResult {
        self.begin_live_timeline(config)
            .map_err(|error| format!("{error:#}"))
    }

    fn send_timeline_events(&self, events: &[TimelineMidiEvent]) -> SinkResult {
        RealtimePlayServerSupervisor::send_timeline_events(self, events)
            .map(|_| ())
            .map_err(|error| format!("{error:#}"))
    }
}
