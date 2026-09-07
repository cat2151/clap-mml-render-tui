//! 空白区切りの各 term を正規表現として扱う、画面横断の絞り込み条件マッチャ。
//!
//! patch 選択（`cmrt-mml-overlay`）と loop tree（`cmrt-loop-browser`）が同じ規則を
//! 共有するための単一ソース。規則は次の3つだけ:
//!
//! - 条件文字列は空白で区切り、各 term を大小無視の正規表現としてコンパイルする
//! - term 間は AND
//! - 1つの term は、渡されたフィールドの**いずれか**にマッチすれば満たされたとみなす
//!   （patch は display と category の2つ、loop tree は相対パス1つ、という違いをここで吸収する）

use regex::{Regex, RegexBuilder};

/// 空白区切りの各 term を大小無視の正規表現へコンパイルする。
///
/// 空文字列（および空白のみ）は空の `Vec` になり、[`matches_any_field`] は常に true を返す
/// ＝「絞り込みなし・全部通す」。
pub fn compile_condition(condition: &str) -> Result<Vec<Regex>, String> {
    condition
        .split_whitespace()
        .map(|term| {
            RegexBuilder::new(term)
                .case_insensitive(true)
                .build()
                .map_err(|error| error.to_string())
        })
        .collect()
}

/// 各 term が `fields` のいずれかにマッチするか（term 間 AND）。
pub fn matches_any_field(condition: &[Regex], fields: &[&str]) -> bool {
    condition
        .iter()
        .all(|regex| fields.iter().any(|field| regex.is_match(field)))
}

/// 条件が正規表現としてコンパイルできるか。打鍵途中の `kick|` などを弾くのに使う。
pub fn is_valid_condition(condition: &str) -> bool {
    compile_condition(condition).is_ok()
}

#[cfg(test)]
mod tests;
