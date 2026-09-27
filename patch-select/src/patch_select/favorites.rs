//! host が注入する音色 favorite。保存は host が持ち、ここは表示と絞り込みだけ。

use super::*;

impl PatchSelect<'_> {
    /// favorite を差し替える。`★ Favorite` を選んでいれば一覧を作り直し、
    /// 選択中の音色が残っていればそこへ留まる。
    pub fn set_favorites(&mut self, favorites: Vec<String>) {
        self.favorites = favorites;
        self.prepared_presets
            .set_favorites(&self.all, &self.favorites);
        if self.presets()[self.preset_cursor].is_favorite {
            self.update_filter();
        }
    }

    /// ★ 列に印を付けるか。
    pub fn is_favorite(&self, patch: &str) -> bool {
        self.favorites.iter().any(|favorite| favorite == patch)
    }
}
