//! 「1周ごとの random」設定 overlay の描画と当たり判定。
//!
//! 設定そのものは [`crate::cycle_random`]。ここは画面中央へ出す枠と、click が
//! どの項目を踏んだかだけを持つ。
//!
//! 描画と当たり判定が同じ矩形を見るよう、矩形は必ず [`overlay_rect`] から取ること。
//! 項目の行は上から [`CycleRandomItem::ALL`] の順で、枠の内側の先頭から並ぶ。

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_YELLOW},
};

use crate::{CycleRandom, CycleRandomItem, GridSequencerScreen};

const TITLE: &str = " 1周ごとの random ";

/// 枠線が食う幅・高さ。
const BORDER_SIZE: u16 = 2;

/// 項目の並びのあとに置く操作説明。空行1つを挟む。
const FOOTER: [&str; 3] = [
    "",
    " 1-7 / click:切替   Space:切替   ↑↓:移動",
    " A:全ON   N:全OFF   Esc / q / a:閉じる",
];

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let cursor = screen.cycle_random_cursor();
    let lines = lines(screen.cycle_random(), cursor);
    let area = overlay_rect(f.area());
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_YELLOW)),
        ),
        area,
    );
}

/// overlay の矩形。中身の幅・高さは設定値によらず一定なので、端末の矩形だけで決まる。
pub(crate) fn overlay_rect(area: Rect) -> Rect {
    // カーソルとチェックの有無で幅は変わらない。代表として全 ON・先頭カーソルで測る。
    let lines = lines(CycleRandom::ALL, Some(0));
    let content_width = lines.iter().map(Line::width).max().unwrap_or(0) as u16;
    let width = content_width
        .max(Line::from(TITLE).width() as u16)
        .saturating_add(BORDER_SIZE);
    let height = (lines.len() as u16).saturating_add(BORDER_SIZE);
    cmrt_tui_core::ui::centered_rect_with_size(width, height, area)
}

/// click が踏んだもの。
///
/// - `Some(Some(index))` … 項目の行
/// - `Some(None)` … 枠の中だが項目ではない行
/// - `None` … 枠の外
pub(crate) fn hit_test(area: Rect, column: u16, row: u16) -> Option<Option<usize>> {
    let overlay = overlay_rect(area);
    let inside = column >= overlay.x
        && column < overlay.x.saturating_add(overlay.width)
        && row >= overlay.y
        && row < overlay.y.saturating_add(overlay.height);
    if !inside {
        return None;
    }
    // 枠線の1行ぶん内側から項目が並ぶ。
    let index = usize::from(row.saturating_sub(overlay.y + 1));
    Some((row > overlay.y && index < CycleRandomItem::ALL.len()).then_some(index))
}

fn lines(random: CycleRandom, cursor: Option<usize>) -> Vec<Line<'static>> {
    let mut lines = CycleRandomItem::ALL
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let selected = cursor == Some(index);
            let marker = if selected { ">" } else { " " };
            let check = if random.get(*item) { "x" } else { " " };
            Line::styled(
                format!(
                    "{marker} [{check}] {} {:<6} {}",
                    index + 1,
                    item.label(),
                    item.description(),
                ),
                entry_style(random.get(*item), selected),
            )
        })
        .collect::<Vec<_>>();
    lines.extend(
        FOOTER
            .iter()
            .map(|text| Line::styled((*text).to_string(), base_style().fg(MONOKAI_GRAY))),
    );
    lines
}

/// ON / OFF が一目で分かるよう、OFF は灰色へ落とす。カーソル行だけ強調する。
fn entry_style(on: bool, selected: bool) -> Style {
    let style = if on {
        base_style().fg(MONOKAI_GREEN)
    } else {
        base_style().fg(MONOKAI_GRAY)
    };
    if selected {
        style.add_modifier(ratatui::style::Modifier::REVERSED)
    } else {
        style
    }
}

#[cfg(test)]
mod tests;
