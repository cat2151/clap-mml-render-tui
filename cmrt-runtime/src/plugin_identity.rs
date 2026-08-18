//! 使用中プラグインの同定。
//!
//! 「いま何のプラグインを使っているか」で振る舞いを変えたい場所（voicing 判定の
//! データ源、キャッシュの置き場）が複数あるので、判定規則をここ 1 か所に置く。

use std::path::PathBuf;

use crate::{default_plugin_path, Config};

pub const SURGE_XT_PLUGIN_ID: &str = "org.surge-synth-team.surge-xt";
pub const DEXED_PLUGIN_ID: &str = "com.digital-suburban.dexed";

/// `plugin_path` のファイル名から拡張子を落としたもの（`Surge XT.clap` → `Surge XT`）。
///
/// `plugin_id` は `active_plugin` / `[plugins.*]` を書いた config にしか無いのに対し、
/// `plugin_path` はどの書き方でも必ず埋まる。plugin_id が無い config の同定に使う。
pub fn plugin_file_stem(plugin_path: &str) -> String {
    PathBuf::from(plugin_path.trim())
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string()
}

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
