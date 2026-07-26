use std::time::Instant;

use cmrt_realtime_play::fast_midi_ipc::MAX_MIDI_MESSAGES;

use super::{frames_ahead, GridScheduledMessage, GridSequencerScreen, LOOKAHEAD};

impl GridSequencerScreen {
    /// 先読み分のステップを組み立て、offset つきでまとめて送る。
    ///
    /// 接続前・音色切替中は進めない。クロックの締切はそのまま残り、Ready 復帰時に
    /// now 基準へ張り直す（欠落ぶんをまとめて鳴らさない）。
    pub fn pump_step(&mut self, now: Instant) {
        if !self.connection_status().phase.accepts_notes() {
            return;
        }
        let scheduled = self.state.poll_steps(now, LOOKAHEAD);
        self.send_scheduled(&scheduled);
    }

    /// 組み立て済みのメッセージを、`ahead` をフレーム数へ直して送る。
    pub(crate) fn send_scheduled(&self, scheduled: &[GridScheduledMessage]) {
        if scheduled.is_empty() {
            return;
        }
        let Some(sender) = &self.midi_sender else {
            return;
        };
        let patch = self.state.sound_patch();
        for batch in batches(scheduled, self.sample_rate) {
            sender.send_scheduled(batch, patch);
        }
    }
}

/// 送信単位へ切り分ける。
///
/// 同じ `ahead` のメッセージ（＝同じステップ）は必ず1回の送信へまとめる。サーバー側は
/// 受信時の live 位置を基準に offset を解釈するため、バッチを跨ぐと基準がずれるから。
/// 1バッチの上限は共有メモリのスロット容量。
fn batches(scheduled: &[GridScheduledMessage], sample_rate: f64) -> Vec<Vec<(u32, [u8; 3])>> {
    let mut batches: Vec<Vec<(u32, [u8; 3])>> = Vec::new();
    let mut current: Vec<(u32, [u8; 3])> = Vec::new();
    for group in group_by_ahead(scheduled) {
        if !current.is_empty() && current.len() + group.len() > MAX_MIDI_MESSAGES {
            batches.push(std::mem::take(&mut current));
        }
        let offset = frames_ahead(group[0].ahead, sample_rate);
        current.extend(group.iter().map(|scheduled| (offset, scheduled.message)));
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
