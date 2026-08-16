//! MML オーバーレイから開く音色選択。
//!
//! 既存の音色選択 UI（notepad / grid sequencer / keyboard）はいずれも各画面の
//! 状態へ密結合していて、部品として借りられない。ここは絞り込み欄 + 1 ペインだけの
//! 最小構成に絞り、共有できる部分は `cmrt_tui_core::patches` の絞り込みと
//! `text_input` の 1 行入力に任せる。
//!
//! 選択そのものはここに閉じ、音を鳴らす/テキストを書き換えるのは [`crate::state`] 側。

use std::cell::Cell;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use cmrt_tui_core::{patches::filter_patches_by_display_path, text_input};

/// 一覧に表示できる行数の既定値。実際の値は描画時に [`PatchSelect::set_page_size`] で入る。
const DEFAULT_PAGE_SIZE: usize = 10;

/// 音色選択が呼び出し側へ求める処理。
pub(crate) enum PatchSelectAction {
    /// 表示が変わっただけ。
    Continue,
    /// この音色を試聴する。
    Preview(String),
    /// この音色で確定して閉じる。
    Confirm(String),
    /// 取り消して閉じる。開いたときの音色へ戻す。
    Cancel,
}

pub(crate) struct PatchSelect<'a> {
    all: Vec<(String, String)>,
    filtered: Vec<String>,
    cursor: usize,
    query: TextArea<'a>,
    /// 開いたときの音色。取り消しで戻す先。
    original: Option<String>,
    /// 直近に試聴した音色。同じ音色を続けて読み込ませないために持つ。
    previewed: Option<String>,
    /// 描画できた行数。描画は `&self` で回るので内部可変で持つ。
    page_size: Cell<usize>,
}

impl<'a> PatchSelect<'a> {
    /// 音色が 1 つも無ければ開かない（`None` を返す）。
    pub(crate) fn open(all: Vec<(String, String)>, current: Option<&str>) -> Option<Self> {
        if all.is_empty() {
            return None;
        }
        let filtered = filter_patches_by_display_path(&all, "");
        let cursor = current
            .and_then(|current| filtered.iter().position(|patch| patch == current))
            .unwrap_or(0);
        Some(Self {
            all,
            filtered,
            cursor,
            query: text_input::new_single_line_textarea(""),
            original: current.map(str::to_string),
            previewed: current.map(str::to_string),
            page_size: Cell::new(DEFAULT_PAGE_SIZE),
        })
    }

    pub(crate) fn query_textarea(&self) -> &TextArea<'a> {
        &self.query
    }

    pub(crate) fn filtered(&self) -> &[String] {
        &self.filtered
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn total(&self) -> usize {
        self.all.len()
    }

    pub(crate) fn original(&self) -> Option<&str> {
        self.original.as_deref()
    }

    /// 直近に試聴した音色。取り消しで戻す必要があるかの判定に使う。
    pub(crate) fn previewed(&self) -> Option<&str> {
        self.previewed.as_deref()
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.filtered.get(self.cursor).map(String::as_str)
    }

    /// 描画できた行数を反映する。PageUp/PageDown の送り幅に使う。
    pub(crate) fn set_page_size(&self, page_size: usize) {
        self.page_size.set(page_size.max(1));
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        match key.code {
            KeyCode::Esc => return PatchSelectAction::Cancel,
            KeyCode::Enter => {
                return match self.selected() {
                    Some(patch) => PatchSelectAction::Confirm(patch.to_string()),
                    None => PatchSelectAction::Cancel,
                }
            }
            KeyCode::Up => return self.move_cursor(-1),
            KeyCode::Down => return self.move_cursor(1),
            KeyCode::PageUp => return self.move_cursor(-(self.page_size.get() as isize)),
            KeyCode::PageDown => return self.move_cursor(self.page_size.get() as isize),
            _ => {}
        }
        // 上記以外はすべて絞り込み欄へ。Ctrl+W の単語削除や Ctrl+U の undo が
        // そのまま効くよう、キーの解釈は textarea に任せる。
        if !text_input::apply_key_event_to_textarea(&mut self.query, key) {
            return PatchSelectAction::Continue;
        }
        self.refilter()
    }

    fn refilter(&mut self) -> PatchSelectAction {
        let selected = self.selected().map(str::to_string);
        self.filtered =
            filter_patches_by_display_path(&self.all, &text_input::textarea_value(&self.query));
        // 絞り込んだ結果に元の選択が残っていれば、そこへ留まる。試聴が鳴り直さずに済む。
        self.cursor = selected
            .and_then(|selected| self.filtered.iter().position(|patch| *patch == selected))
            .unwrap_or(0);
        self.preview_selected()
    }

    fn move_cursor(&mut self, delta: isize) -> PatchSelectAction {
        if self.filtered.is_empty() {
            return PatchSelectAction::Continue;
        }
        let last = self.filtered.len() - 1;
        let next = self.cursor.saturating_add_signed(delta).min(last);
        if next == self.cursor {
            return PatchSelectAction::Continue;
        }
        self.cursor = next;
        self.preview_selected()
    }

    fn preview_selected(&mut self) -> PatchSelectAction {
        let Some(patch) = self.selected() else {
            return PatchSelectAction::Continue;
        };
        if self.previewed.as_deref() == Some(patch) {
            return PatchSelectAction::Continue;
        }
        let patch = patch.to_string();
        self.previewed = Some(patch.clone());
        PatchSelectAction::Preview(patch)
    }
}

/// このキーは音色選択を開く。
///
/// オーバーレイが開いている間は印字可能なキーがすべて MML 入力へ入るので、
/// 起動キーは修飾キー付きにするしかない。notepad 画面の `t`（tone）に合わせた。
pub fn is_patch_select_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('t')
}

#[cfg(test)]
mod tests;
