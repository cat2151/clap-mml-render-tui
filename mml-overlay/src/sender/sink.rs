//! [`super::voice::Voice`] がサーバーへ出す操作。
//!
//! 実体は realtime play server の supervisor 1 つしかない。それでも trait を切って
//! あるのは、**「どの操作でサーバーへ何を送ったか」をテストで数えるため**。
//! 鳴りっぱなしの正体は「サーバーへ 1 つもコマンドが飛ばない経路」だったので、
//! 送ったコマンド列そのものを見ないと再発を止められない。
//!
//! 通常の MML overlay は呼び出し側が [`super::MML_OVERLAY_INSTANCE`] を渡す。
//! Chord Chart の layered preview だけは chord と bass を別 instance に載せるため、
//! patch と MIDI の操作は instance を明示して受け取る。

use cmrt_realtime_play::{
    LiveTimelineConfig, RealtimePlayServerSupervisor, TimelineMidiEvent, BANK_COUNT,
    MAX_MIDI_MESSAGES,
};

use super::live_patch::LivePatch;

/// 失敗の中身は log へ出すだけなので文字列で十分。
pub(super) type SinkResult = Result<(), String>;

pub(super) trait SoundSink {
    fn prepare_patch(&self, instance_id: u8, patch: &LivePatch) -> SinkResult;
    /// `instance_id` と同じ位置にある、もう一方の bank の instance。bank が 1 つなら `None`。
    ///
    /// 音色を変える行はこちらへ読み込んで鳴らす。読み込みの間も、鳴っている bank は
    /// 前の行の release を描き続けられる。
    fn standby_instance_of(&self, _instance_id: u8) -> Option<u8> {
        None
    }
    /// 鳴っていない bank の instance へ音色を読み込む。鳴っている bank の render は止めない。
    fn prepare_standby_patch(&self, instance_id: u8, _patch: &LivePatch) -> SinkResult {
        Err(format!("instance {instance_id} has no standby bank"))
    }
    /// 生 MIDI を offset なしで即座に送る。
    fn send_midi(&self, instance_id: u8, messages: &[[u8; 3]]) -> SinkResult;
    /// serverがprocess済みnoteをすべてNoteOffする。呼び出し側はnoteを知らなくてよい。
    fn stop_all(&self) -> SinkResult;
    /// `instance_ids` の出力を `fade_ms` で 0 まで絞る。応答を待たない。
    fn fade_out_instances(&self, _instance_ids: &[u8], _fade_ms: u32) -> SinkResult {
        Err("fadeout is not supported".to_string())
    }
    fn begin_timeline(&self, config: LiveTimelineConfig) -> SinkResult;
    fn send_timeline_events(&self, events: &[TimelineMidiEvent]) -> SinkResult;
    /// 1 バッチに載せられるイベント数。超えるとサーバーがバッチごと弾く。
    fn max_batch_events(&self) -> usize {
        MAX_MIDI_MESSAGES
    }
}

impl SoundSink for RealtimePlayServerSupervisor {
    fn prepare_patch(&self, instance_id: u8, patch: &LivePatch) -> SinkResult {
        self.prepare_live_patch_with_effect_chain(instance_id, patch.patch(), patch.effect_chain())
            .map_err(|error| format!("{error:#}"))
    }

    fn standby_instance_of(&self, instance_id: u8) -> Option<u8> {
        let per_bank = self.live_instance_count() / BANK_COUNT;
        if per_bank == 0 {
            return None;
        }
        let index = usize::from(instance_id);
        let other_bank = (index / per_bank + 1) % BANK_COUNT;
        u8::try_from(other_bank * per_bank + index % per_bank).ok()
    }

    fn prepare_standby_patch(&self, instance_id: u8, patch: &LivePatch) -> SinkResult {
        self.prepare_standby_patch_with_effect_chain(
            instance_id,
            patch.patch(),
            patch.effect_chain(),
        )
        .map_err(|error| format!("{error:#}"))
    }

    fn send_midi(&self, instance_id: u8, messages: &[[u8; 3]]) -> SinkResult {
        RealtimePlayServerSupervisor::send_midi(self, instance_id, messages)
            .map(|_| ())
            .map_err(|error| format!("{error:#}"))
    }

    fn stop_all(&self) -> SinkResult {
        self.stop_live_all().map_err(|error| format!("{error:#}"))
    }

    fn fade_out_instances(&self, instance_ids: &[u8], fade_ms: u32) -> SinkResult {
        self.fade_out_live_instances(instance_ids, fade_ms)
            .map_err(|error| format!("{error:#}"))
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
