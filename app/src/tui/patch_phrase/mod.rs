mod input;

use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use super::PatchPhrasePane;

/// Patch phrase overlay が所有する表示・入力状態。
pub(super) struct PatchPhraseState<'a> {
    pub(super) patch_name: Option<String>,
    pub(super) history_cursor: usize,
    pub(super) favorites_cursor: usize,
    pub(super) history_state: ListState,
    pub(super) favorites_state: ListState,
    pub(super) focus: PatchPhrasePane,
    pub(super) query: String,
    pub(super) query_textarea: TextArea<'a>,
    pub(super) filter_active: bool,
    pub(super) page_size: usize,
}

impl PatchPhraseState<'_> {
    pub(super) fn new() -> Self {
        Self {
            patch_name: None,
            history_cursor: 0,
            favorites_cursor: 0,
            history_state: ListState::default(),
            favorites_state: ListState::default(),
            focus: PatchPhrasePane::History,
            query: String::new(),
            query_textarea: crate::text_input::new_single_line_textarea(""),
            filter_active: false,
            page_size: 1,
        }
    }
}
