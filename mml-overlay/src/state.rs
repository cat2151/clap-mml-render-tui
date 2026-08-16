//! MML 入力オーバーレイの状態と、打鍵から発音までの判定。

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::{CursorMove, DataCursor, TextArea};

use crate::patch_json;
use crate::patch_select::{is_patch_select_trigger, PatchSelect, PatchSelectAction};
use crate::prefix_notes::{notes_at_cursor, PrefixNotes};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;

/// 打鍵で鳴らした音を鳴らし続ける長さ。
///
/// MML の音長どおりに鳴らすには BPM が要るが、それは画面ごとに持ち方が違う。
/// ここでは「打鍵の手応え」だけを目的に固定長で切る。
const GATE: Duration = Duration::from_millis(250);

/// MML がまだ空のときに、音色の試聴だけを目的に鳴らす音（C5）。
const PREVIEW_PITCH: u8 = 60;
const PREVIEW_VELOCITY: u8 = 127;

/// オーバーレイが呼び出し側へ求める処理。
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// オーバーレイを閉じる。付随する note off を含む。
    Close(Vec<[u8; 3]>),
}

/// どの画面からでも開ける MML 入力オーバーレイ。
///
/// MML そのものは揮発でよいため、閉じると捨てる。音色だけは
/// [`MmlOverlay::patch`] に残し、呼び出し側がセッションへ保存する。
pub struct MmlOverlay<'a> {
    open: bool,
    textarea: TextArea<'a>,
    /// 直近に鳴らした音。これと違う音になった瞬間だけ発音する。
    last_notes: Option<PrefixNotes>,
    /// いま鳴っている note number。
    sounding: Vec<u8>,
    gate_deadline: Option<Instant>,
    /// 行頭 JSON が指す音色。閉じても残す。
    patch: Option<String>,
    /// 開いている間だけ持つ patch 一覧のスナップショット（表示名, 小文字化）。
    patches: Vec<(String, String)>,
    patch_select: Option<PatchSelect<'a>>,
}

impl Default for MmlOverlay<'_> {
    fn default() -> Self {
        Self {
            open: false,
            textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            last_notes: None,
            sounding: Vec::new(),
            gate_deadline: None,
            patch: None,
            patches: Vec::new(),
            patch_select: None,
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

    /// 行頭 JSON が指す音色。セッション保存はこれを見る。
    pub fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }

    /// セッションから復元した音色を入れる。起動時に1度だけ呼ぶ。
    pub fn set_restored_patch(&mut self, patch: Option<String>) {
        self.patch = patch;
    }

    pub(crate) fn patch_select(&self) -> Option<&PatchSelect<'a>> {
        self.patch_select.as_ref()
    }

    /// 前回の音色だけを引き継いだ入力欄で開く。
    ///
    /// `patches` は音色選択に使う一覧のスナップショット。読み込みが終わって
    /// いなければ空で渡してよく、その場合は音色選択が開かないだけになる。
    pub fn open(&mut self, patches: Vec<(String, String)>) {
        let text = match &self.patch {
            Some(patch) => patch_json::set_patch_name("", patch).0,
            None => String::new(),
        };
        self.textarea = cmrt_tui_core::text_input::new_single_line_textarea(&text);
        self.last_notes = None;
        self.sounding.clear();
        self.gate_deadline = None;
        self.patches = patches;
        self.patch_select = None;
        self.open = true;
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) -> MmlOverlayAction {
        if self.patch_select.is_some() {
            return self.handle_patch_select_key(key, now);
        }
        if key.code == KeyCode::Esc {
            return self.close();
        }
        // 1行入力なので改行は入れさせない。
        if is_newline_key(key) {
            return MmlOverlayAction::Continue;
        }
        if is_patch_select_trigger(key) {
            self.open_patch_select();
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

    fn close(&mut self) -> MmlOverlayAction {
        // 手で JSON を書き換えた場合も次に開いたときの音色がテキストと一致するよう、
        // 覚える音色は最後にテキストから読み直す。
        self.patch = patch_json::patch_name(&self.value());
        self.patches = Vec::new();
        self.patch_select = None;
        self.open = false;
        MmlOverlayAction::Close(self.stop_all())
    }

    fn open_patch_select(&mut self) {
        let current = patch_json::patch_name(&self.value());
        self.patch_select = PatchSelect::open(self.patches.clone(), current.as_deref());
    }

    fn handle_patch_select_key(&mut self, key: KeyEvent, now: Instant) -> MmlOverlayAction {
        let Some(select) = self.patch_select.as_mut() else {
            return MmlOverlayAction::Continue;
        };
        match select.handle_key(key) {
            PatchSelectAction::Continue => MmlOverlayAction::Continue,
            PatchSelectAction::Preview(patch) => {
                let messages = self.preview_notes(now);
                MmlOverlayAction::SetPatch {
                    patch: Some(patch),
                    messages,
                }
            }
            PatchSelectAction::Confirm(patch) => {
                // 試聴で読み込み済みの音色がそのまま残るので、ここでは積み直さない。
                self.patch_select = None;
                self.apply_patch_to_text(&patch);
                self.patch = Some(patch);
                MmlOverlayAction::Continue
            }
            PatchSelectAction::Cancel => self.cancel_patch_select(),
        }
    }

    fn cancel_patch_select(&mut self) -> MmlOverlayAction {
        let Some(select) = self.patch_select.take() else {
            return MmlOverlayAction::Continue;
        };
        if select.previewed() == select.original() {
            return MmlOverlayAction::Continue;
        }
        MmlOverlayAction::SetPatch {
            patch: select.original().map(str::to_string),
            messages: Vec::new(),
        }
    }

    /// 行頭へ音色を書き、MML 本体上のカーソル位置は保つ。
    fn apply_patch_to_text(&mut self, patch: &str) {
        let (text, delta) = patch_json::set_patch_name(&self.value(), patch);
        let column = self.cursor_column().saturating_add_signed(delta);
        self.textarea = cmrt_tui_core::text_input::new_single_line_textarea(&text);
        self.textarea
            .move_cursor(CursorMove::Jump(0, column.min(u16::MAX as usize) as u16));
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
        MmlOverlayAction::Send(self.start_notes(&notes.pitches, notes.velocity, now))
    }

    /// 音色を切り替えた直後に鳴らす音。
    ///
    /// カーソル位置に音があればそれを鳴らし直す。まだ MML が空でも音色は聴きたいので、
    /// その場合だけ試聴用の音を1つ鳴らす。
    fn preview_notes(&mut self, now: Instant) -> Vec<[u8; 3]> {
        let notes = notes_at_cursor(&self.value(), self.cursor_column());
        self.last_notes.clone_from(&notes);
        match notes {
            Some(notes) => self.start_notes(&notes.pitches, notes.velocity, now),
            None => self.start_notes(&[PREVIEW_PITCH], PREVIEW_VELOCITY, now),
        }
    }

    fn start_notes(&mut self, pitches: &[u8], velocity: u8, now: Instant) -> Vec<[u8; 3]> {
        let mut messages = self.stop_all();
        messages.extend(pitches.iter().map(|pitch| [NOTE_ON, *pitch, velocity]));
        self.sounding = pitches.to_vec();
        self.gate_deadline = Some(now + GATE);
        messages
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
