//! chord 行のセルから、演奏 track で鳴らす MML を生成する。
//!
//! # 何をどう組み立てるか
//!
//! chord2mml へ渡す 1 小節ぶんの入力は、次の順で連結する。
//!
//! ```text
//! {chord 行の init セル} {track の init セルの指定} | {chord 行の当該 measure セル} |
//! ```
//!
//! - **conductor（track 0）は連結しない。** `t120` のような MML の body を混ぜると
//!   chord2mml が必ず `Syntax error` になる（実測済み）。
//! - **半角の縦棒で囲む。** 囲まないと 1 コード = 全音符になり、1 小節に 2 コード書くと
//!   2 小節ぶんの長さに溢れる。囲むと小節内で自動等分される。
//! - 連結順が「chord 行 init → track init」なのは、chord2mml の `key:` が**後勝ち**だから。
//!   曲全体の key は chord 行 init に書き、その track だけ変えたければ track init に書く、
//!   という 1 つのルールで説明できる。
//!
//! chord 行のセルが空の小節は、convert を呼ばずに空 MML を返す
//! （chord2mml は空入力を `Err` にするため）。
//!
//! # ここで使う API
//!
//! **`cmrt_chord::parse_chord_progression` を使ってはいけない。**
//! あちらは「key は先頭に 1 つ必須」という grid sequencer 用の制約を足した API で、
//! key 省略時は暗黙に C とするこの画面の仕様と衝突する。使うのは
//! `chord2mml_core::convert` 直。

/// 演奏 track の init セルの JSON に置く、生成対象であることを表すキー。
///
/// **このキーが存在すること**だけが判定で、値は chord2mml へ渡す任意の文字列。
pub(crate) const GENERATE_FROM_CHORD_TRACK_KEY: &str = "generate from chord track";

/// chord2mml へ渡す入力文字列を組み立てる。
///
/// chord 行の当該セルが空なら `None`（変換にかけるものが無い）。
///
/// - `chord_init`: chord 行の init セル（measure 0）。`key:G` などの chord2mml 文字列。
/// - `track_directive`: 演奏 track の init セルの `"generate from chord track"` の値。
///   `close` / `drop2` / `octave down` / `key:B` など、chord2mml が認識する任意の文字列。
///   **cmrt 側では検証しない**（語彙を持たない）。空文字でもよい。
/// - `chord_cell`: chord 行の当該 measure のセル。`I-IV-V` など。
pub(crate) fn chord2mml_input(
    chord_init: &str,
    track_directive: &str,
    chord_cell: &str,
) -> Option<String> {
    cmrt_chord::chord_cell_input(chord_init, track_directive, chord_cell)
}

/// chord 行のセルから、演奏 track の 1 小節ぶんの MML を生成する。
///
/// 空セルのときと、chord2mml が `Err` を返したときは空文字列を返す
/// （その小節がその track で無音になるだけで、演奏は止まらない）。
/// `Err` のときはログに残すが、この段階では UI には出さない。
pub(crate) fn generate_mml_from_chord_cell(
    chord_init: &str,
    track_directive: &str,
    chord_cell: &str,
) -> String {
    let Some(input) = chord2mml_input(chord_init, track_directive, chord_cell) else {
        return String::new();
    };

    match chord2mml_core::convert(&input) {
        Ok(mml) => mml,
        Err(error) => {
            crate::log_line(&format!(
                "chord track: convert failed: input={input:?} error={error}"
            ));
            String::new()
        }
    }
}

/// コード進行を 1 小節ぶんずつ（= 1 コードずつ）に切り分ける。
///
/// # なぜ必要か
///
/// grid は **1 セル = 1 小節**で、chord 行もその 1 行なので、chord 行の 1 セルには
/// 原則 1 コードしか入らない。`I-IV-V-I` をまるごと 1 セルへ書くと、
/// [`chord2mml_input`] が縦棒で囲むぶん 1 小節の中で 4 等分され、**時間軸が 1/4 に
/// 圧縮される**（`c4eg f4a<c g4b<d c4eg`）。カタログの進行を chord 行へ配るときは、
/// 必ずここで切ってから 1 小節に 1 つずつ書く。
///
/// # ハイフンで割ってはいけない
///
/// ハイフンは区切りとして働かない位置にも現れる（`cmrt_chord` の
/// `a_hyphen_that_is_part_of_a_chord_name_is_not_a_separator` 参照）。区切りの判定は
/// chord2mml のパーサだけが正しく行えるので、`parse` の結果から拾う。
///
/// # 綴りは chord2mml の正規形になる
///
/// 拾うのはパーサが正規化したあとの綴りなので、`vi` は `VIm` として返る
/// （実測: `close | vi |` と `close | VIm |` はどちらも `v11/*|*/'a1<ce'/*|*/`）。
/// 鳴る音は同じで、正規形は再入力しても同じに読めるので、そのまま chord 行へ書く。
///
/// 解釈できない入力・コードが 1 つも無い入力では空 `Vec` を返す。
pub(crate) fn split_progression_into_measures(degrees: &str) -> Vec<String> {
    let degrees = degrees.trim();
    if degrees.is_empty() {
        return Vec::new();
    }
    let Ok(parsed) = chord2mml_core::parse(degrees) else {
        return Vec::new();
    };
    parsed
        .items()
        .iter()
        .filter_map(|item| match item {
            chord2mml_core::ParsedItem::Chord { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
