//! `.vvp` のファイル名 2 文字コードからカテゴリを読む。

/// Vaporizer2 の音色置き場には factory / 3rdparty の別が無い。
const SORT_PRIORITY: u8 = 0;

/// Vaporizer2 の音色ファイルの拡張子。
const VVP_EXTENSION: &str = ".vvp";

const PATH_SEPARATORS: [char; 2] = ['/', '\\'];

/// カテゴリコードの長さ（`AR Accent Arp.vvp` の `AR`）。
const CATEGORY_CODE_LEN: usize = 2;

/// カテゴリコード → 画面と config に出す展開名。
///
/// 出どころは Vaporizer2 の `VASTPresetData.cpp:12` のコード表と、実プリセット 460 件の
/// `PatchCategory` 属性の実測（**460/460 がファイル名先頭 2 文字と一致した**ので、
/// XML を開かなくてもカテゴリが取れる）の和集合。実データにしか無かった `RI`（Riser）と、
/// コード表にしか無い 9 個（`DK` / `DL` / `IN` / `KB` / `OC` / `RD` / `ST` / `SQ` / `WW`）を
/// 両方入れてある。後者はユーザーが自分で保存したプリセットで出てくる。
///
/// **展開名を画面と config の両方で使う。** 2 文字コードのままでは
/// `[plugins.Vaporizer2]` の `chord_patch_categories = ['PD', 'CH']` が読めない。
pub const PATCH_CATEGORY_CODES: [(&str, &str); 28] = [
    ("AR", "Arpeggio"),
    ("AT", "Atmosphere"),
    ("BA", "Bass"),
    ("BL", "Bell"),
    ("BR", "Brass"),
    ("CH", "Chord"),
    ("DK", "Drum kit"),
    ("DL", "Drum loop"),
    ("DR", "Drum"),
    ("FX", "Effect"),
    ("GT", "Guitar"),
    ("IN", "Instrument"),
    ("KB", "Keyboard"),
    ("LD", "Lead"),
    ("MA", "Mallet"),
    ("OC", "Orchestral"),
    ("OR", "Organ"),
    ("PD", "Pad"),
    ("PL", "Plucked"),
    ("PN", "Piano"),
    ("RD", "Reed"),
    ("RI", "Riser"),
    ("SQ", "Sequence"),
    ("ST", "String"),
    ("SY", "Synth"),
    ("TG", "Trancegate"),
    ("VC", "Vocal"),
    ("WW", "Woodwind"),
];

/// カテゴリコードの展開名。表に無いコードは `None`。
///
/// 大文字小文字を無視するのは、patch 文字列が**小文字化された照合用の形でも
/// 渡ってくる**ため（`crate::grouping` はグループのキーを小文字側から、
/// 表示名を display 側から作る）。ここで同じ展開名を返さないと、同じカテゴリが
/// 2 つに割れる。
pub fn category_name_for_code(code: &str) -> Option<&'static str> {
    PATCH_CATEGORY_CODES
        .iter()
        .find(|(known, _)| code.eq_ignore_ascii_case(known))
        .map(|(_, name)| *name)
}

/// この patch 文字列が Vaporizer2 の音色を指しているか。
///
/// `.syx` 側の判定（play server repo の `is_vvp_patch_path`）と同じく
/// **どのコンポーネントに現れても真**にしてある。`.vvp` は 1 ファイル = 1 音色なので
/// 末尾だけ見ても足りるが、規則を 2 種類にすると覚え方が増える。
pub(crate) fn has_vvp_extension(path: &str) -> bool {
    path.split(PATH_SEPARATORS).any(component_is_vvp)
}

fn component_is_vvp(component: &str) -> bool {
    match component.rfind('.') {
        // `.vvp` だけのファイル名（stem が無い）は音色ではない。
        Some(dot) if dot > 0 => component[dot..].eq_ignore_ascii_case(VVP_EXTENSION),
        _ => false,
    }
}

/// カテゴリ順ソート用に `(カテゴリ, 供給元の優先度, vendor, 残りのパス)` へ分解する。
/// vendor は常に空。
///
/// カテゴリはファイル名から作る**派生値**でパスの一部ではないので、「残りのパス」は
/// 切り詰めずにパス全体を返す。カテゴリが同じ patch 同士の並びはファイル名順になり、
/// どのみち先頭 2 文字は共通なので切り詰めても結果は変わらない。
pub(crate) fn category_sort_parts(path: &str) -> (&str, u8, &str, &str) {
    (category(path), SORT_PRIORITY, "", path)
}

/// パス順ソート用に `(供給元の優先度, パス)` へ分解する。剥がす prefix が無い。
pub(crate) fn path_sort_parts(path: &str) -> (u8, &str) {
    (SORT_PRIORITY, path)
}

/// ファイル名先頭 2 文字を表で引いたカテゴリ。**表に無いコードは生の 2 文字を返す**
/// （ユーザーが独自のコードで保存していても、そのコードでグループ化されて見える）。
fn category(path: &str) -> &str {
    let file_name = path.rsplit(PATH_SEPARATORS).next().unwrap_or(path);
    let code = leading_code(strip_vvp_extension(file_name));
    category_name_for_code(code).unwrap_or(code)
}

fn strip_vvp_extension(file_name: &str) -> &str {
    match file_name.rfind('.') {
        Some(dot) if dot > 0 && file_name[dot..].eq_ignore_ascii_case(VVP_EXTENSION) => {
            &file_name[..dot]
        }
        _ => file_name,
    }
}

/// 先頭 2 **文字**（バイトではない）。2 文字に満たなければ全体。
fn leading_code(stem: &str) -> &str {
    match stem.char_indices().nth(CATEGORY_CODE_LEN) {
        Some((end, _)) => &stem[..end],
        None => stem,
    }
}
