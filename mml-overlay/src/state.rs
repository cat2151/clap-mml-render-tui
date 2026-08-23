//! MML 入力オーバーレイの状態と、打鍵から発音までの判定。
//!
//! **ここは「何を鳴らしてほしいか」しか言わない。** 音源で何が鳴っているかも、
//! それをどう止めるかも持たない（`sender` 側の [`crate::sender`] に 1 つだけある）。
//! かつては打鍵の note off をここで組み立てて [`MmlOverlayAction::Send`] へ混ぜて
//! いたが、行を鳴らす経路だけが「止めるのは行演奏側の仕事」として note off を
//! 出さずに記録を捨てており、移動先が空行だとサーバーへ何も飛ばずに鳴りっぱなしに
//! なった。止める役と gate の計時は sender worker へ寄せ、ここが持つ
//! [`MmlOverlay::sounding`] は表示専用とする。

mod history;
mod patch;

use std::time::{Duration, Instant};

use cmrt_chord::TimedMidiEvent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::{DataCursor, TextArea};

use crate::cursor_notes::{notes_at_cursor, CursorNotes};
use crate::history_select::{is_history_select_trigger, HistorySelect};
use crate::line_play::{line_events, LineStatus};
use crate::patch_select::{is_patch_select_trigger, PatchSelect};
use crate::MmlOverlaySenderStatus;
use crate::NOTE_ON;

/// 生 MIDI の note on と、送信成功後に保つべき音長。
#[derive(Clone, Debug, PartialEq)]
pub struct NoteRequest {
    pub messages: Vec<[u8; 3]>,
    pub duration: Duration,
}

/// 行を鳴らす前に音源の音色を差し替えるか。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchChange {
    /// いまの音色のまま鳴らす。
    Keep,
    /// 鳴らす前にこの音色へ差し替える（`None` は realtime server の既定音色へ戻す）。
    Switch(Option<String>),
}

/// オーバーレイが呼び出し側へ求める処理。
///
/// 音を出す変種（[`Self::Send`] / [`Self::SetPatch`] / [`Self::PlayLine`]）は、
/// どれも「鳴っているものを止めてから鳴らす」の意味になる。止める指示は載せない。
#[derive(Clone, Debug, PartialEq)]
pub enum MmlOverlayAction {
    Continue,
    /// 鳴っているものを止めてから、この note on を送る。
    Send(NoteRequest),
    /// 鳴っているものを止め、音源の音色を差し替えてから、この note on を送る。
    /// `patch` が `None` なら realtime server の既定音色へ戻す。
    SetPatch {
        patch: Option<String>,
        notes: Option<NoteRequest>,
    },
    /// 鳴っているものを止め、あらためてこの行を頭から積む。
    /// `events` が空なら止めるだけ。
    PlayLine {
        patch: PatchChange,
        events: Vec<TimedMidiEvent>,
    },
    /// オーバーレイを閉じる。鳴っているものを止めるのも含む。
    Close,
}

/// MML overlay が受け取る、plugin 非依存の音色一覧スナップショット。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PatchCatalogSnapshot {
    /// バックグラウンド収集中。Ctrl+T は完了後の open 予約になる。
    #[default]
    Loading,
    /// 収集済みの（表示名, 小文字化済み表示名）。空なら選べる音色がない。
    Ready(Vec<(String, String)>),
    /// 収集に失敗した理由。Ctrl+T 時に overlay 内へ表示する。
    Error(String),
}

/// オーバーレイを開くときに呼び出し側から渡すスナップショット。
#[derive(Default)]
pub struct MmlOverlayContext {
    pub patch_catalog: PatchCatalogSnapshot,
    /// notepad 画面と共有しているフレーズ履歴。
    pub history: Vec<String>,
    pub favorites: Vec<String>,
    /// 設定不足でカタログから外れたプラグインの案内（`SkippedCatalogPlugin::notice_line`）。
    ///
    /// 「音色一覧に出てこない」は一覧を見ているだけでは絶対に気づけない
    /// （**出ていないものは見えない**）ので、音色選択を開いている間だけ枠の下へ出す。
    /// 空なら 1 行も増えない。
    pub catalog_notes: Vec<String>,
}

/// どの画面からでも開ける MML 入力オーバーレイ。
///
/// 入力欄は複数行で、1 行が 1 フレーズ。カーソルが別の行へ移るたびにその行を
/// まるごと鳴らすので、書き並べたフレーズを上下キーだけで聴き比べられる。
///
/// MML そのものは揮発でよいため、閉じると捨てる。音色だけは [`MmlOverlay::patch`] に
/// 残し、呼び出し側がセッションへ保存する。
pub struct MmlOverlay<'a> {
    open: bool,
    textarea: TextArea<'a>,
    /// 直近に打鍵で鳴らした発音単位。これと違う単位になった瞬間だけ発音する。
    /// 行をまたいだら別の音として扱うため、行番号も同一性に含める。
    last_notes: Option<(usize, CursorNotes)>,
    /// 打鍵で鳴らした note number。表示だけに使う。
    sounding: Vec<u8>,
    /// 打鍵で鳴らした音が chord 表記から来たか。表示だけに使う。
    sounding_from_chord: bool,
    /// senderへ最後に依頼したcommand。古いworker状態で表示を巻き戻さないための世代。
    sender_command_id: u64,
    /// 入力欄とは別に持つ音色。`Ctrl+T` と履歴の取り込みだけが書き換える。
    patch: Option<String>,
    /// 開いている間だけ持つ patch 一覧のスナップショット（表示名, 小文字化）。
    patch_catalog: PatchCatalogSnapshot,
    /// 開いている間だけ持つフレーズ履歴のスナップショット。
    history: Vec<String>,
    favorites: Vec<String>,
    /// 開いている間だけ持つ「カタログから外れたプラグイン」の案内。
    catalog_notes: Vec<String>,
    /// Ctrl+T を処理できなかった理由。標準 stream ではなく overlay 内へ出す。
    patch_catalog_notice: Option<PatchCatalogNotice>,
    /// Loading 中の Ctrl+T を、一覧完成後に自動で実行する予約。
    patch_select_requested: bool,
    patch_select: Option<PatchSelect<'a>>,
    history_select: Option<HistorySelect<'a>>,
    /// 直近に行を演奏した結果。
    line_status: LineStatus,
}

impl Default for MmlOverlay<'_> {
    fn default() -> Self {
        Self {
            open: false,
            textarea: cmrt_tui_core::text_input::new_multi_line_textarea(Vec::new()),
            last_notes: None,
            sounding: Vec::new(),
            sounding_from_chord: false,
            sender_command_id: 0,
            patch: None,
            patch_catalog: PatchCatalogSnapshot::Loading,
            history: Vec::new(),
            favorites: Vec::new(),
            catalog_notes: Vec::new(),
            patch_catalog_notice: None,
            patch_select_requested: false,
            patch_select: None,
            history_select: None,
            line_status: LineStatus::Idle,
        }
    }
}

impl<'a> MmlOverlay<'a> {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn textarea(&self) -> &TextArea<'a> {
        &self.textarea
    }

    /// 入力欄の中身すべて。行は改行で繋ぐ。
    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// いま鳴っている音（表示用）。
    pub fn sounding(&self) -> &[u8] {
        &self.sounding
    }

    /// いま鳴っている音が chord 表記から来たか（表示用）。
    pub fn sounding_from_chord(&self) -> bool {
        self.sounding_from_chord
    }

    /// いまの音色。セッション保存はこれを見る。
    pub fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }

    pub fn line_status(&self) -> &LineStatus {
        &self.line_status
    }

    /// セッションから復元した音色を入れる。起動時に1度だけ呼ぶ。
    pub fn set_restored_patch(&mut self, patch: Option<String>) {
        self.patch = patch;
    }

    pub(crate) fn patch_select(&self) -> Option<&PatchSelect<'a>> {
        self.patch_select.as_ref()
    }

    pub(crate) fn history_select(&self) -> Option<&HistorySelect<'a>> {
        self.history_select.as_ref()
    }

    /// 前回の音色だけを引き継いだ、空の入力欄で開く。
    pub fn open(&mut self, context: MmlOverlayContext) {
        self.textarea = cmrt_tui_core::text_input::new_multi_line_textarea(Vec::new());
        self.last_notes = None;
        self.sounding.clear();
        self.sounding_from_chord = false;
        self.sender_command_id = 0;
        self.line_status = LineStatus::Idle;
        self.patch_catalog = context.patch_catalog;
        self.history = context.history;
        self.favorites = context.favorites;
        self.catalog_notes = context.catalog_notes;
        self.patch_catalog_notice = None;
        self.patch_select_requested = false;
        self.patch_select = None;
        self.history_select = None;
        self.open = true;
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) -> MmlOverlayAction {
        if self.patch_select.is_some() {
            return self.handle_patch_select_key(key, now);
        }
        if self.history_select.is_some() {
            return self.handle_history_select_key(key);
        }
        if key.code == KeyCode::Esc {
            return self.close();
        }
        if is_patch_select_trigger(key) {
            self.open_patch_select();
            return MmlOverlayAction::Continue;
        }
        if is_history_select_trigger(key) {
            self.open_history_select();
            return MmlOverlayAction::Continue;
        }
        if is_replay_key(key) {
            return self.play_current_line(PatchChange::Keep);
        }
        let DataCursor(previous_row, _) = self.textarea.cursor();
        self.textarea.input(key);
        // 行が変われば、その行をまるごと鳴らす。上下キーだけでなく、改行や
        // 行頭での backspace でも同じ扱いになる。
        if self.cursor_row() != previous_row {
            return self.play_current_line(PatchChange::Keep);
        }
        self.refresh(now)
    }

    /// 呼び出し側がsenderへ積んだ最新commandを記録する。
    pub fn expect_sender_command(&mut self, command_id: u64) {
        self.sender_command_id = command_id;
    }

    /// workerが実際に到達した発音状態を表示へ反映する。
    pub fn sync_sender_status(&mut self, status: &MmlOverlaySenderStatus) {
        if status.command_id() < self.sender_command_id {
            return;
        }
        self.sender_command_id = status.command_id();
        self.sounding.clear();
        self.sounding.extend_from_slice(status.sounding());
    }

    fn close(&mut self) -> MmlOverlayAction {
        self.patch_catalog = PatchCatalogSnapshot::Loading;
        self.history = Vec::new();
        self.favorites = Vec::new();
        self.patch_select = None;
        self.patch_catalog_notice = None;
        self.patch_select_requested = false;
        self.history_select = None;
        self.open = false;
        self.forget_sounding();
        MmlOverlayAction::Close
    }

    /// カーソルのある行をまるごと鳴らす。
    ///
    /// 打鍵の 1 音は行の演奏に飲み込まれるので、その記録は落とす。ここで note off を
    /// 組み立てないのは、[`MmlOverlayAction::PlayLine`] 自体が「鳴っているものを
    /// 止めてから積む」の意味だから。止めるのは受け取る側の 1 か所だけが行う。
    fn play_current_line(&mut self, patch: PatchChange) -> MmlOverlayAction {
        let (status, events) = line_events(self.current_line());
        self.line_status = status;
        self.forget_cursor_unit();
        MmlOverlayAction::PlayLine { patch, events }
    }

    /// カーソルのある発音単位を調べ、直前と別の単位になっていれば鳴らす。
    ///
    /// 文字を打ったときもカーソルを動かしたときも同じ判定を通るので、
    /// 「← で戻ったらそこの音がまた鳴る」が特別扱いなしに成り立つ。同じ単位の
    /// 内側で動くあいだは鳴らし直さないため、和音 `'ceg'` の中をカーソルが
    /// 通っても 1 回しか鳴らない。単位が伸びれば別の単位なので、`c` に続けて
    /// `1` を打てば全音符で鳴り直す。
    ///
    /// 休符やコマンドの上には鳴らす単位が無い。鳴っている音は gate に任せる。
    fn refresh(&mut self, _now: Instant) -> MmlOverlayAction {
        let notes = self.notes_at_cursor();
        if notes == self.last_notes {
            return MmlOverlayAction::Continue;
        }
        self.last_notes.clone_from(&notes);
        let Some((_, notes)) = notes else {
            return MmlOverlayAction::Continue;
        };
        MmlOverlayAction::Send(self.start_notes(&notes))
    }

    fn notes_at_cursor(&self) -> Option<(usize, CursorNotes)> {
        let DataCursor(row, column) = self.textarea.cursor();
        notes_at_cursor(self.current_line(), column).map(|notes| (row, notes))
    }

    /// この発音単位の note on。前の音を止めるのは受け取る側の仕事。
    fn start_notes(&mut self, notes: &CursorNotes) -> NoteRequest {
        // gate の長さは MML の音長そのもの。「打鍵をやめても鳴り続ける」を追うには、
        // 何 ms 先に止める約束をしたのかが残っていないと判断できない。
        crate::log_line(format!(
            "action=mml-overlay-note-on pitches={:?} gate_ms={}",
            notes.pitches,
            notes.duration.as_millis()
        ));
        self.sounding.clone_from(&notes.pitches);
        self.sounding_from_chord = notes.from_chord;
        let messages = notes
            .pitches
            .iter()
            .map(|pitch| [NOTE_ON, *pitch, notes.velocity])
            .collect();
        NoteRequest {
            messages,
            duration: notes.duration,
        }
    }

    fn current_line(&self) -> &str {
        self.textarea
            .lines()
            .get(self.cursor_row())
            .map_or("", String::as_str)
    }

    fn cursor_row(&self) -> usize {
        let DataCursor(row, _) = self.textarea.cursor();
        row
    }

    /// カーソル同一性の記録ごと捨てる。行の演奏で打鍵の音が飲み込まれるときに使う。
    /// 同一性を残すと、行内へカーソルが戻ったときに同じ音が鳴り直さない。
    pub(super) fn forget_cursor_unit(&mut self) {
        self.last_notes = None;
        self.forget_sounding();
    }

    /// 表示の記録を捨てる。
    ///
    /// **ここは音を止めない。** 音を止めるのは [`MmlOverlayAction`] を受け取った側で、
    /// この関数を呼ぶ経路は必ず `Close` / `Send` / `PlayLine` のどれかを返す。
    fn forget_sounding(&mut self) {
        self.sounding.clear();
        self.sounding_from_chord = false;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PatchCatalogNotice {
    Loading,
    Empty,
    Error(String),
}

/// このキーはカーソルのある行をもう一度鳴らす。
///
/// 行が変わったときは自動で鳴るが、同じ行を鳴らし直す手段が別に要る。
/// `Ctrl+Space` は端末によって `Char(' ')` と `Char('\0')` のどちらでも届く。
fn is_replay_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(' ') | KeyCode::Char('\0'))
}

#[cfg(test)]
mod tests;
