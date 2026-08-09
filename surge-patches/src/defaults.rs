//! 用途ごとの既定カテゴリ名。
//!
//! config.toml のひな形（`cmrt-runtime`）はここを参照して行を組み立てる。
//! ユーザーの既存 config.toml には追記されないので、ここの既定が実質の挙動になる。

/// chord mode の和音に使う patch のカテゴリ（patch パスのカテゴリ階層と大文字小文字を無視して照合）。
pub const DEFAULT_CHORD_PATCH_CATEGORY_NAMES: [&str; 4] = ["Keys", "Organs", "Pads", "Polysynths"];
/// chord mode の bass 行に使う patch のカテゴリ。和音と違い単音なので mono patch でよい。
pub const DEFAULT_BASS_PATCH_CATEGORY_NAMES: [&str; 1] = ["Basses"];
/// chord mode のアルペジオ行（4 voice の行）に使う patch のカテゴリ。
/// 音程が意味を持つ行なので、打楽器や効果音のカテゴリは既定から外してある。
pub const DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES: [&str; 10] = [
    "Bells", "Brass", "Guitars", "Keys", "Leads", "Mallets", "Modelled", "MPE", "Organs", "Plucks",
];
