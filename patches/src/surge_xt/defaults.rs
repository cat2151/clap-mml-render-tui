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

/// drum 4 役に共通の既定カテゴリ。
///
/// Surge のカテゴリは打楽器を `Percussion`（factory）と `Drums`（3rdparty に多い）の
/// 2 つでしか分けておらず、kick と hi-hat をカテゴリでは分離できない。役割の分離は
/// キーワード（下記）が受け持つので、カテゴリは4役とも同じで良い。
pub const DEFAULT_DRUM_PATCH_CATEGORY_NAMES: [&str; 2] = ["Percussion", "Drums"];

/// kick（bass drum）に使う patch の名前キーワード。小文字化した patch パスへの部分一致。
///
/// `bd` のような短い語は他の語に埋もれて誤爆するので入れていない。
pub const DEFAULT_KICK_PATCH_KEYWORDS: [&str; 2] = ["kick", "bass drum"];
/// snare に使う patch の名前キーワード。clap も snare 相当の役として拾う。
pub const DEFAULT_SNARE_PATCH_KEYWORDS: [&str; 3] = ["snare", "rimshot", "clap"];
/// hi-hat に使う patch の名前キーワード。`hat` だけで `hi-hat` / `hihat` も拾える。
pub const DEFAULT_HIHAT_PATCH_KEYWORDS: [&str; 3] = ["hat", "hi-hat", "hihat"];
