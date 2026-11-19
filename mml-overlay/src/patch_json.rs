//! notepad 画面の行と同じ表現で書かれた、行頭の patch 指定 JSON を読む。
//!
//! オーバーレイ自身は音色をテキストへ書かない（入力欄には MML だけを置き、音色は
//! `Ctrl+T` で別に持つ）。ここが要るのは履歴からフレーズを取り込むときで、notepad が
//! 書いた `{"patch": "..."} cde` を音色と MML 本体へ分けるために使う。
//!
//! 既にある JSON に他のキーが入っていても、[`patch_name`] は patch キーだけを見て残りは捨てる。

use clap_mml_play_server_core::{patch_display_from_embedded_json, EFFECT_CHAIN_JSON_KEY};
use mmlabc_to_smf::mml_preprocessor;

/// 行頭 JSON を除いた MML 本体。
///
/// 本家が「元テキストのどこから MML 本体が始まるか」を返すので、それで切る。
/// JSON が無効・未完成なら剥がさず、全体を MML として扱う。
pub fn strip_patch_json(text: &str) -> &str {
    let preprocessed = mml_preprocessor::extract_embedded_json(text);
    if preprocessed.embedded_json.is_none() {
        return text;
    }
    &text[preprocessed.remaining_offset..]
}

/// 行頭 JSON から patch の表示名を読む。旧キー・タグの無い旧形式は `None`。
pub fn patch_name(text: &str) -> Option<String> {
    let preprocessed = mml_preprocessor::extract_embedded_json(text);
    let json = preprocessed.embedded_json?;
    patch_display_from_embedded_json(&serde_json::from_str::<serde_json::Value>(&json).ok()?)
}

/// 行頭 JSON の `"effects after instrument"` の値を JSON 文字列で返す。無ければ空。
pub fn effect_chain_json(text: &str) -> Result<String, String> {
    let Some(json) = mml_preprocessor::extract_embedded_json(text).embedded_json else {
        return Ok(String::new());
    };
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|error| format!("行頭 JSON を読めません: {error}: {json}"))?;
    match value.get(EFFECT_CHAIN_JSON_KEY) {
        Some(chain) => serde_json::to_string(chain).map_err(|error| error.to_string()),
        None => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests;
