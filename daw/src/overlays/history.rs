use tui_textarea::TextArea;

use super::super::DawHistoryPane;

pub(crate) struct DawHistoryOverlayState {
    pub(crate) patch_name: Option<String>,
    pub(crate) query: String,
    pub(crate) query_textarea: TextArea<'static>,
    pub(crate) history_cursor: usize,
    pub(crate) favorites_cursor: usize,
    pub(crate) focus: DawHistoryPane,
    pub(crate) filter_active: bool,
}

impl DawHistoryOverlayState {
    pub(crate) fn new() -> Self {
        Self {
            patch_name: None,
            query: String::new(),
            query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            history_cursor: 0,
            favorites_cursor: 0,
            focus: DawHistoryPane::History,
            filter_active: false,
        }
    }
}
