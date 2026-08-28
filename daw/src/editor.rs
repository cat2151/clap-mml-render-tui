use super::NormalCellUndo;

/// DAW の編集対象グリッドと、グリッド操作にだけ必要な一時状態。
///
/// 再生・overlay の runtime から切り離し、編集処理が参照すべき状態を明確にする。
pub(crate) struct DawEditorState {
    /// `data[track][measure]`: measure 0 は音色列。
    pub(crate) data: Vec<Vec<String>>,
    pub(crate) cursor_track: usize,
    pub(crate) cursor_measure: usize,
    /// 行 index の総数。0 = Tempo / 1 = chord 行 / 2 以降が演奏 track
    /// （対応は `crate::tracks` を参照）。
    pub(crate) tracks: usize,
    /// measure 0 は音色列、measure 1 以降は通常小節。
    pub(crate) measures: usize,
    pub(crate) yank_buffer: Option<String>,
    pub(crate) pending_delete: bool,
    /// `u` で 1 回だけ取り消せる直前の編集。paste は 1 セル、chord wizard（`G`）は
    /// 複数セルを 1 操作で書くので、まとめて 1 つの塊として持つ。
    pub(crate) cell_undo: Option<Vec<NormalCellUndo>>,
    /// `C` で chord 行へ跳ぶ直前にいた行。もう一度 `C` を押すとここへ戻る。
    pub(crate) chord_jump_return_track: Option<usize>,
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
            cell_undo: None,
            chord_jump_return_track: None,
        }
    }
}
