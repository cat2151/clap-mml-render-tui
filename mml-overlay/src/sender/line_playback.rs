//! 1 行ぶんのフレーズを live timeline へ積む。
//!
//! **行を演奏するたびに timeline を張り直す。** サーバーの `BeginLiveTimeline` は
//! 全 renderer のリセットとスケジュール済みイベントの破棄を伴うので、これ 1 つで
//! 「前の行を止める」と「新しい行を頭から鳴らす」が同時に片づく。patch は
//! renderer が保持したままなので読み込み直しは起きない。
//!
//! 原点が 0 に戻るぶん、積む秒は「行の先頭からの秒 + [`LOOKAHEAD_SECONDS`]」で済み、
//! 経過時間を追いかけずに済む。
//!
//! 止めるほうは `stop_live_all` を使う。`stop_live_instance` でも音は止まるが、
//! 全 instance が非アクティブになるとサーバーが live モード自体を畳んで timeline を
//! 落とすため、「止める」と「timeline を保つ」を両立できない。

use cmrt_chord::TimedMidiEvent;
use cmrt_realtime_play::{
    LiveTimelineConfig, RealtimePlayServerSupervisor, TimelineId, TimelineMidiEvent,
    MAX_MIDI_MESSAGES,
};

use super::{log_error, MML_OVERLAY_INSTANCE};

/// 行の演奏を積むとき、先頭の音を何秒先へ置くか。
///
/// timeline を張ってからサーバーがコマンドを取り込んで出力リングへ書くまでの遅れを
/// 吸収する。長くすると「キーを押してから鳴るまで」がそのまま遅れるので、出力バッファ
/// （既定 512 フレーム × 2 = 21ms）を少し上回るところに置く。
const LOOKAHEAD_SECONDS: f64 = 0.05;

/// timeline の tempo map の初期値。イベントは絶対秒で積むので演奏の速さには効かない。
const TIMELINE_TEMPO_BPM: f64 = 120.0;

/// 1 行として受け付けるイベント数の上限。
///
/// サーバー側のキューは 8192 で、あふれた分は黙って捨てられる。捨てられた note off が
/// 混ざると音が鳴りっぱなしになるため、送る前にこちらで切る。
const MAX_LINE_EVENTS: usize = 4096;

pub(super) struct LinePlayback {
    sample_rate_hz: f64,
    /// 次に張る timeline の id。サーバーは 0 を無効値として弾くので 1 から始める。
    next_timeline_id: TimelineId,
    /// 積んだ演奏がまだ残っているか。無駄な停止コマンドを送らないために持つ。
    active: bool,
}

impl LinePlayback {
    pub(super) fn new(sample_rate_hz: f64) -> Self {
        Self {
            sample_rate_hz,
            next_timeline_id: 1,
            active: false,
        }
    }

    pub(super) fn play(
        &mut self,
        supervisor: &RealtimePlayServerSupervisor,
        events: &[TimedMidiEvent],
    ) {
        if events.is_empty() {
            self.stop(supervisor);
            return;
        }
        let timeline_id = self.next_timeline_id;
        self.next_timeline_id = advance_timeline_id(timeline_id);
        if let Err(error) = supervisor.begin_live_timeline(LiveTimelineConfig {
            timeline_id,
            sample_rate_hz: self.sample_rate_hz,
            tempo_bpm: TIMELINE_TEMPO_BPM,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
        }) {
            log_error(format!(
                "action=mml-overlay-line-begin event=error timeline={timeline_id} error=\"{error:#}\""
            ));
            self.active = false;
            return;
        }
        // timeline を張った時点で前の演奏は消えている。ここから先が失敗しても
        // 音が残ることはないので、積めたところまでで打ち切ってよい。
        self.active = true;

        if events.len() > MAX_LINE_EVENTS {
            log_error(format!(
                "action=mml-overlay-line-send event=truncated total={} kept={MAX_LINE_EVENTS}",
                events.len()
            ));
        }
        for batch in timeline_batches(events, timeline_id) {
            if let Err(error) = supervisor.send_timeline_events(&batch) {
                log_error(format!(
                    "action=mml-overlay-line-send event=error timeline={timeline_id} error=\"{error:#}\""
                ));
                return;
            }
        }
    }

    #[cfg(test)]
    pub(super) fn is_active(&self) -> bool {
        self.active
    }

    pub(super) fn stop(&mut self, supervisor: &RealtimePlayServerSupervisor) {
        if !self.active {
            return;
        }
        self.active = false;
        if let Err(error) = supervisor.stop_live_all() {
            log_error(format!(
                "action=mml-overlay-line-stop event=error error=\"{error:#}\""
            ));
        }
    }
}

/// 次の timeline id。サーバーは 0 を無効値として弾くので、一周しても 0 は飛ばす。
fn advance_timeline_id(current: TimelineId) -> TimelineId {
    current.wrapping_add(1).max(1)
}

/// 行のイベント列を、共有メモリ 1 回ぶんずつのバッチへ切り分ける。
///
/// 1 バッチに載る数はサーバー側の受け口（[`MAX_MIDI_MESSAGES`]）で決まっていて、
/// 超えるとバッチ全体が弾かれる。
fn timeline_batches(
    events: &[TimedMidiEvent],
    timeline_id: TimelineId,
) -> Vec<Vec<TimelineMidiEvent>> {
    let count = events.len().min(MAX_LINE_EVENTS);
    events[..count]
        .chunks(MAX_MIDI_MESSAGES)
        .map(|batch| {
            batch
                .iter()
                .map(|event| TimelineMidiEvent {
                    timeline_id,
                    instance_id: MML_OVERLAY_INSTANCE,
                    timeline_seconds: LOOKAHEAD_SECONDS + event.seconds.max(0.0),
                    message: event.message,
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests;
