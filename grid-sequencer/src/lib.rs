//! grid sequencer 画面（16行 x 16ステップの grid を常時ループ再生する）。
//!
//! 画面の状態・入力・描画・MIDI 送信はこの crate に閉じており、共有ランタイム
//! （app 側の `TuiApp`）からは `GridSequencerContext` で必要な情報を注入してもらう。
//! app 側との接続は app crate の `tui::grid_sequencer_glue` にある。
//!
//! grid のrow 0〜15を realtime play server のCLAP instance 0〜15へ対応付ける。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

mod screen;
mod screen_runtime;
mod sender;
mod state;
pub mod ui;

pub use screen::GridSequencerScreen;
pub use sender::{
    GridConnectionPhase, GridConnectionStatus, GridMidiSender, GridProgress, GridRowReadiness,
};
pub use state::{
    frames_ahead, step_offset, GridRow, GridScheduledMessage, GridState, StepDuration, BPM,
    GRID_ROWS, GRID_STEPS, LOOKAHEAD, STEPS_PER_BEAT, STEP_INTERVAL,
};

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

/// 画面が返す、共有ランタイム側で処理すべき遷移要求。
pub enum GridSequencerAction {
    Continue,
    Quit,
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
        self.state.start(now);
        log_line(&format!("grid-sequencer: start instances={GRID_ROWS}"));
        self.prepare_connection_or_start_server(ctx);
    }

    /// 直前の grid を保ったまま画面へ戻るときの初期化。
    pub fn resume(&mut self, now: Instant) {
        self.help_open = false;
        self.state.start(now);
        self.prepare_connection();
    }

    /// 画面を離れるときの後始末。鳴っている音を止めてから再生を停止する。
    pub fn finish(&mut self) {
        self.help_open = false;
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

    fn prepare_connection(&self) {
        if let Some(sender) = &self.midi_sender {
            sender.prepare(self.state.patches());
        }
    }

    fn prepare_connection_or_start_server(&self, ctx: &GridSequencerContext<'_>) {
        if self.state.patches().all(|patch| patch.is_some()) {
            self.prepare_connection();
        } else if ctx.patches_are_loading() {
            if let Some(sender) = &self.midi_sender {
                sender.start_server();
            }
        }
    }

    /// 非同期の patch 一覧が Ready へ遷移したら、未設定 row へ一度だけ割り当てて
    /// 16 instance の patch prepare を開始する。
    pub fn refresh_context(&mut self, ctx: &GridSequencerContext<'_>) {
        self.patch_status = ctx.patch_status();
        let assigned = self.state.fill_missing_patches(ctx.patches());
        if assigned == 0 {
            return;
        }
        log_line(&format!(
            "grid-sequencer: patch-list-ready assigned={assigned} instances={GRID_ROWS}"
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
            KeyCode::Char('?') => self.help_open = true,
            KeyCode::Char('r') => self.randomize(now, ctx),
            KeyCode::Char('R') => self.randomize_keeping_patches(now),
            _ => {}
        }
        GridSequencerAction::Continue
    }

    /// grid を丸ごと引き直し、16 instanceすべてのpatchを差し替える。
    fn randomize(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        self.patch_status = ctx.patch_status();
        let _note_offs = self.state.randomize_all(now, ctx.patches());
        log_line(&format!("grid-sequencer: randomize instances={GRID_ROWS}"));
        if let Some(sender) = &self.midi_sender {
            sender.prepare(self.state.patches());
        }
    }

    /// patch を据え置き、note / 音長 / セルだけを引き直す。
    ///
    /// 音色ロード（`sender.prepare()`）を走らせないので再生が途切れない。その代わり
    /// `prepare_instances()` の `stop_live_all()` による消音も無いため、鳴っていた音の
    /// note off はここで自分で送る必要がある。
    fn randomize_keeping_patches(&mut self, now: Instant) {
        let note_offs = self.state.randomize_keeping_patches(now);
        log_line(&format!(
            "grid-sequencer: randomize-keep-patch rows={GRID_ROWS} note_offs={}",
            note_offs.len()
        ));
        self.send_scheduled(&note_offs);
    }
}

#[cfg(test)]
mod tests;
