use tui_textarea::TextArea;

use super::super::DawHistoryPane;

pub(in crate::daw) struct DawHistoryOverlayState {
    pub(in crate::daw) patch_name: Option<String>,
    pub(in crate::daw) query: String,
    pub(in crate::daw) query_textarea: TextArea<'static>,
    pub(in crate::daw) history_cursor: usize,
    pub(in crate::daw) favorites_cursor: usize,
    pub(in crate::daw) focus: DawHistoryPane,
    pub(in crate::daw) filter_active: bool,
}

impl DawHistoryOverlayState {
    pub(in crate::daw) fn new() -> Self {
        Self {
            patch_name: None,
            query: String::new(),
            query_textarea: crate::text_input::new_single_line_textarea(""),
            history_cursor: 0,
            favorites_cursor: 0,
            focus: DawHistoryPane::History,
            filter_active: false,
        }
    }
}
