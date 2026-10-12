use ratatui::widgets::ListState;

/// notepad の編集対象と、vim風編集にだけ必要な一時状態。
pub(crate) struct NotepadEditorState {
    pub(crate) lines: Vec<String>,
    pub(crate) cursor: usize,
    pub(crate) list_state: ListState,
    pub(crate) page_size: usize,
    pub(crate) pending_delete: bool,
    pub(crate) yank_buffer: Option<String>,
}

impl NotepadEditorState {
    /// 復元した行とカーソル位置から編集状態を組み立てる。
    /// リスト選択はここで初期化する。
    pub(crate) fn restored(lines: Vec<String>, cursor: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(cursor));
        Self {
            lines,
            cursor,
            list_state,
            page_size: 1,
            pending_delete: false,
            yank_buffer: None,
        }
    }
}
