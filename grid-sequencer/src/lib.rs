//! grid sequencer 画面（1/2/4/8/16行 x 16ステップの grid を常時ループ再生する）。
//!
//! 画面の状態・入力・描画・MIDI 送信はこの crate に閉じており、共有ランタイム
//! （app 側の `TuiApp`）からは `GridSequencerContext` で必要な情報を注入してもらう。
//! app 側との接続は app crate の `tui::grid_sequencer_glue` にある。
//!
//! grid の各rowを realtime play server の同番号CLAP instanceへ対応付ける。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use cmrt_chord::ChordProgressionCatalog;
use cmrt_realtime_play::PatchVoicing;

mod chord_mode;
mod cycle_swap;
mod screen;
mod screen_runtime;
mod sender;
mod start_wait;
mod state;
pub mod ui;

pub use screen::GridSequencerScreen;
pub use sender::{
    GridConnectionPhase, GridConnectionStatus, GridMidiSender, GridProgress, GridRowReadiness,
};
pub use state::{
    frames_ahead, pick_chord_patch, randomize_row_slice, snap_rows_to_chord, step_offset,
    ChordPlayback, GridRow, GridScheduledMessage, GridState, StepDuration, BPM, CHORD_ROW,
    GRID_ROWS, GRID_STEPS, LOOKAHEAD, STEPS_PER_BEAT, STEP_INTERVAL,
};

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

/// chord mode の和音の行へ与える音量差（dB）。他の行は 0 dB のまま。
pub const CHORD_GAIN_DB: f32 = 6.0;

/// instance ごとの音量差（dB）。chord mode 中だけ和音の行が持ち上がる。
///
/// 返す長さは bank 2 本ぶん（= `row_count * BANK_COUNT`）。差し替え先の bank にも
/// 同じ音量差を載せておく必要があるので、両方の `CHORD_ROW` を持ち上げる。
pub fn chord_gains_db(row_count: usize, chord_on: bool) -> Vec<f32> {
    (0..row_count * cmrt_realtime_play::BANK_COUNT)
        .map(|instance| {
            if chord_on && instance % row_count == CHORD_ROW {
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

/// patch 一覧のバックグラウンド読み込み状態のスナップショット。
/// 共有ランタイム側の `PatchLoadState` から glue が変換して渡す。
pub enum GridPatchLoad<'a> {
    Loading,
    Ready(&'a [(String, String)]),
    Err(&'a str),
}

/// grid sequencer 画面が共有ランタイムから受け取る情報一式。
pub struct GridSequencerContext<'a> {
    pub patch_dirs_configured: bool,
    pub patch_load: GridPatchLoad<'a>,
    /// chord mode が進行を抽選するカタログ。空なら chord mode は開始できない。
    pub chord_catalog: &'a ChordProgressionCatalog,
    /// 和音用 patch の当たり判定に使う mono/poly 判定。
    pub voicing: &'a dyn GridVoicingLookup,
    /// 和音に使う patch のカテゴリ（config.toml の `chord_patch_categories`）。
    /// 空ならカテゴリでは絞らない。
    pub chord_patch_categories: &'a [String],
    /// コード進行カタログが更新されたか（再起動アナウンスの合図。一度だけ true）。
    pub chord_source_updated: bool,
}

impl GridSequencerContext<'_> {
    /// ランダム選択に使える patch 一覧。読み込み中・エラー時は空を返す。
    fn patches(&self) -> &[(String, String)] {
        match &self.patch_load {
            GridPatchLoad::Ready(pairs) => pairs,
            GridPatchLoad::Loading | GridPatchLoad::Err(_) => &[],
        }
    }

    fn patch_status(&self) -> GridPatchStatus {
        if !self.patch_dirs_configured {
            return GridPatchStatus::NotConfigured;
        }
        match &self.patch_load {
            GridPatchLoad::Ready(pairs) => GridPatchStatus::Ready(pairs.len()),
            GridPatchLoad::Loading => GridPatchStatus::Loading,
            GridPatchLoad::Err(error) => GridPatchStatus::Err((*error).to_string()),
        }
    }

    fn patches_are_loading(&self) -> bool {
        matches!(self.patch_load, GridPatchLoad::Loading)
    }
}

/// ステータス行に出す patch 一覧の状態（直近のランダム化時点のスナップショット）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GridPatchStatus {
    #[default]
    Loading,
    Ready(usize),
    NotConfigured,
    Err(String),
}

impl GridPatchStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Loading => "patches loading".to_string(),
            Self::Ready(count) => format!("{count} patches"),
            Self::NotConfigured => "patches_dirs 未設定".to_string(),
            Self::Err(error) => format!("patches error: {error}"),
        }
    }
}

impl GridSequencerScreen {
    /// 画面へ入る。初回はランダムな grid を作り、2回目以降は前回の grid を続ける。
    pub fn enter(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        if self.grid_ready {
            self.resume(now);
        } else {
            self.start(now, ctx);
        }
    }

    /// grid を作り直して先頭から再生する。
    ///
    /// 全 rest のままだと入った瞬間が無音になってしまうため、ここで一度ランダム化
    /// してからクロックを走らせる（`r` を押す前から演奏が始まっているのが仕様）。
    pub fn start(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        self.help_open = false;
        self.patch_status = ctx.patch_status();
        // 入場時は何も送っていないので、引き直しで出る note off は捨ててよい。
        let _ = self.state.randomize_all(now, ctx.patches());
        self.grid_ready = true;
        self.cancel_cycle_swap();
        self.state.start(now);
        log_line(&format!(
            "grid-sequencer: start instances={}",
            self.track_count()
        ));
        self.prepare_connection_or_start_server(ctx);
    }

    /// 直前の grid を保ったまま画面へ戻るときの初期化。
    pub fn resume(&mut self, now: Instant) {
        self.help_open = false;
        self.cancel_cycle_swap();
        self.state.start(now);
        self.prepare_connection();
    }

    /// 画面を離れるときの後始末。鳴っている音を止めてから再生を停止する。
    pub fn finish(&mut self) {
        self.help_open = false;
        self.cancel_cycle_swap();
        self.waiting_for_patches = false;
        self.resume_at = None;
        let _note_offs = self.state.take_reset_messages();
        if let Some(sender) = &self.midi_sender {
            sender.stop();
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
        if let Some(sender) = &self.midi_sender {
            sender.prepare(self.state.patches());
        }
        self.apply_chord_gains();
        self.wait_for_patches();
    }

    /// chord mode の和音を他の行より目立たせるための音量差を適用する。
    ///
    /// ゲインはサーバー側が instance ごとに保持し、音色ロードで live を作り直しても
    /// 残るため、chord mode の on/off が変わったときだけ送ればよい。
    pub(crate) fn apply_chord_gains(&self) {
        let Some(sender) = &self.midi_sender else {
            return;
        };
        sender.set_gains(chord_gains_db(
            self.state.row_count(),
            self.state.chord().is_some(),
        ));
    }

    fn prepare_connection_or_start_server(&mut self, ctx: &GridSequencerContext<'_>) {
        if self.state.patches().all(|(_, patch)| patch.is_some()) {
            self.prepare_connection();
        } else if ctx.patches_are_loading() {
            if let Some(sender) = &self.midi_sender {
                sender.start_server();
            }
        }
    }

    /// 非同期の patch 一覧が Ready へ遷移したら、未設定 row へ一度だけ割り当てて
    /// 有効な全 instance の patch prepare を開始する。
    pub fn refresh_context(&mut self, ctx: &GridSequencerContext<'_>) {
        self.patch_status = ctx.patch_status();
        if ctx.chord_source_updated && self.restart_notice.is_none() {
            self.restart_notice = Some(Instant::now());
            log_line("grid-sequencer: chord-source updated, restarting");
        }
        let assigned = self.state.fill_missing_patches(ctx.patches());
        if assigned == 0 {
            return;
        }
        log_line(&format!(
            "grid-sequencer: patch-list-ready assigned={assigned} instances={}",
            self.track_count()
        ));
        self.prepare_connection();
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
        if self.help_open {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
            ) {
                self.help_open = false;
            }
            return GridSequencerAction::Continue;
        }
        match key.code {
            KeyCode::Char('q') => return GridSequencerAction::Quit,
            KeyCode::Char('t') => {
                let next = cmrt_realtime_play::next_live_instance_count(self.track_count());
                self.finish();
                self.state = GridState::with_row_count(next);
                return GridSequencerAction::RestartWithTrackCount(next);
            }
            KeyCode::Char('?') => {
                self.help_open = true;
                cmrt_tui_core::memory::request_refresh();
            }
            KeyCode::Char('c') => self.toggle_chord_mode(now, ctx),
            KeyCode::Char('r') => self.randomize(now, ctx),
            KeyCode::Char('R') => self.randomize_keeping_patches(now, ctx),
            _ => {}
        }
        GridSequencerAction::Continue
    }

    /// grid を丸ごと引き直し、全 instance の patch を差し替える。
    fn randomize(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        self.patch_status = ctx.patch_status();
        // 全 instance を差し替えるので、走っている先読みは意味を失う。
        self.cancel_cycle_swap();
        let _note_offs = self.state.randomize_all(now, ctx.patches());
        // chord mode 中は和音の行だけ poly patch を当て直す（無差別抽選で mono を
        // 引くと和音が潰れるため）。
        self.rechord_after_randomize(now, ctx, true);
        log_line(&format!(
            "grid-sequencer: randomize instances={}",
            self.track_count()
        ));
        self.prepare_connection();
    }

    /// patch を据え置き、note / 音長 / セルだけを引き直す。
    ///
    /// 音色ロード（`sender.prepare()`）を走らせないので再生が途切れない。その代わり
    /// `prepare_instances()` の `stop_live_all()` による消音も無いため、鳴っていた音の
    /// note off はここで自分で送る必要がある。
    fn randomize_keeping_patches(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        // 譜面が変わるので、抽選済みの次サイクルは古くなる。走っている先読みごと捨てる。
        self.cancel_cycle_swap();
        let note_offs = self.state.randomize_keeping_patches(now);
        log_line(&format!(
            "grid-sequencer: randomize-keep-patch rows={} note_offs={}",
            self.track_count(),
            note_offs.len(),
        ));
        self.send_scheduled(&note_offs);
        self.rechord_after_randomize(now, ctx, false);
    }
}

#[cfg(test)]
mod tests;
