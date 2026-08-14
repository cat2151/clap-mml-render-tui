use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

mod arpeggio;
mod attack;
mod bass_line;
mod cc1;
mod chord;
mod clock;
mod cycle;
mod drum;
mod edit;
mod instance;
mod lanes;
mod measure_lane;
mod note_pattern;
mod pattern_bag;
mod randomize;
mod schedule;
mod session;
mod swing;
mod tempo;
mod velocity;

pub use edit::PitchDirection;
pub use instance::{
    GridInstance, GridLane, GridLaneMode, LaneAddress, VisibleNoteRow, VisibleRowKind,
    BASS_OCTAVE_LANES, CHORD_VOICE_LANES,
};
pub use note_pattern::{NotePattern, NoteStep};
pub use pattern_bag::PatternCombination;

pub use chord::{ChordPlayback, ARPEGGIO_ROW, BASS_ROW, CHORD_ROW};
use clock::StepClock;
#[cfg(test)]
pub use clock::SCHEDULE_GUARD;
pub use clock::{
    frames_ahead, lookahead_at, schedule_guard_at, step_interval_at, step_offset, step_offset_at,
    step_timeline_seconds, step_timeline_seconds_at, BPM, LOOKAHEAD, STEPS_PER_BEAT, STEP_INTERVAL,
};
pub use drum::{FIRST_DRUM_ROW, FULL_DRUM_TRACK_COUNT};
pub use randomize::{randomize_instance_slice, DrawnPhrases};
pub use swing::{clamp_swing, swing_offset_seconds, SWING_MAX, SWING_MIN};
pub use tempo::AppliedTempo;

#[cfg(test)]
pub type GridRow = GridInstance;

/// grid の既定・最大 instance 数。
pub const GRID_ROWS: usize = 16;
/// grid の列数（＝1周のステップ数）。
pub const GRID_STEPS: usize = 16;

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
/// 行の note number の既定値（C4）。
const DEFAULT_NOTE: u8 = 60;

/// 発音中のノートと、その残りステップ数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SoundingNote {
    owner: SoundOwner,
    instance_id: u8,
    midi_note: u8,
    remaining_steps: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SoundOwner {
    Lane(LaneAddress),
    ChordInstance { instance: usize },
}

/// 先読みで組み立てた、まだ鳴っていない MIDI メッセージ1件。
///
/// `ahead` は「`poll_steps()` に渡した `now` から実際に鳴るまで」の時間。送信側が
/// これをフレーム数へ直して live MIDI の offset に載せる。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridScheduledMessage {
    pub instance_id: u8,
    pub ahead: Duration,
    /// Absolute musical time from the most recent Grid timeline start.
    pub timeline_seconds: f64,
    pub message: [u8; 3],
}

/// grid sequencer 画面のドメイン状態。
///
/// I/O は行わず、進行の結果を MIDI メッセージ列として返すだけ。実際の送信は
/// `GridSequencerScreen` が `GridMidiSender` 経由で行う。
#[derive(Debug)]
pub struct GridState {
    instances: Vec<GridInstance>,
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
    last_scheduled_timeline_seconds: Option<f64>,
    last_poll_lateness: Duration,
    /// chord mode の再生状態。`None` なら従来どおりの単音演奏。
    chord: Option<ChordPlayback>,
    /// 直近の引き直しで当てた型（[`DrawnPhrases`]）。表示のためだけに持つ。
    drawn: DrawnPhrases,
    /// Cycle Randomの有効対象別にpattern組み合わせを重複なしで1周させるbag群。
    pattern_bags: pattern_bag::PatternBags,
    /// いま鳴らしている bank（0 か 1）。instance index を bank 幅として写す。
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
    /// 次に1周したところで乗り換えるテンポ。抽選は範囲を持つ画面側の仕事なので、
    /// ここは預かって周の頭で適用するだけ。
    next_cycle_bpm: Option<f64>,
    /// 周の頭で実際に乗り換えたテンポ。画面側が表示の追従とサーバーへの通知に取る。
    applied_cycle_bpm: Option<AppliedTempo>,
    clock: StepClock,
}

impl Default for GridState {
    fn default() -> Self {
        Self::with_instance_count(GRID_ROWS)
    }
}

impl GridState {
    pub fn with_instance_count(instance_count: usize) -> Self {
        assert!(instance_count > 0, "grid instance count must be positive");
        let mut instances = (0..instance_count)
            .map(GridInstance::new)
            .collect::<Vec<_>>();
        // 役割は行番号ではなく track 数から決まるので、instance を並べ終えてから当てる。
        drum::apply_drum_roles(&mut instances);
        let stored_lane_count = instances.iter().map(|instance| instance.lanes.len()).sum();
        Self {
            instances,
            step_index: 0,
            schedule_index: 0,
            started: false,
            sounding: Vec::new(),
            cc1: measure_lane::MeasureLane::new(
                instance_count,
                cc1::CC1_CHOICES,
                measure_lane::LaneCoverage::EveryStep,
            ),
            velocity: measure_lane::MeasureLane::new(
                stored_lane_count,
                velocity::VELOCITY_CHOICES,
                measure_lane::LaneCoverage::SoundingCells,
            ),
            pending_display: VecDeque::new(),
            last_scheduled: None,
            last_scheduled_timeline_seconds: None,
            last_poll_lateness: Duration::ZERO,
            chord: None,
            drawn: DrawnPhrases::default(),
            pattern_bags: pattern_bag::PatternBags::default(),
            bank: 0,
            pending: None,
            pending_ready: false,
            preload_due: false,
            stop_at_cycle_end: false,
            cycle_wrapped: false,
            cycle_stopped_at: None,
            next_cycle_bpm: None,
            applied_cycle_bpm: None,
            clock: StepClock::default(),
        }
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    #[cfg(test)]
    pub fn with_row_count(row_count: usize) -> Self {
        Self::with_instance_count(row_count)
    }

    /// テスト用。全 lane の譜面を空にした grid。
    ///
    /// drum 行（[`drum`]）は作られた時点でリズムが入っているので、「新品の grid は
    /// 無音」を前提にしたテストはこちらを使う。
    #[cfg(test)]
    pub fn silent_with_instance_count(instance_count: usize) -> Self {
        let mut state = Self::with_instance_count(instance_count);
        for instance in &mut state.instances {
            for lane in &mut instance.lanes {
                lane.pattern.clear();
            }
        }
        state
    }

    #[cfg(test)]
    pub fn silent() -> Self {
        Self::silent_with_instance_count(GRID_ROWS)
    }

    #[cfg(test)]
    pub fn row_count(&self) -> usize {
        self.instance_count()
    }

    #[cfg(test)]
    pub fn rows(&self) -> &[GridInstance] {
        self.instances()
    }

    #[cfg(test)]
    pub fn rows_mut(&mut self) -> &mut [GridInstance] {
        self.instances_mut()
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
        self.start_at_bpm(now, BPM);
    }

    pub fn start_at_bpm(&mut self, now: Instant, bpm: f64) {
        self.step_index = 0;
        self.schedule_index = 0;
        self.started = false;
        self.sounding.clear();
        self.reset_lanes_for_start();
        self.pending_display.clear();
        self.last_scheduled = None;
        self.last_scheduled_timeline_seconds = None;
        self.last_poll_lateness = Duration::ZERO;
        self.reset_cycle_stop();
        // クロックを作り直すので、周の頭へ向けた古い予約は意味を失う。
        self.disarm_next_cycle_bpm();
        self.clock.start_at_bpm(now, bpm);
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
