//! 「いつ次の周を積むか」だけを決める、走っているループの状態。
//!
//! **ウォールクロックが絡むのはここだけで、鳴る位置には効かない。** 積む中身は
//! 1 周ぶんを `k * loop_seconds` ずらした絶対秒なので、いつ積んでも鳴る時刻は同じ。
//! 遅れて積んだ場合だけサーバー側が late として頭へ寄せるので、[`REPEAT_HORIZON_SECONDS`]
//! ぶん先まで前もって積んでおく。
//!
//! 周回番号（[`RepeatState::next_cycle`]）を持ち回るのが要で、
//! 「いまの秒から逆算する」形にはしていない。逆算だと積み漏れや二重積みが起きる。

use std::time::{Duration, Instant};

use cmrt_chord::TimedMidiEvent;
use cmrt_realtime_play::TimelineId;

use crate::line_play::FilterSettings;

/// 先読みでここまで積んでおく。ここを割ったら次の周を積む。
///
/// 4 秒はサーバー側の pending 上限（8192）から見ても十分小さい。1 周 0.05 秒の
/// 最短ループでも 80 周ぶんしか先行しない。
pub(super) const REPEAT_HORIZON_SECONDS: f64 = 4.0;

/// これより短い行は繰り返さない。
///
/// `loop_seconds` は「最後のイベントまで」なので、1 音だけの行や解釈に失敗した行では
/// 0 に近い値になり得る。そのまま回すと 1 回の [`RepeatState::take_due_cycles`] で
/// 何千周ぶんも積んでしまうため、下限を切って 1 回だけの演奏へ落とす。
pub(super) const MIN_LOOP_SECONDS: f64 = 0.05;

/// 1 回の pump で積む周回数の上限。時計が飛んだときの暴走止め。
const MAX_CYCLES_PER_PUMP: usize = 128;

/// 待ちの下限。
///
/// **0 を返すと worker が回り続ける。** 積んだ直後は「horizon をちょうど満たした」状態に
/// なることがあり、そこで待ち 0 を返すと「起きる → 積むものが無い → また待ち 0」の
/// 空回りになる。4 秒の先読みに対して 1ms は誤差なので、下限を切って潰す。
const MIN_WAIT: Duration = Duration::from_millis(1);

/// 走っているループ。1 つの timeline に継ぎ足し続ける。
pub(super) struct RepeatState {
    /// 継ぎ足す先。**これが変わらないことが「継ぎ目が無い」ことの定義**。
    timeline_id: TimelineId,
    /// `timeline_seconds = 0` に対応する実時刻。
    origin: Instant,
    /// 1 周ぶん（filter を掛ける前）。周ごとにここから作り直す。
    cycle: Vec<TimedMidiEvent>,
    loop_seconds: f64,
    /// 周ごとに掛ける filter。**周をまたいで変えない**（変えるなら新しい行として張り直す）。
    filters: FilterSettings,
    /// 次に積む周回番号。0 周目も [`Self::take_due_cycles`] が返す。
    next_cycle: u64,
}

impl RepeatState {
    /// `origin` は timeline を張った時刻。0 周目はまだ積んでいない状態で始まる。
    pub(super) fn new(
        timeline_id: TimelineId,
        origin: Instant,
        cycle: Vec<TimedMidiEvent>,
        loop_seconds: f64,
        filters: FilterSettings,
    ) -> Self {
        Self {
            timeline_id,
            origin,
            cycle,
            loop_seconds,
            filters,
            next_cycle: 0,
        }
    }

    pub(super) fn timeline_id(&self) -> TimelineId {
        self.timeline_id
    }

    pub(super) fn cycle(&self) -> &[TimedMidiEvent] {
        &self.cycle
    }

    pub(super) fn loop_seconds(&self) -> f64 {
        self.loop_seconds
    }

    pub(super) fn filters(&self) -> FilterSettings {
        self.filters
    }

    /// いま積むべき周の開始秒（`k * loop_seconds`）を、古い順に返す。
    ///
    /// 返した周は積んだものとして数える。送信に失敗したら呼び出し側がループごと捨てる。
    pub(super) fn take_due_cycles(&mut self, now: Instant) -> Vec<f64> {
        let elapsed = now.saturating_duration_since(self.origin).as_secs_f64();
        let horizon = elapsed + REPEAT_HORIZON_SECONDS;
        let mut offsets = Vec::new();
        while self.scheduled_until() < horizon && offsets.len() < MAX_CYCLES_PER_PUMP {
            offsets.push(self.next_cycle as f64 * self.loop_seconds);
            self.next_cycle += 1;
        }
        offsets
    }

    /// 次に [`Self::take_due_cycles`] が仕事をする時刻までの待ち。
    pub(super) fn wait(&self, now: Instant) -> Duration {
        let ready_at = self.scheduled_until() - REPEAT_HORIZON_SECONDS;
        let elapsed = now.saturating_duration_since(self.origin).as_secs_f64();
        Duration::from_secs_f64((ready_at - elapsed).max(0.0)).max(MIN_WAIT)
    }

    /// ここまでは積んである、という絶対秒。
    fn scheduled_until(&self) -> f64 {
        self.next_cycle as f64 * self.loop_seconds
    }
}

/// この行を繰り返せるか。繰り返せない行は 1 回だけ鳴らす。
pub(super) fn is_repeatable(loop_seconds: f64) -> bool {
    loop_seconds.is_finite() && loop_seconds >= MIN_LOOP_SECONDS
}

#[cfg(test)]
mod tests;
