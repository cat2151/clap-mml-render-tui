//! MML 入力オーバーレイの状態と、打鍵から発音までの判定。
//!
//! **ここは「何を鳴らしてほしいか」しか言わない。** 音源で何が鳴っているかも、
//! それをどう止めるかも持たない（`sender` 側の [`crate::sender`] に 1 つだけある）。
//! かつては打鍵の note off をここで組み立てて [`MmlOverlayAction::Send`] へ混ぜて
//! いたが、行を鳴らす経路だけが「止めるのは行演奏側の仕事」として note off を
//! 出さずに記録を捨てており、移動先が空行だとサーバーへ何も飛ばずに鳴りっぱなしに
//! なった。止める役と gate の計時は sender worker へ寄せ、ここが持つ
//! [`MmlOverlay::sounding`] は表示専用とする。

mod chord_transfer;
mod contract;
mod history;
mod patch;
mod play_settings;
mod single_line;

use std::{collections::BTreeMap, time::Instant};

use cmrt_patches::PatchRoleIndex;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::{DataCursor, TextArea};

use cmrt_tui_core::patch_load::PatchLoadMeasurement;

use crate::chord_transfer::ChordTransferConfirm;
use crate::cursor_notes::{notes_at_cursor, CursorNotes};
use crate::history_select::{is_history_select_trigger, HistorySelect};
use crate::line_play::{is_replay_key, line_events, LineStatus};
use crate::patch_select::{is_patch_select_trigger, PatchSelect};
use crate::play_settings::{PlaySettings, PlaySettingsSelect};
use crate::MmlOverlaySenderStatus;
use crate::NOTE_ON;

pub use contract::{
    MmlOverlayAction, MmlOverlayContext, MmlOverlayInputMode, NoteRequest, PatchCatalogSnapshot,
    PatchChange,
};

/// どの画面からでも開ける MML 入力オーバーレイ。
///
/// 入力欄は複数行で、1 行が 1 フレーズ。カーソルが別の行へ移るたびにその行を
/// まるごと鳴らすので、書き並べたフレーズを上下キーだけで聴き比べられる。
///
/// MML そのものは揮発でよいため、閉じると捨てる。音色だけは [`MmlOverlay::patch`] に
/// 残し、呼び出し側がセッションへ保存する。
pub struct MmlOverlay<'a> {
    open: bool,
    /// 入力欄が 1 行か複数行か。`Enter` / `Esc` の意味がこれで変わる。
    input_mode: MmlOverlayInputMode,
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
    patch_role_index: PatchRoleIndex,
    /// patch selectのLoad列へ渡す、開いているcatalogと同世代の計測結果。
    load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    /// 開いている間だけ持つフレーズ履歴のスナップショット。
    history: Vec<String>,
    favorites: Vec<String>,
    /// 開いている間だけ持つユーザー追加の patch filter preset。
    patch_filter_presets: Vec<(String, String)>,
    /// 開いている間だけ持つ「カタログから外れたプラグイン」の案内。
    catalog_notes: Vec<String>,
    /// Ctrl+T を処理できなかった理由。標準 stream ではなく overlay 内へ出す。
    patch_catalog_notice: Option<PatchCatalogNotice>,
    /// Loading 中の Ctrl+T を、一覧完成後に自動で実行する予約。
    patch_select_requested: bool,
    patch_select: Option<PatchSelect<'a>>,
    history_select: Option<HistorySelect<'a>>,
    /// `Ctrl+L` で決める、この overlay 全体で共通の演奏設定。開き直しでは消えない
    /// （音色と同じく、呼び出し側がセッションへ保存する）。
    play_settings: PlaySettings,
    play_settings_select: Option<PlaySettingsSelect>,
    /// 直近に行を演奏した結果。
    line_status: LineStatus,
    /// 打ちかけの 1 行を chord 行へ移せる画面か。開くときに呼び出し側が決める。
    chord_row_transfer: bool,
    /// いまの 1 行がコード表記として読めるか。ヒントの表示だけに使う。
    chord_hint: bool,
    /// 確定の直前に立っているダイアログ。最も手前のモーダル。
    chord_transfer_confirm: Option<ChordTransferConfirm>,
}

impl Default for MmlOverlay<'_> {
    fn default() -> Self {
        Self {
            open: false,
            input_mode: MmlOverlayInputMode::MultiLine,
            textarea: cmrt_tui_core::text_input::new_multi_line_textarea(Vec::new()),
            last_notes: None,
            sounding: Vec::new(),
            sounding_from_chord: false,
            sender_command_id: 0,
            patch: None,
            patch_catalog: PatchCatalogSnapshot::Loading,
            patch_role_index: PatchRoleIndex::default(),
            load_measurements: BTreeMap::new(),
            history: Vec::new(),
            favorites: Vec::new(),
            patch_filter_presets: Vec::new(),
            catalog_notes: Vec::new(),
            patch_catalog_notice: None,
            patch_select_requested: false,
            patch_select: None,
            history_select: None,
            play_settings: PlaySettings::default(),
            play_settings_select: None,
            line_status: LineStatus::Idle,
            chord_row_transfer: false,
            chord_hint: false,
            chord_transfer_confirm: None,
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

    /// 前回の音色だけを引き継いだ入力欄で開く。
    ///
    /// 複数行モードは従来どおり必ず空で開く。1 行モードだけ
    /// [`MmlOverlayContext::initial_text`] を入れた状態で開く（DAW が
    /// 「そのセルの MML を編集する」ために使う）。
    pub fn open(&mut self, context: MmlOverlayContext) {
        self.input_mode = context.input_mode;
        self.textarea = single_line::new_textarea(context.input_mode, &context.initial_text);
        self.last_notes = None;
        self.sounding.clear();
        self.sounding_from_chord = false;
        self.sender_command_id = 0;
        self.line_status = LineStatus::Idle;
        self.patch_catalog = context.patch_catalog;
        self.patch_role_index = context.patch_role_index;
        self.load_measurements = context.load_measurements;
        self.history = context.history;
        self.favorites = context.favorites;
        self.patch_filter_presets = context.patch_filter_presets;
        self.catalog_notes = context.catalog_notes;
        self.patch_catalog_notice = None;
        self.patch_select_requested = false;
        self.patch_select = None;
        self.history_select = None;
        self.play_settings_select = None;
        self.chord_row_transfer = context.chord_row_transfer;
        self.chord_transfer_confirm = None;
        self.open = true;
        // 開いた直後の初期テキストにも効かせる。手書きで書き込んでしまった
        // コード表記は、開き直したときこそ気づける。
        self.refresh_chord_hint();
    }

    /// 打鍵を 1 つ処理する。
    ///
    /// chord のヒントは**どの経路を通っても最後に**作り直す。入力欄が変わる出口が
    /// 複数ある（打鍵・行の移動・履歴の取り込み）ので、経路ごとに更新すると必ず
    /// 取りこぼす。
    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) -> MmlOverlayAction {
        // 確定ダイアログは演奏設定よりさらに手前。開いている間は打鍵を入力欄へ通さない。
        if self.chord_transfer_confirm.is_some() {
            return self.handle_chord_transfer_key(key);
        }
        let action = self.handle_key_inner(key, now);
        self.refresh_chord_hint();
        action
    }

    fn handle_key_inner(&mut self, key: KeyEvent, now: Instant) -> MmlOverlayAction {
        // 演奏設定は最も手前のモーダル。音色選択の最中にも開ける必要があるので、
        // どの委譲よりも先に判定する。
        if let Some(action) = self.intercept_play_settings_key(key) {
            return action;
        }
        if self.patch_select.is_some() {
            return self.handle_patch_select_key(key, now);
        }
        if self.history_select.is_some() {
            return self.handle_history_select_key(key);
        }
        // 1 行モードの確定は、どのモーダルも開いていないときだけ。音色選択の
        // `Enter`（＝候補の確定）を横取りしてはいけないので、委譲より後に置く。
        if let Some(action) = self.intercept_single_line_key(key) {
            return action;
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
        self.release_context();
        MmlOverlayAction::Close
    }

    /// 開いている間だけ持っていたスナップショットを手放し、閉じた状態にする。
    pub(super) fn release_context(&mut self) {
        self.patch_catalog = PatchCatalogSnapshot::Loading;
        self.patch_role_index = PatchRoleIndex::default();
        self.history = Vec::new();
        self.favorites = Vec::new();
        self.patch_filter_presets = Vec::new();
        self.patch_select = None;
        self.patch_catalog_notice = None;
        self.patch_select_requested = false;
        self.history_select = None;
        self.play_settings_select = None;
        self.chord_row_transfer = false;
        self.chord_hint = false;
        self.chord_transfer_confirm = None;
        self.open = false;
        self.forget_sounding();
    }

    /// カーソルのある行をまるごと鳴らす。
    ///
    /// 打鍵の 1 音は行の演奏に飲み込まれるので、その記録は落とす。ここで note off を
    /// 組み立てないのは、[`MmlOverlayAction::PlayLine`] 自体が「鳴っているものを
    /// 止めてから積む」の意味だから。止めるのは受け取る側の 1 か所だけが行う。
    fn play_current_line(&mut self, patch: PatchChange) -> MmlOverlayAction {
        let (status, performance) = line_events(self.current_line());
        self.line_status = status;
        self.forget_cursor_unit();
        MmlOverlayAction::PlayLine {
            patch,
            program: self.play_settings.program(performance),
        }
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

    pub(super) fn current_line(&self) -> &str {
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

#[cfg(test)]
mod tests;
