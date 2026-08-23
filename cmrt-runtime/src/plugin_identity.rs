//! 使用中プラグインの同定。
//!
//! 「いま何のプラグインを使っているか」で振る舞いを変えたい場所（voicing 判定の
//! データ源、キャッシュの置き場）が複数あるので、判定規則をここ 1 か所に置く。

// プラグインの ID とファイル名、および「その音色は state file か cartridge か」の判定は
// play server repo 側が単一ソース。「その材料で Config をどう判定するか」だけが
// TUI 側の知識としてここに残る。
//
// `patch_form_of` / `PatchForm` を素通しで再輸出しているのは、画面 crate（tui-core など）が
// config crate だけを見ればよい形を保つため。
pub use cmrt_server_config::{
    patch_form_of, plugin_file_stem, PatchForm, DEXED_PLUGIN_ID, FLOE_PLUGIN_ID,
    SFORZANDO_PLUGIN_ID, SURGE_XT_PLUGIN_ID, VAPORIZER2_PLUGIN_ID,
};

use crate::{default_plugin_path, Config};

/// このプラグインが Surge XT か。
///
/// `voicing_shared_source` / `voicing_override_source` が指す JSON は Surge の
/// patch 表示パスをキーにした Surge 専用データなので、Surge のときだけ読む。
///
/// `plugin_id` はプロファイル解決後に埋まる。書かれていない config は
/// `active_plugin` が無かった時代のもの（＝Surge 専用）か、`[plugins.*]` に
/// `plugin_id` を書かなかったもののどちらかなので、`plugin_path` のファイル名で
/// 見分ける。
///
/// 1 プロセスへ複数のプラグインを載せると「Config 全体が Surge か」では問えなくなる
/// （音色ごとに違う）。プロファイル 1 つぶんの材料だけを受け取る形にしてある。
pub fn is_surge_xt_plugin(plugin_id: Option<&str>, plugin_path: &str) -> bool {
    match plugin_id {
        Some(id) => id == SURGE_XT_PLUGIN_ID,
        None => plugin_file_stem(plugin_path)
            .eq_ignore_ascii_case(&plugin_file_stem(default_plugin_path())),
    }
}

/// このプラグインが Vaporizer2 か。
///
/// 用途別カテゴリの組み込み既定（[`crate::PatchRoles::builtin_for`]）を選ぶのに使う。
/// Vaporizer2 の音色置き場は `.vvp` のフラットな 1 階層で、カテゴリは
/// **ファイル名先頭 2 文字のコード**から取る独自の体系なので、Surge のカテゴリ名を
/// 当てると候補が全滅する。
///
/// `plugin_id` が書かれていない config でも判定できるよう、ファイル名も見る。
/// Vaporizer2 に既定の `plugin_path` はあるが `patches_dirs` の既定値は無いので、
/// このプラグインを使う config は必ず `[plugins.Vaporizer2]` を書いている
/// ＝ `plugin_id` が埋まっているのが普通。ファイル名判定は保険。
///
/// [`is_surge_xt_plugin`] と違い既定 `plugin_path` との比較にしないのは、
/// **既定プラグインが Vaporizer2 でない config**（`plugin_path` が Surge や Dexed）でも
/// カタログの 2 つめ以降として現れるため。
pub fn is_vaporizer2_plugin(plugin_id: Option<&str>, plugin_path: &str) -> bool {
    match plugin_id {
        Some(id) => id == VAPORIZER2_PLUGIN_ID,
        None => plugin_file_stem(plugin_path)
            .to_lowercase()
            .contains("vaporizer"),
    }
}

/// このプラグインが Floe か。ID を優先し、未指定時だけファイル名で判定する。
pub fn is_floe_plugin(plugin_id: Option<&str>, plugin_path: &str) -> bool {
    match plugin_id {
        Some(id) => id == FLOE_PLUGIN_ID,
        None => plugin_file_stem(plugin_path)
            .to_lowercase()
            .contains("floe"),
    }
}

/// このプラグインが sforzando か。ID を優先し、未指定時だけファイル名で判定する。
pub fn is_sforzando_plugin(plugin_id: Option<&str>, plugin_path: &str) -> bool {
    match plugin_id {
        Some(id) => id == SFORZANDO_PLUGIN_ID,
        None => plugin_file_stem(plugin_path)
            .to_lowercase()
            .contains("sforzando"),
    }
}

impl Config {
    /// 既定プラグイン（音色無指定の行が鳴るもの）が Surge XT か。
    ///
    /// **音色ごとの判定にはこれを使わないこと。** カタログに複数プラグインが並ぶと
    /// 答えが音色によって変わる。その用途には `PatchPlugins`（tui-core）を通す。
    pub fn is_surge_xt(&self) -> bool {
        is_surge_xt_plugin(self.plugin_id.as_deref(), &self.plugin_path)
    }
}

#[cfg(test)]
mod tests;
