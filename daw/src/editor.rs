use super::NormalPasteUndo;

/// DAW の編集対象グリッドと、グリッド操作にだけ必要な一時状態。
///
/// 再生・overlay の runtime から切り離し、編集処理が参照すべき状態を明確にする。
pub(crate) struct DawEditorState {
    /// `data[track][measure]`: measure 0 は音色列。
    pub(crate) data: Vec<Vec<String>>,
    pub(crate) cursor_track: usize,
    pub(crate) cursor_measure: usize,
    /// track 0 はヘッダ／テンポ、track 1 以降は演奏 track。
    pub(crate) tracks: usize,
    /// measure 0 は音色列、measure 1 以降は通常小節。
    pub(crate) measures: usize,
    pub(crate) yank_buffer: Option<String>,
    pub(crate) pending_delete: bool,
    pub(crate) paste_undo: Option<NormalPasteUndo>,
}

impl DawEditorState {
    pub(crate) fn new(
        data: Vec<Vec<String>>,
        cursor_track: usize,
        cursor_measure: usize,
        tracks: usize,
        measures: usize,
    ) -> Self {
        Self {
            data,
            cursor_track,
            cursor_measure,
            tracks,
            measures,
            yank_buffer: None,
            pending_delete: false,
            paste_undo: None,
        }
    }
}
