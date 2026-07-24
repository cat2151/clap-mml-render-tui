use ratatui::widgets::ListState;
use tui_textarea::TextArea;

/// notepad の編集対象と、vim風編集にだけ必要な一時状態。
pub(crate) struct NotepadEditorState<'a> {
    pub(crate) lines: Vec<String>,
    pub(crate) cursor: usize,
    pub(crate) list_state: ListState,
    pub(crate) textarea: TextArea<'a>,
    pub(crate) page_size: usize,
    pub(crate) pending_delete: bool,
    pub(crate) yank_buffer: Option<String>,
}

impl NotepadEditorState<'static> {
    /// 復元した行とカーソル位置から編集状態を組み立てる。
    /// リスト選択と1行 textarea はここで初期化する。
    pub(crate) fn restored(lines: Vec<String>, cursor: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(cursor));
        Self {
            lines,
            cursor,
            list_state,
            textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            page_size: 1,
            pending_delete: false,
            yank_buffer: None,
        }
    }
}
