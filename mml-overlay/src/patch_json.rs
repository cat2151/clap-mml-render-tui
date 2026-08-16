//! 行頭に置く patch 指定 JSON の読み書き。
//!
//! 文字列表現は notepad 画面と同じ `{"Surge XT patch": "..."} cde` に揃える。
//! そのまま notepad の行へ貼り付けても同じ音色で鳴るようにするため。
//!
//! notepad 側の `Surge XT patch filter`（絞り込み語の持ち越し）は、オーバーレイが
//! 揮発である以上持ち越す先が無いので扱わない。既にある JSON に他のキーが入って
//! いても、ここでは patch キーだけを見て残りは捨てる。

use mmlabc_to_smf::mml_preprocessor;

/// notepad 画面と共通の JSON キー。
const PATCH_JSON_KEY: &str = "Surge XT patch";

/// 行頭 JSON を除いた MML 本体と、その開始位置（元テキスト先頭からの文字数）。
pub struct StrippedMml<'a> {
    pub mml: &'a str,
    /// 元テキストで `mml` が始まるまでの文字数。JSON が無ければ 0。
    pub offset_chars: usize,
}

/// 行頭 JSON を剥がす。
///
/// `mml_preprocessor::extract_embedded_json` は前後の空白を落とした文字列を返すため、
/// 元テキストのどこから MML 本体が始まるかは長さの差から求める。JSON が無効・未完成
/// （入力途中や手で壊した場合）なら剥がさず、全体を MML として扱う。
pub fn strip_patch_json(text: &str) -> StrippedMml<'_> {
    let preprocessed = mml_preprocessor::extract_embedded_json(text);
    if preprocessed.embedded_json.is_none() {
        return StrippedMml {
            mml: text,
            offset_chars: 0,
        };
    }
    // remaining_mml は先頭側だけを削るので、末尾は元テキストと一致する。
    // よって「元テキストの末尾 remaining 文字数ぶん」がそのまま MML 本体になる。
    let remaining_chars = preprocessed.remaining_mml.chars().count();
    let total_chars = text.chars().count();
    let offset_chars = total_chars - remaining_chars;
    let offset_bytes = text
        .char_indices()
        .nth(offset_chars)
        .map_or(text.len(), |(index, _)| index);
    StrippedMml {
        mml: &text[offset_bytes..],
        offset_chars,
    }
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

/// 行頭へ patch 指定を入れる。既にあれば上書きする。
///
/// 戻り値は「新しいテキスト」と「JSON ぶんで増えた文字数」。呼び出し側は後者で
/// カーソルを動かし、書き換えの前後で MML 本体上の位置を保つ。
pub fn set_patch_name(text: &str, patch: &str) -> (String, isize) {
    let stripped = strip_patch_json(text);
    let json = build_patch_json(patch);
    let next = format!("{json} {}", stripped.mml);
    let offset_chars = json.chars().count() + 1;
    (next, offset_chars as isize - stripped.offset_chars as isize)
}

fn build_patch_json(patch: &str) -> String {
    let patch = serde_json::to_string(patch).unwrap_or_else(|_| format!("\"{patch}\""));
    format!(r#"{{"{PATCH_JSON_KEY}": {patch}}}"#)
}

#[cfg(test)]
mod tests;
