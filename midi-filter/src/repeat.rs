//! 1 周ぶんのイベント列を、k 周目の絶対秒へずらす。
//!
//! repeat は「走っている timeline へ未来のイベントを継ぎ足す」形で作る。周回ごとに
//! timeline を張り直すと全 renderer のリセットとスケジュール済みイベントの破棄が入り、
//! 必ず継ぎ目が出るため。ずらすだけのこの関数がその継ぎ足しの素になる。

use crate::TimedMidiEvent;

#[cfg(test)]
mod tests;

/// 全イベントの時刻に同じ offset を足した新しい列を返す。message は触らない。
pub fn shift(events: &[TimedMidiEvent], offset_seconds: f64) -> Vec<TimedMidiEvent> {
    events
        .iter()
        .map(|event| TimedMidiEvent {
            seconds: event.seconds + offset_seconds,
            message: event.message,
        })
        .collect()
}
