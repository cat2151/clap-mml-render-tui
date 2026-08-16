//! notepad 画面の行と同じ表現で書かれた、行頭の patch 指定 JSON を読む。
//!
//! オーバーレイ自身は音色をテキストへ書かない（入力欄には MML だけを置き、音色は
//! `Ctrl+T` で別に持つ）。ここが要るのは履歴からフレーズを取り込むときで、notepad が
//! 書いた `{"Surge XT patch": "..."} cde` を音色と MML 本体へ分けるために使う。
//!
//! 既にある JSON に他のキーが入っていても、ここでは patch キーだけを見て残りは捨てる。

use mmlabc_to_smf::mml_preprocessor;

/// notepad 画面と共通の JSON キー。
const PATCH_JSON_KEY: &str = "Surge XT patch";

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

/// 行頭 JSON から patch 名を読む。
pub fn patch_name(text: &str) -> Option<String> {
    let preprocessed = mml_preprocessor::extract_embedded_json(text);
    let json = preprocessed.embedded_json?;
    serde_json::from_str::<serde_json::Value>(&json)
        .ok()?
        .get(PATCH_JSON_KEY)?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests;
