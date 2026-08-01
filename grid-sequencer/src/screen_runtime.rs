use std::time::{Duration, Instant};

use cmrt_realtime_play::{fast_midi_ipc::MAX_MIDI_MESSAGES, FastMidiEvent};

use super::{
    frames_ahead, GridScheduledMessage, GridSequencerContext, GridSequencerScreen, LOOKAHEAD,
};

/// コード進行データ更新のアナウンスを出しておく時間。読めるだけの長さがあればよい。
const RESTART_NOTICE_DURATION: Duration = Duration::from_secs(3);

impl GridSequencerScreen {
    /// 先読み分のステップを組み立て、offset つきでまとめて送る。
    ///
    /// 接続前・音色切替中は進めない。クロックの締切はそのまま残り、Ready 復帰時に
    /// now 基準へ張り直す（欠落ぶんをまとめて鳴らさない）。
    ///
    /// コード進行データの更新アナウンスを出し終えたら true を返す。共有ランタイムは
    /// これを受けてアプリを再起動する。
    pub fn pump_step(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) -> bool {
        let status = self.connection_status();
        // 上限バッファでもフレームドロップが止まらない環境では、裏読みが成立しない。
        if status.overloaded && !self.overload_applied {
            self.enter_single_buffering("overload");
        }
        if self.poll_start_wait(now, status.phase.accepts_notes()) {
            let scheduled = self.state.poll_steps(now, LOOKAHEAD);
            self.send_scheduled(&scheduled);
            if self.single_buffering {
                // 鳴らしきってからロードする。この間は演奏が止まる。
                self.advance_single_buffer_cycle(now, ctx);
            } else {
                // 進行の最終小節へ入っていれば、待機 bank へ次サイクルを先読みする。
                // 演奏は止まらない（差し替えは次の小節境界で起きる）。
                self.advance_cycle_swap(now, ctx);
            }
        }
        self.take_restart_request(now)
    }

    /// 再起動アナウンスの表示時間が過ぎていたら、一度だけ true を返す。
    fn take_restart_request(&mut self, now: Instant) -> bool {
        let Some(since) = self.restart_notice else {
            return false;
        };
        if now.saturating_duration_since(since) < RESTART_NOTICE_DURATION {
            return false;
        }
        self.restart_notice = None;
        true
    }

    /// 組み立て済みのメッセージを、`ahead` をフレーム数へ直して送る。
    pub(crate) fn send_scheduled(&self, scheduled: &[GridScheduledMessage]) {
        if scheduled.is_empty() {
            return;
        }
        let Some(sender) = &self.midi_sender else {
            return;
        };
        for batch in batches(scheduled, self.sample_rate) {
            sender.send_scheduled(batch);
        }
    }
}

/// 送信単位へ切り分ける。
///
/// 同じ `ahead` のメッセージ（＝同じステップ）は必ず1回の送信へまとめる。サーバー側は
/// 受信時の live 位置を基準に offset を解釈するため、バッチを跨ぐと基準がずれるから。
/// 1バッチの上限は共有メモリのスロット容量。
fn batches(scheduled: &[GridScheduledMessage], sample_rate: f64) -> Vec<Vec<FastMidiEvent>> {
    let mut batches: Vec<Vec<FastMidiEvent>> = Vec::new();
    let mut current: Vec<FastMidiEvent> = Vec::new();
    for group in group_by_ahead(scheduled) {
        if !current.is_empty() && current.len() + group.len() > MAX_MIDI_MESSAGES {
            batches.push(std::mem::take(&mut current));
        }
        let offset = frames_ahead(group[0].ahead, sample_rate);
        current.extend(group.iter().map(|scheduled| FastMidiEvent {
            instance_id: scheduled.instance_id,
            offset_frames: offset,
            message: scheduled.message,
        }));
    }
    if !current.is_empty() {
        batches.push(current);
    }
    batches
}

/// 同じ `ahead` を持つ連続したメッセージ（＝同じステップぶん）へ切り分ける。
fn group_by_ahead(scheduled: &[GridScheduledMessage]) -> Vec<&[GridScheduledMessage]> {
    let mut groups = Vec::new();
    let mut start = 0;
    for index in 1..=scheduled.len() {
        let ended = index == scheduled.len() || scheduled[index].ahead != scheduled[start].ahead;
        if ended {
            groups.push(&scheduled[start..index]);
            start = index;
        }
    }
    groups
}

#[cfg(test)]
mod tests;
