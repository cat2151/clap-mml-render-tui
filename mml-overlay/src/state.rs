//! MML 入力オーバーレイの状態と、打鍵から発音までの判定。

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::{DataCursor, TextArea};

use crate::prefix_notes::{notes_at_cursor, PrefixNotes};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;

/// 打鍵で鳴らした音を鳴らし続ける長さ。
///
/// MML の音長どおりに鳴らすには BPM が要るが、それは画面ごとに持ち方が違う。
/// ここでは「打鍵の手応え」だけを目的に固定長で切る。
const GATE: Duration = Duration::from_millis(250);

/// オーバーレイが呼び出し側へ求める処理。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MmlOverlayAction {
    Continue,
    /// この MIDI メッセージを送る（前の音の note off → 新しい音の note on の順）。
    Send(Vec<[u8; 3]>),
    /// オーバーレイを閉じる。付随する note off を含む。
    Close(Vec<[u8; 3]>),
}

/// どの画面からでも開ける MML 入力オーバーレイ。
///
/// 入力内容は揮発でよいため、閉じると捨てる。
pub struct MmlOverlay<'a> {
    open: bool,
    textarea: TextArea<'a>,
    /// 直近に鳴らした音。これと違う音になった瞬間だけ発音する。
    last_notes: Option<PrefixNotes>,
    /// いま鳴っている note number。
    sounding: Vec<u8>,
    gate_deadline: Option<Instant>,
}

impl Default for MmlOverlay<'_> {
    fn default() -> Self {
        Self {
            open: false,
            textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            last_notes: None,
            sounding: Vec::new(),
            gate_deadline: None,
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

    pub fn value(&self) -> String {
        cmrt_tui_core::text_input::textarea_value(&self.textarea)
    }

    /// いま鳴っている音（表示用）。
    pub fn sounding(&self) -> &[u8] {
        &self.sounding
    }

    /// 空の入力欄で開く。
    pub fn open(&mut self) {
        self.textarea = cmrt_tui_core::text_input::new_single_line_textarea("");
        self.last_notes = None;
        self.sounding.clear();
        self.gate_deadline = None;
        self.open = true;
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) -> MmlOverlayAction {
        if key.code == KeyCode::Esc {
            self.open = false;
            return MmlOverlayAction::Close(self.stop_all());
        }
        // 1行入力なので改行は入れさせない。
        if is_newline_key(key) {
            return MmlOverlayAction::Continue;
        }
        self.textarea.input(key);
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

    /// カーソル位置の音を調べ、直前と変わっていれば鳴らす。
    ///
    /// 文字を打ったときもカーソルを動かしたときも同じ判定を通るので、
    /// 「← で戻ったらそこの音がまた鳴る」が特別扱いなしに成り立つ。
    fn refresh(&mut self, now: Instant) -> MmlOverlayAction {
        let notes = notes_at_cursor(&self.value(), self.cursor_column());
        if notes == self.last_notes {
            return MmlOverlayAction::Continue;
        }
        self.last_notes.clone_from(&notes);
        let Some(notes) = notes else {
            // カーソルより前にノートが無くなった。鳴っている音は gate に任せる。
            return MmlOverlayAction::Continue;
        };
        let mut messages = self.stop_all();
        messages.extend(
            notes
                .pitches
                .iter()
                .map(|pitch| [NOTE_ON, *pitch, notes.velocity]),
        );
        self.sounding.clone_from(&notes.pitches);
        self.gate_deadline = Some(now + GATE);
        MmlOverlayAction::Send(messages)
    }

    fn cursor_column(&self) -> usize {
        let DataCursor(_, column) = self.textarea.cursor();
        column
    }

    fn stop_all(&mut self) -> Vec<[u8; 3]> {
        self.gate_deadline = None;
        self.sounding
            .drain(..)
            .map(|pitch| [NOTE_OFF, pitch, 0])
            .collect()
    }
}

/// このキーは改行になるか。`Ctrl+M` は端末上で `Enter` と同じ扱いになる。
fn is_newline_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Enter
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m'))
}

#[cfg(test)]
mod tests;
