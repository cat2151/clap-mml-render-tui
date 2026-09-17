//! Regex 欄。選択中の Preset の一覧に、手入力の条件を AND で重ねる。
//!
//! 条件の規則（表示パス + Category が対象、空白区切り AND、正規表現）は
//! `cmrt_mml_overlay::filter_candidates` が単一ソース。

use std::sync::Arc;

use cmrt_mml_overlay::filter_candidates;
use cmrt_tui_core::text_input;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::TextArea;

use super::PatchSelector;

impl PatchSelector {
    pub(crate) fn query_textarea(&self) -> &TextArea<'static> {
        &self.query
    }

    pub(crate) fn filter_editing(&self) -> bool {
        self.filter_editing
    }

    pub(crate) fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    /// Regex 欄を出すか。編集中か、確定済みの条件が空でないとき。
    pub(crate) fn filter_visible(&self) -> bool {
        self.filter_editing || !self.committed_query.trim().is_empty()
    }

    pub(super) fn start_filter_edit(&mut self) {
        self.filter_editing = true;
        self.query = text_input::new_single_line_textarea(&self.committed_query);
    }

    /// 編集中のキー。一覧が変わったら `true`。
    ///
    /// 候補は入力中にも更新するが、`Esc` なら編集開始時の確定値へ戻す。
    pub(super) fn handle_filter_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.filter_editing = false;
                self.query = text_input::new_single_line_textarea(&self.committed_query);
                self.refilter();
                return true;
            }
            KeyCode::Enter => {
                self.filter_editing = false;
                self.committed_query = text_input::textarea_value(&self.query);
                return false;
            }
            _ => {}
        }
        if !text_input::apply_key_event_to_textarea(&mut self.query, key) {
            return false;
        }
        self.refilter();
        true
    }

    /// 選択中の Preset に Regex 欄の条件を AND して一覧を作り直す。
    /// 今の音色が新しい一覧に残っていればそこに留まり、無ければ先頭。
    pub(super) fn refilter(&mut self) {
        let selected = self.selected_patch().map(str::to_string);
        let query = text_input::textarea_value(&self.query);
        let candidates = &self.presets()[self.preset_cursor].matches;
        let result = if query.trim().is_empty() {
            Ok(Arc::clone(candidates))
        } else {
            filter_candidates(&self.entries, candidates, &query).map(Arc::from)
        };
        match result {
            Ok(filtered) => {
                self.filtered = filtered;
                self.filter_error = None;
            }
            Err(error) => {
                self.filtered = Arc::default();
                self.filter_error = Some(error);
            }
        }
        self.patch_cursor = self.position_of(selected.as_deref()).unwrap_or(0);
    }
}
