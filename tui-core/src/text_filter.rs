//! 空白区切りの各 term を正規表現として扱う、画面横断の絞り込み条件マッチャ。
//!
//! patch 選択（`cmrt-mml-overlay`）と loop tree（`cmrt-loop-browser`）が同じ規則を
//! 共有するための単一ソース。規則は次の3つだけ:
//!
//! - 条件文字列は空白で区切り、各 term を大小無視の正規表現としてコンパイルする
//! - term 間は AND
//! - 1つの term は、渡されたフィールドの**いずれか**にマッチすれば満たされたとみなす
//!   （patch は display と category の2つ、loop tree は相対パス1つ、という違いをここで吸収する）
//! - `-` で始まる term（`-` 1文字は除く）は除外: 残りの正規表現が**どの**フィールドにも
//!   マッチしなければ満たされる。`-` そのものを探すなら `\-` と書く

use regex::{Regex, RegexBuilder};

/// コンパイル済みの 1 term。
#[derive(Clone, Debug)]
pub struct FilterTerm {
    regex: Regex,
    exclude: bool,
}

impl FilterTerm {
    fn compile(term: &str) -> Result<Self, String> {
        let (pattern, exclude) = match term.strip_prefix('-') {
            Some(rest) if !rest.is_empty() => (rest, true),
            _ => (term, false),
        };
        let regex = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { regex, exclude })
    }

    fn matches(&self, fields: &[&str]) -> bool {
        fields.iter().any(|field| self.regex.is_match(field)) != self.exclude
    }

    pub fn is_exclude(&self) -> bool {
        self.exclude
    }
}

/// 空白区切りの各 term を大小無視の正規表現へコンパイルする。
///
/// 空文字列（および空白のみ）は空の `Vec` になり、[`matches_any_field`] は常に true を返す
/// ＝「絞り込みなし・全部通す」。
pub fn compile_condition(condition: &str) -> Result<Vec<FilterTerm>, String> {
    condition
        .split_whitespace()
        .map(FilterTerm::compile)
        .collect()
}

/// 各 term が `fields` に対して満たされるか（term 間 AND）。
pub fn matches_any_field(condition: &[FilterTerm], fields: &[&str]) -> bool {
    condition.iter().all(|term| term.matches(fields))
}

/// 条件が正規表現としてコンパイルできるか。打鍵途中の `kick|` などを弾くのに使う。
pub fn is_valid_condition(condition: &str) -> bool {
    compile_condition(condition).is_ok()
}

#[cfg(test)]
mod tests;
