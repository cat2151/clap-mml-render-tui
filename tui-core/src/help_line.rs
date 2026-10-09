//! help overlay の 1 行を monokai 色に塗り分ける。
//! 「── 見出し ──」は pink、行頭 `key_width` セルのキー欄は yellow、残りの説明は前景色。

use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::{
    status::base_style,
    theme::{MONOKAI_PINK, MONOKAI_YELLOW},
};

/// `row` を行頭から表示幅 `key_width` セルでキー欄と説明に分けて塗る。
/// キー欄が空白だけの行（前行の説明の続き）は説明だけの行になる。
pub fn help_line(row: &'static str, key_width: usize) -> Line<'static> {
    if row.trim_start().starts_with("──") {
        return Line::from(Span::styled(
            row,
            base_style().fg(MONOKAI_PINK).add_modifier(Modifier::BOLD),
        ));
    }
    let split = row
        .char_indices()
        .map(|(index, _)| index)
        .find(|&index| Span::raw(&row[..index]).width() >= key_width)
        .unwrap_or(row.len());
    let (key, description) = row.split_at(split);
    Line::from(vec![
        Span::styled(key, base_style().fg(MONOKAI_YELLOW)),
        Span::styled(description, base_style()),
    ])
}

#[cfg(test)]
mod tests;
