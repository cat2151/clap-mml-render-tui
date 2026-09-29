//! app 側・crate 内のテストが、画面の内部状態を直接差し替えるための口。

use crate::{NotepadScreen, TuiRenderJobStatus};

impl NotepadScreen<'_> {
    /// 編集行を差し替える（テスト用）。
    pub fn set_session_lines_for_test(&mut self, lines: Vec<String>) {
        self.editor.lines = lines;
    }

    /// カーソル行を差し替える（テスト用）。リスト選択も追従させる。
    pub fn set_session_cursor_for_test(&mut self, cursor: usize) {
        self.editor.cursor = cursor;
        self.editor.list_state.select(Some(cursor));
    }

    /// 音色×フレーズの履歴・favorite を差し替える（テスト用）。
    pub fn set_patch_phrase_store_for_test(&mut self, store: cmrt_history::PatchPhraseStore) {
        self.patch_phrase_store = store;
    }

    pub fn test_set_render_job_status(
        &self,
        mml: impl Into<String>,
        status: Option<TuiRenderJobStatus>,
    ) {
        self.playback.render_queue.set_test_job_status(mml, status);
    }
}
