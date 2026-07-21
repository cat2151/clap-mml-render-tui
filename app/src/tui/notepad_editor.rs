use ratatui::widgets::ListState;
use tui_textarea::TextArea;

/// notepad の編集対象と、vim風編集にだけ必要な一時状態。
pub(in crate::tui) struct NotepadEditorState<'a> {
    pub(in crate::tui) lines: Vec<String>,
    pub(in crate::tui) cursor: usize,
    pub(in crate::tui) list_state: ListState,
    pub(in crate::tui) textarea: TextArea<'a>,
    pub(in crate::tui) page_size: usize,
    pub(in crate::tui) pending_delete: bool,
    pub(in crate::tui) yank_buffer: Option<String>,
}

impl<'a> NotepadEditorState<'a> {
    pub(in crate::tui) fn new(
        lines: Vec<String>,
        cursor: usize,
        list_state: ListState,
        textarea: TextArea<'a>,
    ) -> Self {
        Self {
            lines,
            cursor,
            list_state,
            textarea,
            page_size: 1,
            pending_delete: false,
            yank_buffer: None,
        }
    }
}
