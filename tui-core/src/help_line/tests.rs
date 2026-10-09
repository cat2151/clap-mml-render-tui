use super::*;
use crate::theme::MONOKAI_FG;

fn parts(line: &Line<'_>) -> Vec<(String, Option<ratatui::style::Color>)> {
    line.spans
        .iter()
        .map(|span| (span.content.to_string(), span.style.fg))
        .collect()
}

#[test]
fn heading_is_pink_as_a_whole() {
    let line = help_line(" ── 編集 ──", 9);
    assert_eq!(parts(&line), [(" ── 編集 ──".into(), Some(MONOKAI_PINK))]);
}

#[test]
fn key_column_is_split_by_display_width_not_by_spaces() {
    assert_eq!(
        parts(&help_line(" - / +   長さ", 9)),
        [
            (" - / +   ".into(), Some(MONOKAI_YELLOW)),
            ("長さ".into(), Some(MONOKAI_FG)),
        ]
    );
    // 全角は 1 文字 2 セルとして数える。
    assert_eq!(
        parts(&help_line(" 数字    次のキー", 9)),
        [
            (" 数字    ".into(), Some(MONOKAI_YELLOW)),
            ("次のキー".into(), Some(MONOKAI_FG)),
        ]
    );
}

#[test]
fn row_shorter_than_key_column_is_all_key() {
    assert_eq!(
        parts(&help_line(" q", 9)),
        [
            (" q".into(), Some(MONOKAI_YELLOW)),
            (String::new(), Some(MONOKAI_FG))
        ]
    );
}
