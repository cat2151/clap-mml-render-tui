//! grid sequencer 画面（1/2/3/4/7/8/16 instancesのnote lanesを16ステップでループ再生する）。
//!
//! 画面の状態・入力・描画・MIDI 送信はこの crate に閉じており、共有ランタイム
//! （app 側の `TuiApp`）からは `GridSequencerContext` で必要な情報を注入してもらう。
//! app 側との接続は app crate の `tui::grid_sequencer_glue` にある。
//!
//! grid の各instanceを realtime play server の同番号CLAP instanceへ対応付ける。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

pub use cmrt_arpeggiator::{ArpPattern, BassPattern};
use cmrt_realtime_play::PatchVoicing;
pub use cmrt_rhythm::{DrumPattern, DrumRole};

mod arpeggio;
mod bass_line;
mod chord_input;
mod chord_mode;
mod context;
mod cycle_random;
mod cycle_swap;
mod drum_line;
mod input;
mod patch_bag;
mod patch_notice;
mod patch_role;
mod patch_selector;
mod screen;
mod screen_runtime;
mod sender;
mod session;
mod single_buffer;
mod start_wait;
mod state;
mod tempo;
pub mod ui;
mod undo;

pub use context::{GridPatchLoad, GridPatchStatus, GridSequencerContext};
pub use cycle_random::{CycleRandom, CycleRandomItem};
pub(crate) use input::ListDirection;
pub use screen::{GridSequencerParts, GridSequencerScreen};
pub use sender::{
    GridConnectionPhase, GridConnectionStatus, GridMidiSender, GridProgress, GridRowPatchPhase,
    GridRowPatchStatus, GridRowReadiness,
};
pub use session::{FixedChordProgression, GridSequencerSession};
pub use state::{
    clamp_swing, frames_ahead, lookahead_at, randomize_instance_slice, schedule_guard_at,
    step_interval_at, step_offset, step_offset_at, step_timeline_seconds, step_timeline_seconds_at,
    swing_offset_seconds, AppliedTempo, ChordPlayback, DrawnPhrases, GridInstance, GridLane,
    GridLaneMode, GridScheduledMessage, GridState, LaneAddress, NotePattern, NoteStep,
    PatternCombination, PitchDirection, VisibleNoteRow, VisibleRowKind, ARPEGGIO_ROW,
    BASS_OCTAVE_LANES, BASS_ROW, BPM, CHORD_ROW, CHORD_VOICE_LANES, FIRST_DRUM_ROW,
    FULL_DRUM_TRACK_COUNT, GRID_ROWS, GRID_STEPS, LOOKAHEAD, STEPS_PER_BEAT, STEP_INTERVAL,
    SWING_MAX, SWING_MIN,
};

#[cfg(test)]
pub use state::GridRow;

/// patch が mono か poly かの判定を共有ランタイムから引くための窓口。
///
/// chord mode の和音は poly patch でしか成立しないため、抽選の当たり判定に使う。
pub trait GridVoicingLookup {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing>;
}

/// voicing 判定を一切持たない lookup。テストと、判定データ無効時のフォールバック用。
pub struct NoVoicingLookup;

impl GridVoicingLookup for NoVoicingLookup {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

type LogSink = fn(&str);
static LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合（テスト実行時を含む）、この crate のログは黙って捨てられる。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

pub(crate) fn log_line(message: &str) {
    if let Some(sink) = LOG_SINK.get() {
        sink(message);
    }
}

/// 完全ランダム演奏中の chord mode で、和音の行へ与える音量差（dB）。他の行は 0 dB のまま。
pub const CHORD_GAIN_DB: f32 = 6.0;

/// instance ごとの音量差（dB）。譜面ごと毎周変わる完全ランダム演奏中だけ、chord mode の
/// 和音の行が持ち上がる。
///
/// 返す長さは bank 2 本ぶん（= `instance_count * BANK_COUNT`）。差し替え先の bank にも
/// 同じ音量差を載せておく必要があるので、両方の `CHORD_ROW` を持ち上げる。
pub fn chord_gains_db(instance_count: usize, chord_on: bool, note_random: bool) -> Vec<f32> {
    let boost_chord = chord_on && note_random;
    (0..instance_count * cmrt_realtime_play::BANK_COUNT)
        .map(|instance| {
            if boost_chord && instance % instance_count == CHORD_ROW {
                CHORD_GAIN_DB
            } else {
                0.0
            }
        })
        .collect()
}

/// 画面が返す、共有ランタイム側で処理すべき遷移要求。
pub enum GridSequencerAction {
    Continue,
    Quit,
    RestartWithTrackCount(usize),
}

impl GridSequencerScreen {
    /// 画面へ入る。初回はランダムな grid を作り、2回目以降は前回の grid を続ける。
    pub fn enter(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        if self.grid_ready {
            self.resume(now, ctx);
        } else {
            self.start(now, ctx);
        }
    }

    /// grid を作り直して先頭から再生する。
    ///
    /// 全 rest のままだと入った瞬間が無音になってしまうため、ここで一度ランダム化
    /// してからクロックを走らせる（`r` を押す前から演奏が始まっているのが仕様）。
    pub fn start(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        self.cancel_mouse_gesture();
        self.help_open = false;
        self.close_chord_input();
        self.close_cycle_random_overlay();
        self.patch_status = ctx.patch_status();
        // 入場時は何も送っていないので、引き直しで出る note off は捨ててよい。
        let _ = self.state.randomize_all(now, ctx.patches());
        self.absorb_drawn_phrases(self.state.drawn_phrases());
        // 無差別抽選のあとに、用途が決まっている行（drum）だけ当て直す。
        self.apply_assigned_patches(ctx, false);
        self.grid_ready = true;
        self.restored_patches_pending = false;
        self.cancel_cycle_swap();
        // クロックを走らせる前に自動BPMを引く。範囲を指定していれば、起動のたびに
        // 違うテンポで始まる。
        self.reseed_auto_bpm();
        self.state.start_at_bpm(now, self.bpm());
        log_line(&format!(
            "grid-sequencer: start instances={}",
            self.track_count()
        ));
        self.prepare_connection_or_start_server(ctx);
    }

    /// 直前の grid を保ったまま画面へ戻るときの初期化。
    pub fn resume(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        self.cancel_mouse_gesture();
        self.help_open = false;
        self.close_chord_input();
        self.close_cycle_random_overlay();
        self.cancel_cycle_swap();
        self.state.start_at_bpm(now, self.bpm());
        self.refresh_context(ctx);
        if !self.waiting_for_patches {
            self.prepare_connection_or_start_server(ctx);
        }
    }

    /// 画面を離れるときの後始末。開いている overlay を閉じてから再生を停止する。
    pub fn finish(&mut self) {
        self.cancel_mouse_gesture();
        self.help_open = false;
        self.close_chord_input();
        self.close_cycle_random_overlay();
        self.bpm_input = None;
        self.stop_playing();
    }

    /// 演奏だけを止める。鳴っている音を消してクロックを止めるが、grid の内容は
    /// 残るので [`Self::resume`] で続きから鳴らし直せる。
    ///
    /// 画面を離れるとき（[`Self::finish`]）と、`P` で明示的に止めたとき、MML
    /// オーバーレイへ音源を明け渡すときの3つで共有する。
    pub fn stop_playing(&mut self) {
        self.cancel_cycle_swap();
        self.waiting_for_patches = false;
        self.resume_at = None;
        // 止めたら判定は白紙。次に走らせるときはダブルバッファリングから試す
        // （送信ワーカー側の判定も `stop()` で戻る）。
        self.single_buffering = false;
        self.overload_applied = false;
        let _note_offs = self.state.take_reset_messages();
        if let Some(sender) = &self.midi_sender {
            sender.set_auto_gain_enabled(false);
            sender.set_gains(vec![
                0.0;
                self.state.instance_count()
                    * cmrt_realtime_play::BANK_COUNT
            ]);
            sender.stop();
        }
    }

    /// 演奏中か。
    pub fn is_playing(&self) -> bool {
        self.state.is_running()
    }

    /// 再生/停止をトグルする。止まっていれば直前の grid のまま鳴らし直す。
    ///
    /// DAW 画面の `P`（`TogglePlay`）と同じ操作感に揃えてある。
    fn toggle_playing(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        if self.is_playing() {
            self.stop_playing();
            log_line("grid-sequencer: stop-playing key=P");
        } else {
            self.resume(now, ctx);
            log_line("grid-sequencer: resume-playing key=P");
        }
    }

    pub fn connection_status(&self) -> GridConnectionStatus {
        self.midi_sender
            .as_ref()
            .map(GridMidiSender::status)
            .unwrap_or_default()
    }

    /// 鳴っている bank の音色を差し替え、ロードが終わるまで鳴らし始めを待たせる。
    ///
    /// `stop_live_all()` を伴うのでロード中は無音になる。待機 bank への先読み
    /// （[`GridSequencerScreen::advance_cycle_swap`]）とは別経路。
    fn prepare_connection(&mut self) {
        self.cancel_mouse_gesture();
        if let Some(sender) = &self.midi_sender {
            sender.prepare(self.state.patches());
            sender.set_auto_gain_enabled(true);
        }
        self.apply_chord_gains();
        self.wait_for_patches();
    }

    /// chord mode の和音を他の行より目立たせるための音量差を適用する。
    ///
    /// ゲインはサーバー側が instance ごとに保持し、音色ロードで live を作り直しても
    /// 残る。chord mode または NOTE の random が変わったときに送り直す。
    pub(crate) fn apply_chord_gains(&self) {
        let Some(sender) = &self.midi_sender else {
            return;
        };
        sender.set_gains(chord_gains_db(
            self.state.instance_count(),
            self.state.chord().is_some(),
            self.cycle_random.note,
        ));
    }

    fn prepare_connection_or_start_server(&mut self, ctx: &GridSequencerContext<'_>) {
        // 保存 patch は catalog が Ready になって存在確認できるまでロードしない。
        if self.restored_patches_pending {
            if ctx.patches_are_loading() {
                if let Some(sender) = &self.midi_sender {
                    sender.start_server();
                }
            }
            return;
        }
        if self.state.patches().all(|(_, patch)| patch.is_some()) {
            self.prepare_connection();
        } else if ctx.patches_are_loading() {
            if let Some(sender) = &self.midi_sender {
                sender.start_server();
            }
        }
    }

    /// 非同期の patch 一覧が Ready へ遷移したら、未設定instanceへ一度だけ割り当てて
    /// 有効な全 instance の patch prepare を開始する。
    ///
    /// セッションから復元した chord mode も、patch 一覧が揃うここで on にする。割り当てと
    /// 同じ prepare に相乗りさせるので、音色ロードは1回で済む。
    pub fn refresh_context(&mut self, ctx: &GridSequencerContext<'_>) {
        self.patch_status = ctx.patch_status();
        if ctx.chord_source_updated && self.restart_notice.is_none() {
            self.restart_notice = Some(Instant::now());
            log_line("grid-sequencer: chord-source updated, restarting");
        }
        let restored_validated = if ctx.patches_are_ready() {
            self.validate_restored_patches(ctx.patches())
        } else {
            false
        };
        let chord_restored = self.poll_pending_chord(Instant::now(), ctx);
        // 用途が決まっている行は無差別抽選より先に埋める。あとから埋めると
        // drum 行に打楽器でない音色が残る。
        let by_role = if ctx.patches_are_ready() {
            self.apply_assigned_patches(ctx, true)
        } else {
            0
        };
        let assigned = self.state.fill_missing_patches(ctx.patches());
        if assigned == 0 && by_role == 0 && !chord_restored && !restored_validated {
            return;
        }
        log_line(&format!(
            "grid-sequencer: patch-list-ready assigned={assigned} by_role={by_role} \
             chord_restored={chord_restored} instances={}",
            self.track_count()
        ));
        self.prepare_connection_or_start_server(ctx);
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) -> GridSequencerAction {
        if key.kind != KeyEventKind::Press {
            return GridSequencerAction::Continue;
        }
        if self.bpm_input.is_some() {
            self.handle_bpm_input_key(key, now);
            return GridSequencerAction::Continue;
        }
        if self.chord_input.is_some() {
            self.handle_chord_input_key(key, now, ctx);
            return GridSequencerAction::Continue;
        }
        if self.cycle_random_open() {
            self.handle_cycle_random_key(key);
            return GridSequencerAction::Continue;
        }
        if self.patch_selector.is_some() {
            if self.patch_selector_input_enabled() {
                self.handle_patch_selector_key(key, ctx);
            } else {
                self.cancel_mouse_gesture();
            }
            return GridSequencerAction::Continue;
        }
        if self.help_open {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
            ) {
                self.help_open = false;
            }
            return GridSequencerAction::Continue;
        }
        if key.modifiers == crossterm::event::KeyModifiers::CONTROL
            && key.code == KeyCode::Char('b')
        {
            self.cancel_mouse_gesture();
            self.bpm_input = Some(cmrt_tui_core::bpm::BpmInput::default());
            return GridSequencerAction::Continue;
        }
        match key.code {
            KeyCode::Char('q') => return GridSequencerAction::Quit,
            KeyCode::Char('t') => {
                let next = cmrt_realtime_play::next_live_instance_count(self.track_count());
                self.resize_for_restart(next, ctx.patches());
                return GridSequencerAction::RestartWithTrackCount(next);
            }
            KeyCode::Char('?') => {
                self.cancel_mouse_gesture();
                self.help_open = true;
                cmrt_tui_core::memory::request_refresh();
            }
            KeyCode::Char('P') => self.toggle_playing(now, ctx),
            KeyCode::Char('a') => self.toggle_cycle_random_overlay(),
            KeyCode::Char('b') if key.modifiers.is_empty() => self.toggle_single_buffering(),
            KeyCode::Char('c') => self.toggle_chord_mode(now, ctx),
            KeyCode::Char('i') => self.open_chord_input(),
            KeyCode::Char('r') => self.randomize(now, ctx),
            KeyCode::Char('R') => self.randomize_keeping_patches(now, ctx),
            KeyCode::Char('u') => self.undo(ctx),
            KeyCode::Char('x') => self.clear_notes(),
            _ => {}
        }
        GridSequencerAction::Continue
    }

    /// grid を丸ごと引き直し、全 instance の patch を差し替える。
    fn randomize(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        let undo = self.capture_undo();
        self.patch_status = ctx.patch_status();
        // 全 instance を差し替えるので、走っている先読みは意味を失う。
        self.cancel_cycle_swap();
        let _note_offs = self.state.randomize_all(now, ctx.patches());
        self.absorb_drawn_phrases(self.state.drawn_phrases());
        // chord mode 中は和音の行だけ poly patch を当て直す（無差別抽選で mono を
        // 引くと和音が潰れるため）。
        self.rechord_after_randomize(now, ctx, true);
        log_line(&format!(
            "grid-sequencer: randomize instances={}",
            self.track_count()
        ));
        self.prepare_connection();
        self.commit_undo(undo);
    }

    /// patch を据え置き、note / patternだけを引き直す。
    ///
    /// 音色ロード（`sender.prepare()`）を走らせないので再生が途切れない。その代わり
    /// `prepare_instances()` の `stop_live_all()` による消音も無いため、鳴っていた音の
    /// note off はここで自分で送る必要がある。
    fn randomize_keeping_patches(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        let undo = self.capture_undo();
        // 譜面が変わるので、抽選済みの次サイクルは古くなる。走っている先読みごと捨てる。
        self.cancel_cycle_swap_preserving_drain();
        let note_offs = self.state.randomize_keeping_patches(now);
        self.absorb_drawn_phrases(self.state.drawn_phrases());
        log_line(&format!(
            "grid-sequencer: randomize-keep-patch instances={} note_offs={}",
            self.track_count(),
            note_offs.len(),
        ));
        self.send_scheduled(&note_offs);
        self.rechord_after_randomize(now, ctx, false);
        self.commit_undo(undo);
    }

    fn clear_notes(&mut self) {
        let undo = self.capture_undo();
        // `x` は白紙の pattern を保持するという明示操作なので、すでに空でも
        // NOTE の引き直しを止める。
        self.begin_manual_edit(CycleRandomItem::Note);
        self.state.clear_notes();
        self.commit_undo(undo);
    }
}

#[cfg(test)]
mod tests;
