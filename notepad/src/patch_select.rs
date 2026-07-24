use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use cmrt_tui_core::patches::PatchSortOrder;

use super::PatchSelectPane;

/// 音色選択 overlay が所有する表示・入力状態。
///
/// 音色の読み込みや preview 再生といった runtime service は `TuiApp` に残し、
/// overlay の状態だけをここへ集約する。
pub(super) struct PatchSelectState<'a> {
    pub(super) patch_all: Vec<(String, String)>,
    pub(super) patch_all_source_order: Vec<(String, String)>,
    pub(super) patch_query: String,
    pub(super) patch_query_textarea: TextArea<'a>,
    pub(super) patch_filtered: Vec<String>,
    pub(super) patch_cursor: usize,
    pub(super) patch_list_state: ListState,
    pub(super) patch_favorite_items: Vec<String>,
    pub(super) patch_favorites_query: String,
    pub(super) patch_favorites_query_textarea: TextArea<'a>,
    pub(super) patch_favorites_cursor: usize,
    pub(super) patch_favorites_state: ListState,
    pub(super) patch_select_focus: PatchSelectPane,
    pub(super) patch_select_filter_active: bool,
    pub(super) patch_select_sort_order: PatchSortOrder,
    pub(super) patch_select_page_size: usize,
}

impl PatchSelectState<'_> {
    pub(super) fn new() -> Self {
        Self {
            patch_all: Vec::new(),
            patch_all_source_order: Vec::new(),
            patch_query: String::new(),
            patch_query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            patch_filtered: Vec::new(),
            patch_cursor: 0,
            patch_list_state: ListState::default(),
            patch_favorite_items: Vec::new(),
            patch_favorites_query: String::new(),
            patch_favorites_query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            patch_favorites_cursor: 0,
            patch_favorites_state: ListState::default(),
            patch_select_focus: PatchSelectPane::Patches,
            patch_select_filter_active: false,
            patch_select_sort_order: PatchSortOrder::Path,
            patch_select_page_size: 1,
        }
    }
}
