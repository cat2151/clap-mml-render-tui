//! patch 文字列を、それを載せているプラグインの patch 体系で読む。
//!
//! カタログには複数プラグインの音色が並ぶ（`docs/adr/0005-mixed-catalog-on-by-default.md`）ので、
//! 「カテゴリはどこか」「どの順で並べるか」の答えはプラグインごとに違う。ここは
//! **どの体系で読むかを patch 文字列の形から決めるだけ**で、体系そのものは
//! [`crate::surge_xt`] / [`crate::cartridge`] / [`crate::vaporizer2`] が持つ。

use crate::{cartridge, floe, surge_xt, vaporizer2};

/// patch 文字列をどの体系で読むか。
///
/// 判別材料は patch 文字列の形だけ。これは display 文字列＝永続 ID を変えずに
/// プラグインを判別するための決定（`docs/adr/0001-patch-string-decides-the-plugin.md`）に従っている。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchLayout {
    /// `patches_factory/<category>/` または `patches_3rdparty/<vendor>/<category>/`。
    SurgeXt,
    /// `<cartridge>.syx/<voice>` のように、先頭セグメントがそのままカテゴリになる形。
    ///
    /// **prefix 抜きで保存された Surge の名前もここへ落ちる。** どちらも先頭
    /// セグメントをカテゴリとして読み、供給元の優先度も 0 なので結果は変わらない。
    Cartridge,
    /// `<...>/<コード> <名前>.vvp`。ファイル名先頭 2 文字のコードがカテゴリになる。
    Vaporizer2,
    /// `<category>/<name>.floe-preset`。先頭ディレクトリがカテゴリになる。
    Floe,
}

impl PatchLayout {
    /// patch 文字列の形から体系を選ぶ。
    ///
    /// **`.vvp` の判定を先に置く。** Vaporizer2 の音色置き場を Surge の
    /// `patches_factory/` の下に置いているユーザーがいても、拡張子のほうが強い材料。
    /// 残りは今までどおり「Surge の prefix があるか」で分かれる。
    pub fn of(path: &str) -> Self {
        let path = path.trim_matches('/');
        if floe::has_floe_preset_extension(path) {
            Self::Floe
        } else if vaporizer2::has_vvp_extension(path) {
            Self::Vaporizer2
        } else if surge_xt::has_known_prefix(path) {
            Self::SurgeXt
        } else {
            Self::Cartridge
        }
    }
}

pub(crate) fn split_first_path_segment(path: &str) -> (&str, &str) {
    path.split_once('/').unwrap_or((path, ""))
}

/// カテゴリ順ソート用に `(カテゴリ, 供給元の優先度, vendor, 残りのパス)` へ分解する。
pub(crate) fn patch_category_sort_parts(path: &str) -> (&str, u8, &str, &str) {
    let path = path.trim_matches('/');
    match PatchLayout::of(path) {
        PatchLayout::SurgeXt => surge_xt::category_sort_parts(path),
        PatchLayout::Cartridge => cartridge::category_sort_parts(path),
        PatchLayout::Vaporizer2 => vaporizer2::category_sort_parts(path),
        PatchLayout::Floe => floe::category_sort_parts(path),
    }
}

/// パス順ソート用に `(供給元の優先度, prefix を除いた残りのパス)` へ分解する。
pub(crate) fn patch_path_sort_parts(path: &str) -> (u8, &str) {
    let path = path.trim_matches('/');
    match PatchLayout::of(path) {
        PatchLayout::SurgeXt => surge_xt::path_sort_parts(path),
        PatchLayout::Cartridge => cartridge::path_sort_parts(path),
        PatchLayout::Vaporizer2 => vaporizer2::path_sort_parts(path),
        PatchLayout::Floe => floe::path_sort_parts(path),
    }
}

/// patch パスのカテゴリ階層を返す。Surge なら `patches_factory/<category>/` または
/// `patches_3rdparty/<vendor>/<category>/`、カートリッジならカートリッジ名、
/// `.vvp` ならファイル名先頭 2 文字のコードを展開した名前。
pub fn patch_category(path: &str) -> &str {
    patch_category_sort_parts(path).0
}

/// patch がカテゴリ一覧のいずれかに属するか。大文字小文字は無視する。
/// カテゴリ一覧が空なら「絞り込まない」とみなして常に true。
///
/// Surge のカテゴリ名には `Bass`（単数）と `Basses`（複数）が併存するような表記ゆれが
/// あるが、正規化を足すと既存の config.toml の設定が壊れるので大文字小文字だけを無視する。
pub fn patch_matches_categories(path: &str, categories: &[String]) -> bool {
    if categories.is_empty() {
        return true;
    }
    let category = patch_category(path);
    categories
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(category))
}

#[cfg(test)]
mod tests;
