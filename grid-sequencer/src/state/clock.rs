use std::time::{Duration, Instant};

/// grid sequencer の固定テンポ。
pub const BPM: u64 = 130;
/// 1ステップ = 16分音符（1拍を4分割）。
pub const STEPS_PER_BEAT: u64 = 4;

const NANOS_PER_MINUTE: u64 = 60_000_000_000;
const NANOS_PER_SECOND: u128 = 1_000_000_000;

const STEP_NANOS: u64 = NANOS_PER_MINUTE / (BPM * STEPS_PER_BEAT);

/// 1ステップの長さ。BPM130 では 115.3846ms。
///
/// `Duration::from_millis(115)` で丸めると1ステップあたり 0.38ms ずれ、16ステップで
/// 6ms 積もる。締切は毎回ステップ番号から絶対位置で計算するので、この定数は
/// 表示と「どれだけ遅れたか」の判定にだけ使う。
pub const STEP_INTERVAL: Duration = Duration::from_nanos(STEP_NANOS);

/// 先読み幅。ここまで先に鳴るステップをまとめて offset つきで送る。
///
/// UI のポーリング間隔と出力リングのレイテンシを必ず上回ること。下回ると offset が
/// 過去になってサーバー側で 0 へクランプされ、先読みなしと同じジッタに戻る。
pub const LOOKAHEAD: Duration = Duration::from_nanos(STEP_NANOS * 2);

/// 先読み済みの note on より後、次のステップより前に note off を置くための猶予（半ステップ）。
/// これがないと、送信済みで未発音の note on を先回りして止めてしまい音が残る。
pub const SCHEDULE_GUARD: Duration = Duration::from_nanos(STEP_NANOS / 2);

/// アンカーから `steps` ステップぶん進んだ位置。整数演算のみで、丸め誤差を蓄積しない。
///
/// `STEP_INTERVAL * steps` とは一致しない（16ステップで 6ns ずれる）。ステップの絶対位置は
/// 必ずこちらで求めること。
pub const fn step_offset(steps: u64) -> Duration {
    Duration::from_nanos(steps.saturating_mul(NANOS_PER_MINUTE) / (BPM * STEPS_PER_BEAT))
}

/// Musical time for a step, calculated from its ordinal rather than by repeated addition.
pub fn step_timeline_seconds(step: u64) -> f64 {
    step as f64 * 60.0 / (BPM * STEPS_PER_BEAT) as f64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ClockDeadline {
    pub(super) step: u64,
    pub(super) deadline: Instant,
}

/// `ahead`（今から鳴るまでの時間）を live MIDI の offset に使うフレーム数へ変換する。
pub fn frames_ahead(ahead: Duration, sample_rate: f64) -> u32 {
    if sample_rate <= 0.0 {
        return 0;
    }
    let frames = ahead.as_nanos().saturating_mul(sample_rate as u128) / NANOS_PER_SECOND;
    u32::try_from(frames).unwrap_or(u32::MAX)
}

/// ステップ進行の締切を管理するクロック。
///
/// 専用スレッドは持たず、UI ループが毎フレーム `take_due()` を呼んでポーリングする
/// （keyboard 画面の周期送信と同じ方式）。締切はアンカーからの絶対位置で計算するため、
/// ポーリング間隔がぶれてもテンポはずれない。
#[derive(Debug, Default)]
pub struct StepClock {
    /// 締切計算の基準時刻。`None` なら停止中。
    anchor: Option<Instant>,
    /// 次に発行するステップの通し番号。
    next_step: u64,
}

impl StepClock {
    /// 次の `take_due()` で即座に1ステップ目が発火するようアンカーを `now` に置く。
    /// 画面へ入った瞬間から音を出すため、初回だけは待たせない。
    pub(super) fn start(&mut self, now: Instant) {
        self.anchor = Some(now);
        self.next_step = 0;
    }

    pub(super) fn stop(&mut self) {
        self.anchor = None;
    }

    pub(super) fn is_running(&self) -> bool {
        self.anchor.is_some()
    }

    /// `now + lookahead` までに締切が来るステップを、締切つきで古い順に返す。
    /// 返したぶんは発行済みとして進む。
    ///
    /// 大幅遅延時（非Ready停滞など）は now 基準へアンカーを張り直し、復帰直後の
    /// バースト送信を防ぐ（欠落ステップはスキップ）。
    pub(super) fn take_due(&mut self, now: Instant, lookahead: Duration) -> Vec<ClockDeadline> {
        if self.anchor.is_none() {
            return Vec::new();
        }
        self.snap_if_far_behind(now);
        let horizon = now + lookahead;
        let mut due = Vec::new();
        while let Some(deadline) = self.next_deadline() {
            if deadline > horizon {
                break;
            }
            due.push(ClockDeadline {
                step: self.next_step,
                deadline,
            });
            self.next_step += 1;
        }
        due
    }

    fn next_deadline(&self) -> Option<Instant> {
        let anchor = self.anchor?;
        Some(anchor + step_offset(self.next_step))
    }

    /// 1ステップ以上遅れていたら、欠落したordinalを飛ばす。wall clockだけを張り直すと
    /// absolute timeline secondsが過去へ戻るため、元のanchorは維持する。
    fn snap_if_far_behind(&mut self, now: Instant) {
        let Some(deadline) = self.next_deadline() else {
            return;
        };
        if deadline + STEP_INTERVAL <= now {
            let elapsed = now.saturating_duration_since(self.anchor.expect("running clock"));
            let next = (elapsed.as_secs_f64() * (BPM * STEPS_PER_BEAT) as f64 / 60.0).ceil();
            self.next_step = (next as u64).max(self.next_step);
        }
    }
}

#[cfg(test)]
mod tests;
