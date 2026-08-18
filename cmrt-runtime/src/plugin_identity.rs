//! 使用中プラグインの同定。
//!
//! 「いま何のプラグインを使っているか」で振る舞いを変えたい場所（voicing 判定の
//! データ源、キャッシュの置き場）が複数あるので、判定規則をここ 1 か所に置く。

// プラグインの ID とファイル名は play server repo 側が単一ソース。
// 「その材料で Config をどう判定するか」だけが TUI 側の知識としてここに残る。
pub use cmrt_server_config::{plugin_file_stem, DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};

use crate::{default_plugin_path, Config};

impl Config {
    /// 使用中プラグインが Surge XT か。
    ///
    /// `voicing_shared_source` / `voicing_override_source` が指す JSON は Surge の
    /// patch 表示パスをキーにした Surge 専用データなので、Surge のときだけ読む。
    ///
    /// `plugin_id` はプロファイル解決後に埋まる。書かれていない config は
    /// `active_plugin` が無かった時代のもの（＝Surge 専用）か、`[plugins.*]` に
    /// `plugin_id` を書かなかったもののどちらかなので、`plugin_path` のファイル名で
    /// 見分ける。
    pub fn is_surge_xt(&self) -> bool {
        match self.plugin_id.as_deref() {
            Some(id) => id == SURGE_XT_PLUGIN_ID,
            None => plugin_file_stem(&self.plugin_path)
                .eq_ignore_ascii_case(&plugin_file_stem(default_plugin_path())),
        }
    }
}

#[cfg(test)]
mod tests;
