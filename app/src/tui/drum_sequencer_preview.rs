//! Drum の演奏（matrix の繰り返し再生と kit 試聴）と、共有 sender への受け渡し。

use std::time::Instant;

use cmrt_chord::TimedMidiEvent;
use cmrt_drum_sequencer::{DrumHit, DrumSequencerScreen, DRUM_BPM, DRUM_STEPS};
use cmrt_mml_overlay::{
    line_play::{LinePerformance, LineProgram},
    LivePatch, MmlOverlayLinePlayback, StepHit, StepLoop, StepShot,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;

use super::{
    drum_sequencer_glue::{DrumSequencerAction, KitAudition},
    TuiApp,
};

/// 16 分音符。
const STEP_SECONDS: f64 = 60.0 / DRUM_BPM / 4.0;
/// 16 step（4/4 の 1 小節）。打点や gate の位置では変わらない。
const LOOP_SECONDS: f64 = STEP_SECONDS * DRUM_STEPS as f64;
/// kit 試聴の 1 音の長さ。同じ note の次の打点が先に来れば、そこで切れる。
const NOTE_SECONDS: f64 = 2.0;
/// kit 試聴の note-on 間隔。matrix の step 間隔とは独立。
const AUDITION_NOTE_SECONDS: f64 = 0.25;
/// `a` で順に回す、繰り返し再生の先読みの秒。短いほど編集が早く届き、打点を落としやすい。
const LOOP_HORIZONS: [f64; 5] = [0.05, 0.1, 0.25, 0.5, 1.0];
const DEFAULT_LOOP_HORIZON: usize = 2;

/// [`LOOP_HORIZONS`] の位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct LoopHorizon(usize);

impl Default for LoopHorizon {
    fn default() -> Self {
        Self(DEFAULT_LOOP_HORIZON)
    }
}

impl LoopHorizon {
    pub(in crate::tui) fn seconds(self) -> f64 {
        LOOP_HORIZONS[self.0]
    }

    /// 1 つ長い値へ。最長の次は最短へ戻る。
    fn next(self) -> Self {
        Self((self.0 + 1) % LOOP_HORIZONS.len())
    }
}

fn note_events(events: &mut Vec<TimedMidiEvent>, note: u8, seconds: f64) {
    events.push(TimedMidiEvent {
        seconds,
        message: [0x90, note, 127],
    });
    events.push(TimedMidiEvent {
        seconds: seconds + NOTE_SECONDS,
        message: [0x80, note, 0],
    });
}

fn once(mut events: Vec<TimedMidiEvent>) -> LineProgram {
    events.sort_by(|left, right| left.seconds.total_cmp(&right.seconds));
    let loop_seconds = events.last().map_or(0.0, |event| event.seconds);
    LineProgram::once(LinePerformance {
        events,
        loop_seconds,
    })
}

/// kit の割当 note を低い順に 1 回ずつ、`AUDITION_NOTE_SECONDS` 間隔で鳴らす。
fn kit_audition_program(notes: &[u8]) -> LineProgram {
    let mut notes = notes.to_vec();
    notes.sort_unstable();
    notes.dedup();
    let mut events = Vec::new();
    for (index, &note) in notes.iter().enumerate() {
        note_events(&mut events, note, index as f64 * AUDITION_NOTE_SECONDS);
    }
    once(events)
}

/// 現在の kit と、編集中の pattern の積集合。gate は各セルの音長。
fn loop_hits(screen: &DrumSequencerScreen) -> Vec<StepHit> {
    let mut hits = Vec::new();
    for step in 0..DRUM_STEPS {
        for &note in screen.notes() {
            if let Some(hit) = screen.cell_hit(note, step) {
                hits.push(StepHit {
                    seconds: step as f64 * STEP_SECONDS,
                    note,
                    velocity: hit.velocity,
                    gate_seconds: f64::from(hit.steps) * STEP_SECONDS,
                });
            }
        }
    }
    hits
}

/// 繰り返し再生中の演奏位置。別の演奏・停止・開始前は `None`。
fn playhead_step(
    line: Option<MmlOverlayLinePlayback>,
    command: Option<u64>,
    now: Instant,
) -> Option<usize> {
    let line = line.filter(|line| {
        Some(line.command_id()) == command && line.ends_at().is_none() && line.is_sounding_at(now)
    })?;
    let elapsed = now.duration_since(line.started_at()).as_secs_f64();
    Some((((elapsed % LOOP_SECONDS) / STEP_SECONDS) as usize).min(DRUM_STEPS - 1))
}

fn is_preview_key(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && (key.code == KeyCode::Char('P')
            || (key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::SHIFT)))
}

impl TuiApp<'_> {
    /// selector を最初に処理する。true は主画面での終了要求。
    pub(in crate::tui) fn dispatch_drum_sequencer_key_event(&mut self, key: KeyEvent) -> bool {
        if self.handle_drum_sequencer_key_event(key) != DrumSequencerAction::Unhandled {
            return false;
        }
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if is_preview_key(key) {
            self.toggle_drum_sequencer_loop();
        } else if key.code == KeyCode::Char('a') && key.modifiers == KeyModifiers::NONE {
            self.cycle_drum_sequencer_loop_horizon();
        } else if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
            self.finish_drum_sequencer();
            return true;
        }
        false
    }

    /// 起動直後・画面に入ったとき・selector や MML overlay を閉じたときに、Shift+P を代わりに押す。
    /// kit の割当 note が届くまでは待つ。毎フレーム呼ぶ。
    pub(in crate::tui) fn pump_drum_sequencer_auto_play(&mut self) {
        let state = &self.drum_sequencer;
        if state.auto_play_spent
            || state.selector_open()
            || state.loop_hits.is_some()
            || state.screen.notes().is_empty()
            || self.mml_overlay.is_open()
        {
            return;
        }
        self.toggle_drum_sequencer_loop();
    }

    /// 繰り返し再生中なら止め、止まっていれば先頭から回す。ON のセルが無くても回す。
    fn toggle_drum_sequencer_loop(&mut self) {
        self.drum_sequencer.auto_play_spent = true;
        self.drum_sequencer.preview_error = None;
        let looping = self.drum_sequencer.loop_hits.is_some();
        let screen = &self.drum_sequencer.screen;
        // kit 未選択、割当が不明 / 空の kit では始めない。
        if looping || screen.notes().is_empty() {
            self.stop_drum_sequencer_preview();
            return;
        }
        let patch = LivePatch::new(screen.kit_name());
        let hits = loop_hits(screen);
        let Some(sender) = &self.mml_overlay_sender else {
            self.drum_sequencer.preview_error = Some("音源の送信先を利用できません。".to_string());
            return;
        };
        let command = sender.play_step_loop(
            patch,
            StepLoop {
                loop_seconds: LOOP_SECONDS,
                hits: hits.clone(),
                horizon_seconds: self.drum_sequencer.loop_horizon.seconds(),
            },
        );
        self.drum_sequencer.preview_command = Some(command);
        self.drum_sequencer.loop_hits = Some(hits);
    }

    /// 繰り返し再生中に matrix が変わったら、まだ送っていない step から反映させる。
    pub(in crate::tui) fn sync_drum_sequencer_loop_hits(&mut self) {
        let Some(sent) = &self.drum_sequencer.loop_hits else {
            return;
        };
        let hits = loop_hits(&self.drum_sequencer.screen);
        if *sent == hits {
            return;
        }
        if let Some(sender) = &self.mml_overlay_sender {
            sender.update_step_loop(hits.clone());
        }
        self.drum_sequencer.loop_hits = Some(hits);
    }

    /// 繰り返し再生中に ON にしたセルを、周回を待たずにいま 1 回鳴らす。止まっていれば鳴らさない。
    pub(in crate::tui) fn shoot_drum_sequencer_cell(&mut self, hit: DrumHit) {
        if self.drum_sequencer.loop_hits.is_none() {
            return;
        }
        if let Some(sender) = &self.mml_overlay_sender {
            sender.shoot_step_loop(StepShot {
                note: hit.note,
                velocity: hit.velocity,
                gate_seconds: f64::from(hit.steps) * STEP_SECONDS,
            });
        }
    }

    /// 先読みを次の値へ。回っているループにもその場で効かせる（周回位置は変えない）。
    fn cycle_drum_sequencer_loop_horizon(&mut self) {
        let horizon = self.drum_sequencer.loop_horizon.next();
        self.drum_sequencer.loop_horizon = horizon;
        if self.drum_sequencer.loop_hits.is_some() {
            if let Some(sender) = &self.mml_overlay_sender {
                sender.set_step_loop_horizon(horizon.seconds());
            }
        }
    }

    /// sender が公開した演奏開始時刻から、`now` に鳴っている step を画面へ渡す。
    pub(in crate::tui) fn sync_drum_sequencer_playhead(&mut self, now: Instant) {
        let line = self
            .mml_overlay_sender
            .as_ref()
            .and_then(|sender| sender.status().line_playback());
        let command = self
            .drum_sequencer
            .loop_hits
            .as_ref()
            .and(self.drum_sequencer.preview_command);
        self.drum_sequencer
            .screen
            .set_playhead(playhead_step(line, command, now));
    }

    /// 試聴は matrix の preview と同じ command 枠を使い、互いに置き換える。
    pub(in crate::tui) fn apply_kit_audition(&mut self, audition: KitAudition) {
        self.drum_sequencer.preview_error = None;
        self.drum_sequencer.loop_hits = None;
        let KitAudition::Play { patch, notes } = audition else {
            self.stop_drum_sequencer_preview();
            return;
        };
        let Some(sender) = &self.mml_overlay_sender else {
            self.drum_sequencer.preview_error = Some("音源の送信先を利用できません。".to_string());
            return;
        };
        let program = kit_audition_program(&notes);
        self.drum_sequencer.preview_command =
            Some(sender.stop_and_play_line(LivePatch::new(Some(patch.as_str())), program));
    }

    pub(in crate::tui) fn stop_drum_sequencer_preview(&mut self) {
        self.drum_sequencer.preview_command = None;
        self.drum_sequencer.loop_hits = None;
        self.drum_sequencer.screen.set_playhead(None);
        if let Some(sender) = &self.mml_overlay_sender {
            sender.stop();
        }
    }

    pub(in crate::tui) fn finish_drum_sequencer(&mut self) {
        self.stop_drum_sequencer_preview();
        self.drum_sequencer.close_selector();
    }

    pub(in crate::tui) fn draw_drum_sequencer(&mut self, frame: &mut Frame<'_>) {
        let status = self
            .mml_overlay_sender
            .as_ref()
            .map(|sender| sender.status());
        let command = self.drum_sequencer.preview_command;
        if let Some(error) = status
            .as_ref()
            .and_then(|status| command.and_then(|id| status.prepare_error_for(id)))
        {
            self.drum_sequencer.preview_error = Some(format!("鳴らせません: {error}"));
            // 準備に失敗したループは回っていない。次の Shift+P は停止ではなく開始にする。
            self.drum_sequencer.loop_hits = None;
        }
        let loading = status
            .as_ref()
            .is_some_and(|status| Some(status.command_id()) == command && status.is_loading());
        let playing = status
            .as_ref()
            .and_then(|status| status.line_playback())
            .is_some_and(|playback| {
                Some(playback.command_id()) == command
                    && playback.is_sounding_at(std::time::Instant::now())
            });
        let selector = self.drum_sequencer.selector_open();
        let looping = self.drum_sequencer.loop_hits.is_some();
        self.sync_drum_sequencer_playhead(Instant::now());
        let horizon = self.drum_sequencer.loop_horizon.seconds();
        // selector を開いている間の試聴の状態は、selector の枠内に出す。
        let text = if selector {
            "Drum kit 選択中  ?:help".to_string()
        } else if let Some(error) = self
            .drum_sequencer
            .storage_error
            .as_deref()
            .or(self.drum_sequencer.preview_error.as_deref())
        {
            // 保存の失敗は、入力が次の起動で消えることを意味するので再生の状態より先に出す。
            error.to_string()
        } else {
            match (loading, playing) {
                (true, _) => "音源を読み込み中です。".to_string(),
                (false, true) if looping => format!(
                    "再生中: BPM 120 / 4/4 / 繰り返し / 先読み {horizon}秒  a:先読み Shift+P:停止"
                ),
                (false, _) => format!("BPM 120 / 4/4 / 16 steps / 先読み {horizon}秒"),
            }
        };
        cmrt_drum_sequencer::ui::draw_with_status(&mut self.drum_sequencer.screen, &text, frame);
    }
}

#[cfg(test)]
mod tests;
