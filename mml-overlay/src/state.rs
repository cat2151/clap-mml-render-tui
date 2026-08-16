//! MML 入力オーバーレイの状態と、打鍵から発音までの判定。

mod history;
mod patch;

use std::time::Instant;

use cmrt_chord::TimedMidiEvent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::{DataCursor, TextArea};

use crate::cursor_notes::{notes_at_cursor, CursorNotes};
use crate::history_select::{is_history_select_trigger, HistorySelect};
use crate::line_play::{line_events, LineStatus};
use crate::patch_select::{is_patch_select_trigger, PatchSelect};
use crate::{NOTE_OFF, NOTE_ON};

/// 行を鳴らす前に音源の音色を差し替えるか。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchChange {
    /// いまの音色のまま鳴らす。
    Keep,
    /// 鳴らす前にこの音色へ差し替える（`None` は realtime server の既定音色へ戻す）。
    Switch(Option<String>),
}

/// オーバーレイが呼び出し側へ求める処理。
#[derive(Clone, Debug, PartialEq)]
pub enum MmlOverlayAction {
    Continue,
    /// この MIDI メッセージを送る（前の音の note off → 新しい音の note on の順）。
    Send(Vec<[u8; 3]>),
    /// 音源の音色を差し替えてから、続けてこの MIDI メッセージを送る。
    /// `patch` が `None` なら realtime server の既定音色へ戻す。
    SetPatch {
        patch: Option<String>,
        messages: Vec<[u8; 3]>,
    },
    /// 走っている行の演奏を止め、あらためてこの行を頭から積む。
    /// `events` が空なら止めるだけ。
    PlayLine {
        patch: PatchChange,
        events: Vec<TimedMidiEvent>,
    },
    /// オーバーレイを閉じる。付随する note off を含む。
    Close(Vec<[u8; 3]>),
}

/// オーバーレイを開くときに呼び出し側から渡すスナップショット。
///
/// どれも読み込みが終わっていなければ空で渡してよく、その場合は音色選択や
/// 履歴が開かないだけになる。
#[derive(Default)]
pub struct MmlOverlayContext {
    /// 音色一覧（表示名, 小文字化）。
    pub patches: Vec<(String, String)>,
    /// notepad 画面と共有しているフレーズ履歴。
    pub history: Vec<String>,
    pub favorites: Vec<String>,
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
    /// いま鳴っている note number。
    sounding: Vec<u8>,
    /// いま鳴っている音が chord 表記から来たか。表示だけに使う。
    sounding_from_chord: bool,
    /// 鳴っている音を止める時刻。書かれた音長ぶんだけ先に置く。
    gate_deadline: Option<Instant>,
    /// 入力欄とは別に持つ音色。`Ctrl+T` と履歴の取り込みだけが書き換える。
    patch: Option<String>,
    /// 開いている間だけ持つ patch 一覧のスナップショット（表示名, 小文字化）。
    patches: Vec<(String, String)>,
    /// 開いている間だけ持つフレーズ履歴のスナップショット。
    history: Vec<String>,
    favorites: Vec<String>,
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
            gate_deadline: None,
            patch: None,
            patches: Vec::new(),
            history: Vec::new(),
            favorites: Vec::new(),
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
        self.gate_deadline = None;
        self.line_status = LineStatus::Idle;
        self.patches = context.patches;
        self.history = context.history;
        self.favorites = context.favorites;
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

    /// gate の期限が来た音を止める。メインループから毎フレーム呼ぶ。
    pub fn poll(&mut self, now: Instant) -> Option<Vec<[u8; 3]>> {
        if now < self.gate_deadline? {
            return None;
        }
        let messages = self.stop_all();
        (!messages.is_empty()).then_some(messages)
    }

    fn close(&mut self) -> MmlOverlayAction {
        self.patches = Vec::new();
        self.history = Vec::new();
        self.favorites = Vec::new();
        self.patch_select = None;
        self.history_select = None;
        self.open = false;
        MmlOverlayAction::Close(self.stop_all())
    }

    /// カーソルのある行をまるごと鳴らす。
    ///
    /// 打鍵の 1 音は行の演奏に飲み込まれるので、鳴らしていた音の記録は落とす。
    /// note off を送らないのは、行の演奏を積む側が先に音源を止めるため。
    fn play_current_line(&mut self, patch: PatchChange) -> MmlOverlayAction {
        let (status, events) = line_events(self.current_line());
        self.line_status = status;
        self.forget_typed_note();
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
    fn refresh(&mut self, now: Instant) -> MmlOverlayAction {
        let notes = self.notes_at_cursor();
        if notes == self.last_notes {
            return MmlOverlayAction::Continue;
        }
        self.last_notes.clone_from(&notes);
        let Some((_, notes)) = notes else {
            return MmlOverlayAction::Continue;
        };
        MmlOverlayAction::Send(self.start_notes(&notes, now))
    }

    fn notes_at_cursor(&self) -> Option<(usize, CursorNotes)> {
        let DataCursor(row, column) = self.textarea.cursor();
        notes_at_cursor(self.current_line(), column).map(|notes| (row, notes))
    }

    fn start_notes(&mut self, notes: &CursorNotes, now: Instant) -> Vec<[u8; 3]> {
        let mut messages = self.stop_all();
        messages.extend(
            notes
                .pitches
                .iter()
                .map(|pitch| [NOTE_ON, *pitch, notes.velocity]),
        );
        self.sounding.clone_from(&notes.pitches);
        self.sounding_from_chord = notes.from_chord;
        self.gate_deadline = Some(now + notes.duration);
        messages
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

    /// 打鍵で鳴らした音の記録を捨てる。行の演奏で音源ごと止まるときに使う。
    /// 記録を残すと、行内へカーソルが戻ったときに同じ音が鳴り直さない。
    fn forget_typed_note(&mut self) {
        self.last_notes = None;
        self.sounding.clear();
        self.sounding_from_chord = false;
        self.gate_deadline = None;
    }

    fn stop_all(&mut self) -> Vec<[u8; 3]> {
        self.gate_deadline = None;
        self.sounding_from_chord = false;
        self.sounding
            .drain(..)
            .map(|pitch| [NOTE_OFF, pitch, 0])
            .collect()
    }
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
