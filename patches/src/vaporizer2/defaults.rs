//! 用途ごとの既定カテゴリ名（Vaporizer2 の展開名）。
//!
//! config.toml のひな形（`cmrt-runtime`）はここを参照して行を組み立てる。
//! ユーザーの既存 config.toml には追記されないので、ここの既定が実質の挙動になる。
//!
//! 名前は [`super::layout::PATCH_CATEGORY_CODES`] の**展開名**で書く。2 文字コードを
//! 書くと `patch_category()` の戻り（展開名）と照合できない。

/// chord mode の和音に使う patch のカテゴリ。
///
/// 和音が成立する音色を広めに拾う。`Chord`(4 件) だけでは少なすぎるので、
/// 実データで件数の多い `Pad`(69) / `Synth`(104) を土台にしてある。
/// **Mono プリセット 144 件はカテゴリでは外れない。** 外すのは voicing 側の仕事で、
/// `.vvp` の `m_uPolyMode` を読んで絞る（Stage 7）。
pub const DEFAULT_CHORD_PATCH_CATEGORY_NAMES: [&str; 5] =
    ["Pad", "Chord", "Organ", "Synth", "Atmosphere"];

/// chord mode の bass 行に使う patch のカテゴリ。和音と違い単音なので mono patch でよい。
pub const DEFAULT_BASS_PATCH_CATEGORY_NAMES: [&str; 1] = ["Bass"];

/// chord mode のアルペジオ行（4 voice の行）に使う patch のカテゴリ。
/// 音程が意味を持つ行なので、打楽器や効果音のカテゴリは既定から外してある。
pub const DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES: [&str; 5] =
    ["Arpeggio", "Plucked", "Bell", "Mallet", "Trancegate"];

/// drum 4 役に共通の既定カテゴリ。
///
/// **実データの `Drum` は 9 件しかなく `Drum kit` は 0 件**なので、drum 4 役は事実上
/// ほぼ空になる。それでよい（役に当たる音色が無いプラグインを無理に鳴らすより、
/// 候補が出ないほうが分かる）。`PatchRole::Free` はカテゴリで絞らないので全滅しない
/// （`docs/adr/0007-patch-role-defaults-three-layers.md`）。
pub const DEFAULT_DRUM_PATCH_CATEGORY_NAMES: [&str; 2] = ["Drum", "Drum kit"];

// kick / snare / hi-hat の判別は**カテゴリではなく patch 名のキーワード**で行う。
// 語そのものは太鼓の一般名でプラグインに依らないので、Surge のぶんを単一ソースとして
// 引く（同じ語をもう一組書くと、片方だけ直したときに役が食い違う）。4 つめの
// プラグインを足すときは、中立の置き場へ移すことを検討すること。
pub use crate::surge_xt::{
    DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS, DEFAULT_SNARE_PATCH_KEYWORDS,
};
