//! カーソル行の編集を、共有 MML overlay（app 側）へ任せるための入口と書き戻し。
//!
//! notepad 自身は入力欄を持たない。`i` / `o` / `O` は「この行を編集したい」という
//! 依頼を立てるだけで、[`NotepadScreen::handle_key_event`] が
//! [`crate::NormalAction::EditLine`] として app へ返す。
//!
//! overlay の入力欄には行頭 JSON を除いた MML だけが入る。確定時は元の行の行頭 JSON を
//! **書かれていた文字列のまま**前へ付け直す。編集せずに抜けた行が 1 文字も変わらないことを
//! この作りで保つ（JSON を組み立て直すと key の順や空白が変わる）。

use mmlabc_to_smf::mml_preprocessor;

use crate::NotepadScreen;

impl NotepadScreen<'_> {
    /// カーソル行を overlay で編集したい、という依頼を立てる。
    ///
    /// 読み込み待ちの音色選択は捨てる。overlay は notepad のモードを変えないので、
    /// 残すと読み込み完了時に overlay の下で音色選択が開いてしまう。
    pub(crate) fn request_line_edit(&mut self) {
        self.pending_patch_select_open = None;
        self.line_edit_requested = true;
    }

    pub(crate) fn insert_empty_line_and_request_edit(&mut self, index: usize) {
        self.editor.lines.insert(index, String::new());
        self.editor.cursor = index;
        self.editor.list_state.select(Some(self.editor.cursor));
        self.request_line_edit();
    }

    /// 編集対象のカーソル行（行頭 JSON 込み）。
    pub fn current_line(&self) -> &str {
        &self.editor.lines[self.editor.cursor]
    }

    /// overlay の確定をカーソル行へ書き戻し、履歴へ記録して鳴らす。
    ///
    /// `body` は行頭 JSON を除いた MML。`changed_patch` は overlay で音色を選び直したときだけ
    /// `Some` で、そのときは `t` と同じく行頭 JSON の音色を差し替える（filter と effect は残る）。
    pub fn commit_line_edit(&mut self, body: &str, changed_patch: Option<&str>) {
        let cursor = self.editor.cursor;
        let original = &self.editor.lines[cursor];
        let prefix = &original[..patch_json_prefix_len(original)];
        self.editor.lines[cursor] = format!("{prefix}{body}");
        if let Some(patch_name) = changed_patch {
            let effects = self.current_line_effect_keys();
            self.replace_current_line_patch(patch_name, &effects);
        }
        let text = self.editor.lines[cursor].trim().to_string();
        if !text.is_empty() {
            self.record_notepad_history(&text);
            self.record_patch_phrase_history(&text);
            self.play_mml(text);
        }
    }

    /// `Enter` の確定後、次に編集する空行をカーソル行の下へ足す。
    pub fn insert_line_below_for_edit(&mut self) {
        self.editor
            .lines
            .insert(self.editor.cursor + 1, String::new());
        self.editor.cursor += 1;
        self.editor.list_state.select(Some(self.editor.cursor));
    }

    /// overlay の `Ctrl+O` に渡すフレーズ履歴（履歴, お気に入り）。
    ///
    /// 音色にフレーズ履歴があればそれを、無ければ notepad の履歴を使う（DAW と同じ選び方）。
    pub fn phrase_history_for_patch(&self, patch_name: Option<&str>) -> (Vec<String>, Vec<String>) {
        patch_name
            .and_then(|patch_name| self.patch_phrase_store.patches.get(patch_name))
            .filter(|state| !state.history.is_empty() || !state.favorites.is_empty())
            .map(|state| (state.history.clone(), state.favorites.clone()))
            .unwrap_or_else(|| {
                let (history, favorites) = self.phrase_history();
                (history.to_vec(), favorites.to_vec())
            })
    }
}

/// 行のうち MML 本体より前（行頭 JSON）の長さ。JSON が無い・読めないなら 0。
///
/// app 側が入力欄へ入れる本体（`cmrt_mml_overlay::patch_json::strip_patch_json`）と同じ境界で切る。
fn patch_json_prefix_len(line: &str) -> usize {
    let preprocessed = mml_preprocessor::extract_embedded_json(line);
    if preprocessed.embedded_json.is_none() {
        return 0;
    }
    preprocessed.remaining_offset
}
