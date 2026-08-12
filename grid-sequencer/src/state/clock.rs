use std::time::{Duration, Instant};

/// grid sequencer の自動モードで使う既定テンポ。
pub const BPM: f64 = 130.0;
/// 1ステップ = 16分音符（1拍を4分割）。
pub const STEPS_PER_BEAT: u64 = 4;

const NANOS_PER_MINUTE: u64 = 60_000_000_000;
const NANOS_PER_SECOND: u128 = 1_000_000_000;

const STEP_NANOS: u64 = NANOS_PER_MINUTE / (130 * STEPS_PER_BEAT);

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
#[cfg(test)]
pub const SCHEDULE_GUARD: Duration = Duration::from_nanos(STEP_NANOS / 2);

pub fn step_interval_at(bpm: f64) -> Duration {
    duration_from_nanos_f64(NANOS_PER_MINUTE as f64 / (bpm * STEPS_PER_BEAT as f64))
}

pub fn lookahead_at(bpm: f64) -> Duration {
    step_interval_at(bpm) * 2
}

pub fn schedule_guard_at(bpm: f64) -> Duration {
    step_interval_at(bpm) / 2
}

/// アンカーから `steps` ステップぶん進んだ位置。整数演算のみで、丸め誤差を蓄積しない。
///
/// `STEP_INTERVAL * steps` とは一致しない（16ステップで 6ns ずれる）。ステップの絶対位置は
/// 必ずこちらで求めること。
pub const fn step_offset(steps: u64) -> Duration {
    Duration::from_nanos(steps.saturating_mul(NANOS_PER_MINUTE) / (130 * STEPS_PER_BEAT))
}

pub fn step_offset_at(steps: u64, bpm: f64) -> Duration {
    duration_from_nanos_f64(steps as f64 * NANOS_PER_MINUTE as f64 / (bpm * STEPS_PER_BEAT as f64))
}

fn duration_from_nanos_f64(nanos: f64) -> Duration {
    Duration::from_nanos(nanos.clamp(0.0, u64::MAX as f64) as u64)
}

/// Musical time for a step, calculated from its ordinal rather than by repeated addition.
pub fn step_timeline_seconds(step: u64) -> f64 {
    step_timeline_seconds_at(step, BPM)
}

pub fn step_timeline_seconds_at(step: u64, bpm: f64) -> f64 {
    step as f64 * 60.0 / (bpm * STEPS_PER_BEAT as f64)
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
#[derive(Debug)]
pub struct StepClock {
    /// 締切計算の基準時刻。`None` なら停止中。
    anchor: Option<Instant>,
    /// アンカーに対応するステップ番号。テンポを変えるたびに張り直すので 0 とは限らない。
    anchor_step: u64,
    /// アンカー時点の絶対 musical time（秒）。テンポを変えても `timeline_seconds` を
    /// 単調増加に保つために持つ。サーバーはこの秒数でイベントを並べるので、
    /// 過去へ戻すと鳴らし損ねる。
    anchor_seconds: f64,
    /// 次に発行するステップの通し番号。
    next_step: u64,
    bpm: f64,
}

impl Default for StepClock {
    fn default() -> Self {
        Self {
            anchor: None,
            anchor_step: 0,
            anchor_seconds: 0.0,
            next_step: 0,
            bpm: BPM,
        }
    }
}

impl StepClock {
    /// 次の `take_due()` で即座に1ステップ目が発火するようアンカーを `now` に置く。
    /// 画面へ入った瞬間から音を出すため、初回だけは待たせない。
    #[cfg(test)]
    pub(super) fn start(&mut self, now: Instant) {
        self.start_at_bpm(now, BPM);
    }

    pub(super) fn start_at_bpm(&mut self, now: Instant, bpm: f64) {
        self.anchor = Some(now);
        self.anchor_step = 0;
        self.anchor_seconds = 0.0;
        self.next_step = 0;
        self.bpm = bpm;
    }

    /// `at_step` の頭からテンポを変える。演奏は止めず、位相も musical time も繋がる。
    ///
    /// `at_step` の締切と絶対 musical time は**変更前のテンポで**求めてからアンカーに
    /// する。これで「そのステップはこれまでどおりの位置で鳴り、次のステップから新しい
    /// 間隔になる」となり、周の境目でテンポを差し替えても音が飛ばない。
    ///
    /// 丸めはアンカーを張り直すたびに1回だけ乗る（1ns 未満）。ステップの絶対位置は
    /// アンカーからの通し計算なので、周の内側では従来どおり誤差が積もらない。
    ///
    /// 乗り換えた絶対 musical time（＝新しい `anchor_seconds`）を返す。これはそのまま
    /// サーバーの tempo map へ積む「この秒から新テンポ」なので、呼び出し側は必ず送ること。
    /// 停止中は何もせず `None`。
    pub(super) fn retempo(&mut self, at_step: u64, bpm: f64) -> Option<f64> {
        self.anchor?;
        let anchor = self.deadline_of(at_step);
        let anchor_seconds = self.timeline_seconds(at_step);
        self.anchor_seconds = anchor_seconds;
        self.anchor = Some(anchor);
        self.anchor_step = at_step;
        self.bpm = bpm;
        Some(anchor_seconds)
    }

    pub(super) fn stop(&mut self) {
        self.anchor = None;
    }

    pub(super) fn is_running(&self) -> bool {
        self.anchor.is_some()
    }

    pub(super) fn bpm(&self) -> f64 {
        self.bpm
    }

    /// まだ組み立てていない、次に発行するステップ。
    ///
    /// 先読み済みのステップは既に送信済みで、その絶対 musical time は変更前のテンポで
    /// 確定している。演奏中にテンポを乗り換えるなら、境目はここより前にしないこと。
    pub(super) fn next_step(&self) -> u64 {
        self.next_step
    }

    pub(super) fn step_interval(&self) -> Duration {
        step_interval_at(self.bpm)
    }

    pub(super) fn schedule_guard(&self) -> Duration {
        schedule_guard_at(self.bpm)
    }

    pub(super) fn timeline_seconds(&self, step: u64) -> f64 {
        self.anchor_seconds
            + step_timeline_seconds_at(step.saturating_sub(self.anchor_step), self.bpm)
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
        self.anchor
            .is_some()
            .then(|| self.deadline_of(self.next_step))
    }

    /// アンカーから数えた `step` の絶対位置。アンカーが無いときは呼ばないこと。
    fn deadline_of(&self, step: u64) -> Instant {
        self.anchor.expect("running clock")
            + step_offset_at(step.saturating_sub(self.anchor_step), self.bpm)
    }

    /// 1ステップ以上遅れていたら、欠落したordinalを飛ばす。wall clockだけを張り直すと
    /// absolute timeline secondsが過去へ戻るため、元のanchorは維持する。
    fn snap_if_far_behind(&mut self, now: Instant) {
        let Some(deadline) = self.next_deadline() else {
            return;
        };
        if deadline + self.step_interval() <= now {
            let elapsed = now.saturating_duration_since(self.anchor.expect("running clock"));
            let next = (elapsed.as_secs_f64() * self.bpm * STEPS_PER_BEAT as f64 / 60.0).ceil();
            self.next_step = (next as u64 + self.anchor_step).max(self.next_step);
        }
    }
}

#[cfg(test)]
mod tests;
