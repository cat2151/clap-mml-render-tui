use tui_textarea::TextArea;

use super::super::DawPatchSelectPane;

pub(crate) struct DawPatchSelectOverlayState {
    pub(crate) all: Vec<(String, String)>,
    pub(crate) query: String,
    pub(crate) query_textarea: TextArea<'static>,
    pub(crate) query_before_input: String,
    pub(crate) filtered: Vec<String>,
    pub(crate) cursor: usize,
    pub(crate) favorite_items: Vec<String>,
    pub(crate) favorites_query: String,
    pub(crate) favorites_query_textarea: TextArea<'static>,
    pub(crate) favorites_query_before_input: String,
    pub(crate) favorites_cursor: usize,
    pub(crate) focus: DawPatchSelectPane,
    pub(crate) filter_active: bool,
}

impl DawPatchSelectOverlayState {
    pub(crate) fn new() -> Self {
        Self {
            all: Vec::new(),
            query: String::new(),
            query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            query_before_input: String::new(),
            filtered: Vec::new(),
            cursor: 0,
            favorite_items: Vec::new(),
            favorites_query: String::new(),
            favorites_query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            favorites_query_before_input: String::new(),
            favorites_cursor: 0,
            focus: DawPatchSelectPane::Patches,
            filter_active: false,
        }
    }
}
