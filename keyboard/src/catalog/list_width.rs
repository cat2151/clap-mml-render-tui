//! Patches pane の幅の元になる、一覧の最長の音色名の表示幅。
//!
//! 測るのは一覧を「確定」したときだけ。catalog の読み込み、Patches pane へ focus が入ったとき、
//! 絞り込み欄を閉じたとき、plugin solo/mute を選んだとき。Role / Preset を動かしている間や絞り込みの入力中は測らないので、
//! キーを押すたびに pane の幅が揺れない。

use ratatui::text::Span;

use super::KeyboardPatchCatalog;

impl KeyboardPatchCatalog {
    /// 最後に測った時点の一覧の最長の音色名の表示幅。測った一覧が空なら 0。
    pub fn patch_name_width(&self) -> usize {
        self.patch_name_width
    }

    /// 今の一覧で測り直す。
    pub(crate) fn measure_patch_list(&mut self) {
        self.patch_name_width = self
            .patches()
            .map(|entry| Span::raw(entry.display()).width())
            .max()
            .unwrap_or(0);
    }
}
