use tui_textarea::TextArea;

use super::super::DawPatchSelectPane;

pub(in crate::daw) struct DawPatchSelectOverlayState {
    pub(in crate::daw) all: Vec<(String, String)>,
    pub(in crate::daw) query: String,
    pub(in crate::daw) query_textarea: TextArea<'static>,
    pub(in crate::daw) query_before_input: String,
    pub(in crate::daw) filtered: Vec<String>,
    pub(in crate::daw) cursor: usize,
    pub(in crate::daw) favorite_items: Vec<String>,
    pub(in crate::daw) favorites_query: String,
    pub(in crate::daw) favorites_query_textarea: TextArea<'static>,
    pub(in crate::daw) favorites_query_before_input: String,
    pub(in crate::daw) favorites_cursor: usize,
    pub(in crate::daw) focus: DawPatchSelectPane,
    pub(in crate::daw) filter_active: bool,
}

impl DawPatchSelectOverlayState {
    pub(in crate::daw) fn new() -> Self {
        Self {
            all: Vec::new(),
            query: String::new(),
            query_textarea: crate::text_input::new_single_line_textarea(""),
            query_before_input: String::new(),
            filtered: Vec::new(),
            cursor: 0,
            favorite_items: Vec::new(),
            favorites_query: String::new(),
            favorites_query_textarea: crate::text_input::new_single_line_textarea(""),
            favorites_query_before_input: String::new(),
            favorites_cursor: 0,
            focus: DawPatchSelectPane::Patches,
            filter_active: false,
        }
    }
}
