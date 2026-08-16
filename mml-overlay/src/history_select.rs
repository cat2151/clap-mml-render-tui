//! MML オーバーレイから開くフレーズ履歴。
//!
//! 履歴の中身は notepad 画面と共有している（`patch_history.json`）。notepad 側の
//! 履歴 overlay は `NotepadScreen` の editor・音声キャッシュ・先読みへ密結合していて
//! 部品として借りられないため、音色選択と同じく最小構成をここへ置く。
//!
//! 履歴の項目は notepad が書いた行そのもの（`{"Surge XT patch": "..."} cde`）なので、
//! 選ぶ時点で音色と MML 本体へ分ける。読み取り専用で、ここから履歴は書き換えない。

use std::cell::Cell;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use cmrt_tui_core::{patches::filter_items, text_input};

use crate::patch_json;

/// 一覧に表示できる行数の既定値。実際の値は描画時に [`HistorySelect::set_page_size`] で入る。
const DEFAULT_PAGE_SIZE: usize = 10;

/// 履歴とお気に入りのどちらを見ているか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryPane {
    History,
    Favorites,
}

/// 履歴から選んだ 1 行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryPick {
    /// 行頭 JSON を除いた MML 本体。
    pub(crate) mml: String,
    /// 行頭 JSON が指していた音色。無ければ今の音色のまま鳴らす。
    pub(crate) patch: Option<String>,
}

impl HistoryPick {
    fn from_line(line: &str) -> Self {
        Self {
            mml: patch_json::strip_patch_json(line).to_string(),
            patch: patch_json::patch_name(line),
        }
    }
}

pub(crate) enum HistorySelectAction {
    /// 表示が変わっただけ。
    Continue,
    /// この項目を試聴する。
    Preview(HistoryPick),
    /// この項目で入力欄を上書きして閉じる。
    Confirm(HistoryPick),
    /// 取り消して閉じる。試聴で変えた音色は呼び出し側が戻す。
    Cancel,
}

pub(crate) struct HistorySelect<'a> {
    history: Vec<String>,
    favorites: Vec<String>,
    /// 絞り込み後の表示（ペインごと）。
    filtered_history: Vec<String>,
    filtered_favorites: Vec<String>,
    focus: HistoryPane,
    history_cursor: usize,
    favorites_cursor: usize,
    query: TextArea<'a>,
    /// 一度でも試聴したか。取り消しで音色を戻す必要があるかの判定に使う。
    previewed: bool,
    /// 描画できた行数。描画は `&self` で回るので内部可変で持つ。
    page_size: Cell<usize>,
}

impl<'a> HistorySelect<'a> {
    /// 履歴もお気に入りも空なら開かない（`None` を返す）。
    pub(crate) fn open(history: Vec<String>, favorites: Vec<String>) -> Option<Self> {
        if history.is_empty() && favorites.is_empty() {
            return None;
        }
        let focus = if history.is_empty() {
            HistoryPane::Favorites
        } else {
            HistoryPane::History
        };
        Some(Self {
            filtered_history: history.clone(),
            filtered_favorites: favorites.clone(),
            history,
            favorites,
            focus,
            history_cursor: 0,
            favorites_cursor: 0,
            query: text_input::new_single_line_textarea(""),
            previewed: false,
            page_size: Cell::new(DEFAULT_PAGE_SIZE),
        })
    }

    pub(crate) fn query_textarea(&self) -> &TextArea<'a> {
        &self.query
    }

    pub(crate) fn focus(&self) -> HistoryPane {
        self.focus
    }

    pub(crate) fn previewed(&self) -> bool {
        self.previewed
    }

    pub(crate) fn items(&self, pane: HistoryPane) -> &[String] {
        match pane {
            HistoryPane::History => &self.filtered_history,
            HistoryPane::Favorites => &self.filtered_favorites,
        }
    }

    pub(crate) fn total(&self, pane: HistoryPane) -> usize {
        match pane {
            HistoryPane::History => self.history.len(),
            HistoryPane::Favorites => self.favorites.len(),
        }
    }

    pub(crate) fn cursor(&self, pane: HistoryPane) -> usize {
        match pane {
            HistoryPane::History => self.history_cursor,
            HistoryPane::Favorites => self.favorites_cursor,
        }
    }

    pub(crate) fn set_page_size(&self, page_size: usize) {
        self.page_size.set(page_size.max(1));
    }

    fn selected(&self) -> Option<&str> {
        self.items(self.focus)
            .get(self.cursor(self.focus))
            .map(String::as_str)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> HistorySelectAction {
        match key.code {
            KeyCode::Esc => return HistorySelectAction::Cancel,
            KeyCode::Enter => {
                return match self.selected() {
                    Some(line) => HistorySelectAction::Confirm(HistoryPick::from_line(line)),
                    None => HistorySelectAction::Cancel,
                }
            }
            KeyCode::Tab | KeyCode::BackTab => return self.toggle_focus(),
            KeyCode::Up => return self.move_cursor(-1),
            KeyCode::Down => return self.move_cursor(1),
            KeyCode::PageUp => return self.move_cursor(-(self.page_size.get() as isize)),
            KeyCode::PageDown => return self.move_cursor(self.page_size.get() as isize),
            _ => {}
        }
        // 上記以外はすべて絞り込み欄へ。Ctrl+W の単語削除や Ctrl+U の undo が
        // そのまま効くよう、キーの解釈は textarea に任せる。
        if !text_input::apply_key_event_to_textarea(&mut self.query, key) {
            return HistorySelectAction::Continue;
        }
        self.refilter()
    }

    /// 左右のペインを行き来する。絞り込み欄でカーソルを動かせるよう、
    /// 切り替えは矢印ではなく Tab に割り当てる。
    fn toggle_focus(&mut self) -> HistorySelectAction {
        self.focus = match self.focus {
            HistoryPane::History => HistoryPane::Favorites,
            HistoryPane::Favorites => HistoryPane::History,
        };
        self.preview_selected()
    }

    fn refilter(&mut self) -> HistorySelectAction {
        let selected = self.selected().map(str::to_string);
        let query = text_input::textarea_value(&self.query);
        self.filtered_history = filter_items(&self.history, &query);
        self.filtered_favorites = filter_items(&self.favorites, &query);
        // 絞り込んだ結果に元の選択が残っていれば、そこへ留まる。試聴が鳴り直さずに済む。
        let cursor = selected
            .as_deref()
            .and_then(|selected| {
                self.items(self.focus)
                    .iter()
                    .position(|line| line == selected)
            })
            .unwrap_or(0);
        self.set_cursor(cursor);
        if self.selected() == selected.as_deref() {
            return HistorySelectAction::Continue;
        }
        self.preview_selected()
    }

    fn move_cursor(&mut self, delta: isize) -> HistorySelectAction {
        let items = self.items(self.focus).len();
        if items == 0 {
            return HistorySelectAction::Continue;
        }
        let next = self
            .cursor(self.focus)
            .saturating_add_signed(delta)
            .min(items - 1);
        if next == self.cursor(self.focus) {
            return HistorySelectAction::Continue;
        }
        self.set_cursor(next);
        self.preview_selected()
    }

    fn set_cursor(&mut self, cursor: usize) {
        match self.focus {
            HistoryPane::History => self.history_cursor = cursor,
            HistoryPane::Favorites => self.favorites_cursor = cursor,
        }
    }

    fn preview_selected(&mut self) -> HistorySelectAction {
        let Some(line) = self.selected() else {
            return HistorySelectAction::Continue;
        };
        let pick = HistoryPick::from_line(line);
        self.previewed = true;
        HistorySelectAction::Preview(pick)
    }
}

/// このキーはフレーズ履歴を開く。
///
/// オーバーレイが開いている間は印字可能なキーがすべて MML 入力へ入るので、
/// 起動キーは修飾キー付きにするしかない。`Ctrl+H` は端末上で Backspace と、
/// `Ctrl+R` / `Ctrl+U` は入力欄の undo / redo とぶつかるため `Ctrl+O` を使う。
pub fn is_history_select_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('o')
}

#[cfg(test)]
mod tests;
