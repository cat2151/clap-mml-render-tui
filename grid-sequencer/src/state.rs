use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

mod attack;
mod cc1;
mod chord;
mod clock;
mod cycle;
mod measure_lane;
mod randomize;
mod velocity;

pub use chord::{snap_rows_to_chord, ChordPlayback, CHORD_ROW};
pub use clock::{frames_ahead, step_offset, BPM, LOOKAHEAD, STEPS_PER_BEAT, STEP_INTERVAL};
use clock::{StepClock, SCHEDULE_GUARD};
pub use randomize::{pick_chord_patch, randomize_row_slice};

/// grid の既定・最大行数（＝パート数）。
pub const GRID_ROWS: usize = 16;
/// grid の列数（＝1周のステップ数）。
pub const GRID_STEPS: usize = 16;

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
/// 行の note number の既定値（C4）。
const DEFAULT_NOTE: u8 = 60;

/// 行の音長。ステップ長（16分音符）の何個ぶん鳴らすかを決める。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepDuration {
    #[default]
    Sixteenth,
    Quarter,
    /// 全音符。grid 1周（16ステップ）ぶん鳴らし続ける。chord mode の和音で使う。
    Whole,
}

impl StepDuration {
    /// この音長が占めるステップ数。
    pub fn steps(self) -> u8 {
        match self {
            Self::Sixteenth => 1,
            Self::Quarter => 4,
            Self::Whole => GRID_STEPS as u8,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sixteenth => "1/16",
            Self::Quarter => "1/4",
            Self::Whole => "1/1",
        }
    }
}

/// grid の1行。1行 = 1パート。
///
/// 行番号を realtime play server の instance ID として使い、`patch` と MIDI を
/// その行専用の CLAP instance へ送る。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridRow {
    pub patch: Option<String>,
    /// 実際に鳴らす note number。chord mode 中は `base_note` をコード構成音へ
    /// 寄せた値が入る。
    pub note: u8,
    /// ランダム抽選した素の note number。chord mode を切ったときの復帰点であり、
    /// コードへ寄せるときの「音域の目安」でもある。
    pub base_note: u8,
    pub duration: StepDuration,
    pub cells: [bool; GRID_STEPS],
}

impl Default for GridRow {
    fn default() -> Self {
        Self {
            patch: None,
            note: DEFAULT_NOTE,
            base_note: DEFAULT_NOTE,
            duration: StepDuration::default(),
            cells: [false; GRID_STEPS],
        }
    }
}

/// 発音中のノートと、その残りステップ数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SoundingNote {
    instance_id: u8,
    midi_note: u8,
    remaining_steps: u8,
}

/// 先読みで組み立てた、まだ鳴っていない MIDI メッセージ1件。
///
/// `ahead` は「`poll_steps()` に渡した `now` から実際に鳴るまで」の時間。送信側が
/// これをフレーム数へ直して live MIDI の offset に載せる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridScheduledMessage {
    pub instance_id: u8,
    pub ahead: Duration,
    pub message: [u8; 3],
}

/// grid sequencer 画面のドメイン状態。
///
/// I/O は行わず、進行の結果を MIDI メッセージ列として返すだけ。実際の送信は
/// `GridSequencerScreen` が `GridMidiSender` 経由で行う。
#[derive(Debug)]
pub struct GridState {
    rows: Vec<GridRow>,
    /// 表示用の再生位置。先読みで組み立て済みでも、締切が来るまでは進めない。
    step_index: usize,
    /// メッセージ組み立て用のカーソル。先読みぶんだけ `step_index` より先を指す。
    schedule_index: usize,
    /// 1ステップ目をまだ鳴らしていない間だけ false。`advance_schedule()` が
    /// step 0 を飛ばさないようにするためのフラグ。
    started: bool,
    sounding: Vec<SoundingNote>,
    /// 小節ごとに抽選する CC1 と、実発音まで待たせる表示状態。
    cc1: measure_lane::MeasureLane,
    /// 小節ごとに抽選する note velocity。CC1 と同じ仕組みで動く。
    velocity: measure_lane::MeasureLane,
    /// 組み立て済みで、まだ締切が来ていないステップの (締切, 列)。表示位置を
    /// 実際に鳴るタイミングへ合わせるために持つ。
    pending_display: VecDeque<(Instant, usize)>,
    /// 先読みで既に送ってしまったステップのうち、いちばん新しいものの締切。
    /// 送信済みの note on より後ろへ note off を置くために見る。
    last_scheduled: Option<Instant>,
    /// chord mode の再生状態。`None` なら従来どおりの単音演奏。
    chord: Option<ChordPlayback>,
    /// いま鳴らしている bank（0 か 1）。行 `r` は instance `bank * 行数 + r` へ写る。
    bank: usize,
    /// 抽選済みで、先読みロードが終われば差し替える次サイクル。詳細は [`cycle`]。
    pending: Option<cycle::PendingCycle>,
    /// 待機 bank の先読みロードが終わったか。立っていないと差し替えない。
    pending_ready: bool,
    /// 進行の最終小節へ入ったことを画面側へ伝えるフラグ。抽選はカタログと rng を
    /// 持つ画面側の仕事なので、ここでは合図だけを立てる。
    preload_due: bool,
    /// サイクルを鳴らしきったらクロックを止める（シングルバッファリング。詳細は
    /// [`crate::single_buffer`]）。
    stop_at_cycle_end: bool,
    /// 進行を1周し終えたことを `poll_steps` へ伝える一度きりの合図。
    cycle_wrapped: bool,
    /// 鳴らしきった最後の音の締切。画面側が出力の吐き出し待ちの起点に使う。
    cycle_stopped_at: Option<Instant>,
    clock: StepClock,
}

impl Default for GridState {
    fn default() -> Self {
        Self::with_row_count(GRID_ROWS)
    }
}

impl GridState {
    pub fn with_row_count(row_count: usize) -> Self {
        assert!(row_count > 0, "grid row count must be positive");
        Self {
            rows: vec![GridRow::default(); row_count],
            step_index: 0,
            schedule_index: 0,
            started: false,
            sounding: Vec::new(),
            cc1: measure_lane::MeasureLane::new(row_count, cc1::CC1_CHOICES),
            velocity: measure_lane::MeasureLane::new(row_count, velocity::VELOCITY_CHOICES),
            pending_display: VecDeque::new(),
            last_scheduled: None,
            chord: None,
            bank: 0,
            pending: None,
            pending_ready: false,
            preload_due: false,
            stop_at_cycle_end: false,
            cycle_wrapped: false,
            cycle_stopped_at: None,
            clock: StepClock::default(),
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn rows(&self) -> &[GridRow] {
        &self.rows
    }

    /// 行を直接書き換える。将来のセル編集 UI と、決め打ちの grid を作るテストで使う。
    pub fn rows_mut(&mut self) -> &mut [GridRow] {
        &mut self.rows
    }

    /// 現在の再生位置（列）。
    pub fn step_index(&self) -> usize {
        self.step_index
    }

    pub fn is_running(&self) -> bool {
        self.clock.is_running()
    }

    /// chord mode の再生状態。`None` なら従来どおりの単音演奏。
    pub fn chord(&self) -> Option<&ChordPlayback> {
        self.chord.as_ref()
    }

    /// 画面へ入るときの初期化。再生位置を先頭へ戻してクロックを走らせる。
    pub fn start(&mut self, now: Instant) {
        self.step_index = 0;
        self.schedule_index = 0;
        self.started = false;
        self.sounding.clear();
        self.reset_lanes_for_start();
        self.pending_display.clear();
        self.last_scheduled = None;
        self.reset_cycle_stop();
        self.clock.start(now);
    }

    /// `now + lookahead` までに鳴るステップをまとめて組み立て、送るべき MIDI メッセージを
    /// 「今から鳴るまでの時間」つきで返す。締切がまだ先なら空を返す。
    ///
    /// 先読みして送るので、UI のポーリング間隔ぶんのジッタが発音位置に乗らない。
    pub fn poll_steps(&mut self, now: Instant, lookahead: Duration) -> Vec<GridScheduledMessage> {
        let mut scheduled = Vec::new();
        for deadline in self.clock.take_due(now, lookahead) {
            let ahead = deadline.saturating_duration_since(now);
            let mut messages = self.expire_sounding();
            self.advance_schedule();
            let stopping = std::mem::take(&mut self.cycle_wrapped);
            if stopping {
                // 鳴らしきった。次の小節は組み立てず、残っている音を止めてクロックを畳む。
                messages.extend(self.silence_sounding());
                self.clock.stop();
                self.cycle_stopped_at = Some(deadline);
            } else {
                if self.schedule_index == 0 {
                    self.prepare_lane_measures(deadline);
                }
                messages.extend(self.attack_current_step());
            }
            scheduled.extend(messages.into_iter().map(|(instance_id, message)| {
                GridScheduledMessage {
                    instance_id,
                    ahead,
                    message,
                }
            }));
            self.pending_display
                .push_back((deadline, self.schedule_index));
            self.last_scheduled = Some(deadline);
            if stopping {
                break;
            }
        }
        self.advance_display(now);
        scheduled
    }

    /// 鳴っている音を止める note off を、送信済みの先読みぶんより後ろへ置くための猶予。
    /// まだ何も送っていなければ即座（0）で良い。
    pub(crate) fn silence_ahead(&self, now: Instant) -> Duration {
        match self.last_scheduled {
            Some(deadline) => (deadline + SCHEDULE_GUARD).saturating_duration_since(now),
            None => Duration::ZERO,
        }
    }

    /// 鳴っている音をすべて止める note off を作り、再生位置とクロックをリセットする。
    /// 画面を離れるときに呼び、音が鳴りっぱなしになるのを防ぐ。
    pub fn take_reset_messages(&mut self) -> Vec<GridScheduledMessage> {
        self.clock.stop();
        self.step_index = 0;
        self.schedule_index = 0;
        self.started = false;
        self.reset_lanes_for_start();
        self.pending_display.clear();
        self.last_scheduled = None;
        self.reset_cycle_stop();
        self.silence_sounding()
            .into_iter()
            .map(|(instance_id, message)| GridScheduledMessage {
                instance_id,
                ahead: Duration::ZERO,
                message,
            })
            .collect()
    }

    /// 締切を過ぎたステップまで表示位置を進める。先読みぶんが先走って見えるのを防ぐ。
    fn advance_display(&mut self, now: Instant) {
        while self
            .pending_display
            .front()
            .is_some_and(|(deadline, _)| *deadline <= now)
        {
            let (_, step) = self
                .pending_display
                .pop_front()
                .expect("front was just observed");
            self.step_index = step;
        }
        self.advance_lane_displays(now);
    }

    /// 鳴っている音の残りステップを1減らし、尽きたものの note off を返す。
    fn expire_sounding(&mut self) -> Vec<(u8, [u8; 3])> {
        let mut messages = Vec::new();
        self.sounding.retain_mut(|note| {
            note.remaining_steps = note.remaining_steps.saturating_sub(1);
            if note.remaining_steps == 0 {
                messages.push((note.instance_id, note_off(note.midi_note)));
                false
            } else {
                true
            }
        });
        messages
    }

    fn advance_schedule(&mut self) {
        if self.started {
            self.schedule_index = (self.schedule_index + 1) % GRID_STEPS;
            if self.schedule_index == 0 {
                // grid を1周したので、chord mode なら次のコードへ進む。
                self.advance_chord();
            }
        } else {
            self.started = true;
        }
    }

    /// 鳴っている音の note off だけを作り、発音中リストを空にする。
    fn silence_sounding(&mut self) -> Vec<(u8, [u8; 3])> {
        self.sounding
            .drain(..)
            .map(|note| (note.instance_id, note_off(note.midi_note)))
            .collect()
    }
}

fn note_on(midi_note: u8, velocity: u8) -> [u8; 3] {
    [NOTE_ON, midi_note, velocity]
}

fn note_off(midi_note: u8) -> [u8; 3] {
    [NOTE_OFF, midi_note, 0]
}

#[cfg(test)]
mod tests;
