//! 周期を固定して回り続け、回している最中に打点を差し替えられるループ。
//!
//! [`super::line_playback`] の repeat は 1 周ぶんの固定列を数秒先まで積むので、積んだ後の
//! 編集は届かない。こちらは**短い先読みぶんだけ**を絶対秒で積み、まだ積んでいない時刻の
//! 打点は差し替え後の内容から作る。張り直さないので、差し替えても周回位置は先頭へ戻らない。
//!
//! note off は note on と一緒には積まない。同じ note の次の打点が先に来たら、その時刻で
//! off を出してから on を出す（同時刻は off が先）。こうすると、前の打点の off が後の
//! 打点の音を切ることも、周期が gate の長さで伸びることもない。gate は打点ごとに持つ。
//!
//! 周とは別に「いま 1 回」鳴らす単発（[`StepShot`]）も同じ規則で積む。積み済みの時刻より
//! 前へ割り込むので、同じ note の積み済みイベントと交差しない時刻を選ぶ。

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cmrt_chord::TimedMidiEvent;

use crate::{NOTE_OFF, NOTE_ON};

/// ループ 1 周の中の 1 打点。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepHit {
    /// 周の先頭からの秒。`0 <= seconds < loop_seconds` の外と velocity 0 は鳴らさない。
    pub seconds: f64,
    pub note: u8,
    pub velocity: u8,
    /// note on から note off までの最長の秒。同じ note の次の打点が先ならそこで切る。
    /// 周の長さを超えてよい。
    pub gate_seconds: f64,
}

/// 周回とは別に、いま 1 回だけ鳴らす打点。velocity 0 は鳴らさない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepShot {
    pub note: u8,
    pub velocity: u8,
    /// [`StepHit::gate_seconds`] と同じ。
    pub gate_seconds: f64,
}

/// [`super::MmlOverlaySender::play_step_loop`] で回すループ。
#[derive(Clone, Debug, PartialEq)]
pub struct StepLoop {
    /// 1 周の秒。打点や gate の位置では変わらない。
    pub loop_seconds: f64,
    pub hits: Vec<StepHit>,
    /// この秒先までを積んでおく。打点の差し替えが音に届くまでの最大の遅れでもある。
    /// 短いほど worker が頻繁に起き、起きるのが遅れたときに打点を落としやすい。
    pub horizon_seconds: f64,
}

/// 走っているループへの変更。command ではないので、列で待つ操作を置き換えない。
#[derive(Clone, Debug, PartialEq)]
pub(super) enum StepLoopEdit {
    Hits(Vec<StepHit>),
    Shot(StepShot),
    Horizon(f64),
}

/// 待ちの下限。0 を返すと worker が空回りする。
const MIN_WAIT: Duration = Duration::from_millis(1);

/// これより短い周期は回さない（1 回の積み込みで何千周も積まないため）。
const MIN_LOOP_SECONDS: f64 = 0.05;

pub(super) struct StepLoopState {
    /// `timeline_seconds = 0` に対応する実時刻。
    origin: Instant,
    loop_seconds: f64,
    horizon_seconds: f64,
    /// 周の中の秒の昇順。
    hits: Vec<StepHit>,
    /// ここまでの時刻（周をまたいだ絶対秒）は積んである。
    scheduled_until: f64,
    /// 積んだ note on のうち、note off をまだ積んでいないもの（note → off の絶対秒）。
    pending_offs: BTreeMap<u8, f64>,
    /// note ごとの、積んだ最後のイベント（on か off）の絶対秒。単発の割り込み先を選ぶのに使う。
    last_event: BTreeMap<u8, f64>,
    /// note ごとの、積んだ最後の on の絶対秒。
    last_on: BTreeMap<u8, f64>,
}

impl StepLoopState {
    /// 周期か先読みが回せない値なら `None`。
    pub(super) fn new(origin: Instant, step_loop: StepLoop) -> Option<Self> {
        let StepLoop {
            loop_seconds,
            hits,
            horizon_seconds,
        } = step_loop;
        if !loop_seconds.is_finite()
            || loop_seconds < MIN_LOOP_SECONDS
            || !is_valid_horizon(horizon_seconds)
        {
            return None;
        }
        let mut state = Self {
            origin,
            loop_seconds,
            horizon_seconds,
            hits: Vec::new(),
            scheduled_until: 0.0,
            pending_offs: BTreeMap::new(),
            last_event: BTreeMap::new(),
            last_on: BTreeMap::new(),
        };
        state.set_hits(hits);
        Some(state)
    }

    /// 打点を差し替える。積み済みの時刻には効かず、まだ積んでいない時刻から効く。
    pub(super) fn set_hits(&mut self, mut hits: Vec<StepHit>) {
        let loop_seconds = self.loop_seconds;
        hits.retain(|hit| {
            hit.velocity > 0
                && hit.gate_seconds.is_finite()
                && hit.seconds.is_finite()
                && (0.0..loop_seconds).contains(&hit.seconds)
        });
        hits.sort_by(|left, right| left.seconds.total_cmp(&right.seconds));
        self.hits = hits;
    }

    /// 先読みの秒を替える。積み済みの時刻はそのまま。回せない値は無視する。
    pub(super) fn set_horizon(&mut self, horizon_seconds: f64) {
        if is_valid_horizon(horizon_seconds) {
            self.horizon_seconds = horizon_seconds;
        }
    }

    /// いま積むべきイベントを、時刻順（同時刻は off が先）で返す。積んだものとして数える。
    ///
    /// 起きるのが遅れて過ぎてしまった時刻の打点は鳴らさない（遅れた note on をまとめて
    /// 鳴らさない）。off は過ぎていても出す。
    pub(super) fn take_due_events(&mut self, now: Instant) -> Vec<TimedMidiEvent> {
        let elapsed = self.elapsed(now);
        let until = elapsed + self.horizon_seconds;
        if until <= self.scheduled_until {
            return Vec::new();
        }
        let from = self.scheduled_until.max(elapsed);
        let mut events = Vec::new();
        let first_cycle = (from / self.loop_seconds).floor() as u64;
        let last_cycle = (until / self.loop_seconds).floor() as u64;
        for cycle in first_cycle..=last_cycle {
            let cycle_start = cycle as f64 * self.loop_seconds;
            for hit in &self.hits {
                let seconds = cycle_start + hit.seconds;
                if seconds < from || seconds >= until {
                    continue;
                }
                if let Some(off) = self.pending_offs.remove(&hit.note) {
                    events.push(note_off(hit.note, off.min(seconds)));
                }
                events.push(TimedMidiEvent {
                    seconds,
                    message: [NOTE_ON, hit.note, hit.velocity],
                });
                self.pending_offs
                    .insert(hit.note, seconds + hit.gate_seconds.max(0.0));
            }
        }
        self.scheduled_until = until;
        self.release_scheduled_offs(&mut events);
        self.finish(events)
    }

    /// `shot` をいま鳴らすイベントを返す。積み済みの時刻より前へ割り込む。
    ///
    /// 同じ note の on をいまより後に積んであるなら、その打点がすぐ鳴るので単発は出さない。
    /// 同じ note が積み済みの範囲で鳴り終わる（off を積んである）なら、その off の時刻まで
    /// 遅らせる（前の off が単発を切らないように）。
    pub(super) fn take_shot(&mut self, now: Instant, shot: StepShot) -> Vec<TimedMidiEvent> {
        if shot.velocity == 0 || !shot.gate_seconds.is_finite() {
            return Vec::new();
        }
        let elapsed = self.elapsed(now);
        let note = shot.note;
        if self.last_on.get(&note).is_some_and(|&on| on > elapsed) {
            return Vec::new();
        }
        let mut events = Vec::new();
        let seconds = if self.pending_offs.contains_key(&note) {
            // 鳴っている最中。最後に積んだのはいま以前の on なので、いま切って鳴らし直す。
            events.push(note_off(note, elapsed));
            elapsed
        } else {
            elapsed.max(self.last_event.get(&note).copied().unwrap_or(0.0))
        };
        events.push(TimedMidiEvent {
            seconds,
            message: [NOTE_ON, note, shot.velocity],
        });
        self.pending_offs
            .insert(note, seconds + shot.gate_seconds.max(0.0));
        self.release_scheduled_offs(&mut events);
        self.finish(events)
    }

    /// 次に [`Self::take_due_events`] が仕事をするまでの待ち。先読みの半分を割ったら起きる。
    pub(super) fn wait(&self, now: Instant) -> Duration {
        let ready_at = self.scheduled_until - self.horizon_seconds / 2.0;
        Duration::from_secs_f64((ready_at - self.elapsed(now)).max(0.0)).max(MIN_WAIT)
    }

    fn elapsed(&self, now: Instant) -> f64 {
        now.saturating_duration_since(self.origin).as_secs_f64()
    }

    /// 積み済みの範囲に入った off を出す。
    fn release_scheduled_offs(&mut self, events: &mut Vec<TimedMidiEvent>) {
        let until = self.scheduled_until;
        self.pending_offs.retain(|&note, &mut off| {
            if off < until {
                events.push(note_off(note, off));
                return false;
            }
            true
        });
    }

    /// 時刻順（同時刻は off が先）に並べ、note ごとの最後の時刻を記録する。
    fn finish(&mut self, mut events: Vec<TimedMidiEvent>) -> Vec<TimedMidiEvent> {
        events.sort_by(|left, right| {
            left.seconds
                .total_cmp(&right.seconds)
                .then_with(|| is_note_on(left).cmp(&is_note_on(right)))
        });
        for event in &events {
            let note = event.message[1];
            let last = self.last_event.entry(note).or_insert(0.0);
            *last = last.max(event.seconds);
            if is_note_on(event) {
                let last = self.last_on.entry(note).or_insert(0.0);
                *last = last.max(event.seconds);
            }
        }
        events
    }
}

fn is_valid_horizon(seconds: f64) -> bool {
    seconds.is_finite() && seconds > 0.0
}

fn note_off(note: u8, seconds: f64) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [NOTE_OFF, note, 0],
    }
}

fn is_note_on(event: &TimedMidiEvent) -> bool {
    event.message[0] == NOTE_ON && event.message[2] > 0
}

#[cfg(test)]
mod tests;
