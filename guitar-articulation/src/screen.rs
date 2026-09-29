//! Guitar Articulation 画面の状態とキー処理。
//!
//! 持つのは入力 MML・そこから作ったrawの列・ルール表・Articulatedの列・カーソル列。
//! 音は鳴らさない。移動・入力の取り消し・ヘルプ以外の操作では、鳴らしてほしい版を
//! [`GuitarArticulationAction::Play`] で host へ返す（MML の確定・ルールの toggle はArticulated）。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use crate::ui::{ROW_RULE_ROWS, RULE_ROWS};
use crate::{
    articulate, convert, notes_from_events, Articulated, Note, RowRule, Rule, RuleTable,
    TimedMidiEvent,
};

/// 画面に入ったとき MML が空なら入れておく MML。何を触れば何が変わるかを、打つ前から見せる。
pub const DEFAULT_MML: &str = "l16cdefgab<c";

/// どちらの版を鳴らすか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    /// 入力 MML をそのまま（KS なし）。
    Plain,
    /// ルール表で KS を足した版。
    Converted,
}

impl Take {
    /// ログへ出す綴り。
    pub fn label(self) -> &'static str {
        match self {
            Take::Plain => "plain",
            Take::Converted => "converted",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuitarArticulationAction {
    Continue,
    Quit,
    /// その版のイベント列（[`GuitarArticulationScreen::events`]）を鳴らしてほしい。
    Play(Take),
}

#[derive(Clone, Debug, Default)]
pub struct GuitarArticulationScreen {
    mml: String,
    /// MML を編集している間だけ `Some`。英字キーを入力欄とルールのどちらへ渡すかをこれで分ける。
    input: Option<TextArea<'static>>,
    plain: Vec<TimedMidiEvent>,
    notes: Vec<Note>,
    rules: RuleTable,
    /// `notes` と同じ並びの、音ごとの奏法と強さ。
    articulated: Vec<Articulated>,
    converted: Vec<TimedMidiEvent>,
    cursor: usize,
    help_open: bool,
    /// 直前の操作ができなかった理由。次のキーで消える。
    pub error: Option<String>,
}

impl GuitarArticulationScreen {
    pub fn mml(&self) -> &str {
        &self.mml
    }

    pub fn input_open(&self) -> bool {
        self.input.is_some()
    }

    pub fn help_open(&self) -> bool {
        self.help_open
    }

    pub(crate) fn input(&self) -> Option<&TextArea<'static>> {
        self.input.as_ref()
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn rules(&self) -> &RuleTable {
        &self.rules
    }

    pub fn articulated(&self) -> &[Articulated] {
        &self.articulated
    }

    /// カーソルのある列番号。
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 列の数（同時刻の note on のまとまりの数）。
    pub fn column_count(&self) -> usize {
        self.notes.last().map_or(0, |note| note.column + 1)
    }

    pub fn events(&self, take: Take) -> &[TimedMidiEvent] {
        match take {
            Take::Plain => &self.plain,
            Take::Converted => &self.converted,
        }
    }

    /// 画面に入ったときに呼ぶ。MML 入力欄を開き、MML が空なら [`DEFAULT_MML`] を確定して鳴らす。
    pub fn enter(&mut self) -> GuitarArticulationAction {
        let action = if self.mml.is_empty() {
            match self.commit_mml(DEFAULT_MML) {
                Ok(()) => GuitarArticulationAction::Play(Take::Converted),
                Err(reason) => {
                    self.error = Some(reason);
                    GuitarArticulationAction::Continue
                }
            }
        } else {
            GuitarArticulationAction::Continue
        };
        self.open_input();
        action
    }

    fn open_input(&mut self) {
        if self.input.is_none() {
            self.input = Some(cmrt_tui_core::text_input::new_single_line_textarea(
                &self.mml,
            ));
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        // ヘルプを開いている間は閉じるキーだけを見る。
        if self.help_open {
            if is_help_key(key) || key.code == KeyCode::Esc {
                self.help_open = false;
            }
            return GuitarArticulationAction::Continue;
        }
        // MML は `?` を使わないので、入力欄を開いていても `?` はヘルプへ回す。
        if is_help_key(key) {
            self.help_open = true;
            return GuitarArticulationAction::Continue;
        }
        if self.input.is_some() {
            return self.handle_input_key(key);
        }
        self.error = None;
        match key.code {
            KeyCode::Char('i') => {
                self.open_input();
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.cursor + 1 < self.column_count() {
                    self.cursor += 1;
                }
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('b') => self.play(Take::Plain),
            KeyCode::Char(' ') => self.play(Take::Converted),
            KeyCode::Char('q') => GuitarArticulationAction::Quit,
            KeyCode::Char(ch) => {
                if let Some((rule, _, _)) = RULE_ROWS.iter().find(|(_, key, _)| *key == ch) {
                    self.toggle_rule(*rule)
                } else if let Some((rule, _, _)) =
                    ROW_RULE_ROWS.iter().find(|(_, key, _)| *key == ch)
                {
                    self.toggle_row_rule(*rule)
                } else {
                    GuitarArticulationAction::Continue
                }
            }
            _ => GuitarArticulationAction::Continue,
        }
    }

    /// カーソル列のルールを切り替え、Articulatedを作り直して鳴らす。
    fn toggle_rule(&mut self, rule: Rule) -> GuitarArticulationAction {
        if self.column_count() == 0 {
            self.error = Some("i で MML を入力してください".to_string());
            return GuitarArticulationAction::Continue;
        }
        self.rules.toggle(self.cursor, rule);
        self.rebuild_converted();
        GuitarArticulationAction::Play(Take::Converted)
    }

    /// 行全体のルールを切り替え、Articulatedを作り直して鳴らす。
    fn toggle_row_rule(&mut self, rule: RowRule) -> GuitarArticulationAction {
        self.rules.toggle_row(rule);
        self.rebuild_converted();
        self.play(Take::Converted)
    }

    fn play(&mut self, take: Take) -> GuitarArticulationAction {
        if self.plain.is_empty() {
            self.error = Some("i で MML を入力してください".to_string());
            return GuitarArticulationAction::Continue;
        }
        GuitarArticulationAction::Play(take)
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(mut input) = self.input.take() else {
            return GuitarArticulationAction::Continue;
        };
        if key.code == KeyCode::Esc {
            self.error = None;
            return GuitarArticulationAction::Continue;
        }
        if is_commit_key(key) {
            let value = cmrt_tui_core::text_input::textarea_value(&input);
            return match self.commit_mml(value.trim()) {
                Ok(()) if !self.plain.is_empty() => GuitarArticulationAction::Play(Take::Converted),
                Ok(()) => GuitarArticulationAction::Continue,
                Err(reason) => {
                    // 閉じると打った文字列ごと消えるので、開いたまま理由を出す。
                    self.error = Some(reason);
                    self.input = Some(input);
                    GuitarArticulationAction::Continue
                }
            };
        }
        if cmrt_tui_core::text_input::apply_key_event_to_textarea(&mut input, key) {
            self.error = None;
        }
        self.input = Some(input);
        GuitarArticulationAction::Continue
    }

    /// MML を確定し、raw・Articulated・matrix を作り直す。列の数が変わると列ごとのルールの位置が
    /// 意味を失うので、それは全部消す。列に依らない行全体のルールは残す。空の MML は音を全部空にする。
    fn commit_mml(&mut self, mml: &str) -> Result<(), String> {
        let plain = if mml.is_empty() {
            Vec::new()
        } else {
            cmrt_chord::timed_performance(mml)?.events
        };
        self.mml = mml.to_string();
        self.notes = notes_from_events(&plain);
        self.plain = plain;
        self.rules = self.rules.without_column_rules();
        self.cursor = 0;
        self.error = None;
        self.rebuild_converted();
        Ok(())
    }

    fn rebuild_converted(&mut self) {
        self.articulated = articulate(&self.notes, &self.rules);
        self.converted = convert(&self.plain, &self.rules);
    }
}

fn is_help_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('?')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

/// 1 行入力欄の確定キー。crossterm は `Ctrl+M` を `Enter` とは別に渡してくることがある。
fn is_commit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => true,
        KeyCode::Char('m') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
