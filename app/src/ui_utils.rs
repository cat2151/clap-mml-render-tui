//! UI ユーティリティ（TUI / DAW 共通）

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
};

const BLOCK_BORDER_SIZE: usize = 2;

/// 指定した割合で中央に配置した矩形を返す。ポップアップ表示に利用する。
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let px = percent_x.min(100);
    let py = percent_y.min(100);
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ])
        .split(v[1])[1]
}

/// 指定したサイズで中央に配置した矩形を返す。
pub(crate) fn centered_rect_with_size(width: u16, height: u16, area: Rect) -> Rect {
    if area.width == 0 || area.height == 0 {
        return area;
    }

    let width = width.max(1).min(area.width);
    let height = height.max(1).min(area.height);
    Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    )
}

/// テキスト行数と最大描画幅に合わせた Block 用の中央配置矩形を返す。
pub(crate) fn centered_text_block_rect(area: Rect, title: &str, lines: &[Line<'_>]) -> Rect {
    let content_width = lines.iter().map(Line::width).max().unwrap_or(0);
    let title_width = Line::from(title).width();
    let raw_width = content_width
        .max(title_width)
        .saturating_add(BLOCK_BORDER_SIZE);
    let raw_height = lines.len().saturating_add(BLOCK_BORDER_SIZE);
    let clamped_width = raw_width.min(area.width as usize);
    let clamped_height = raw_height.min(area.height as usize);

    centered_rect_with_size(clamped_width as u16, clamped_height as u16, area)
}

/// 現在位置から `j` / `k` / `PageDown` / `PageUp` が次に押されると仮定し、
/// その移動先 index を返す。
///
/// 現在位置そのものや重複した候補は除外する。
pub(crate) fn predicted_navigation_indices(
    current: usize,
    item_count: usize,
    page_size: usize,
) -> Vec<usize> {
    if item_count == 0 {
        return Vec::new();
    }

    let mut predicted = Vec::new();
    let mut push_delta = |delta: isize| {
        let next =
            (current as isize + delta).clamp(0, item_count.saturating_sub(1) as isize) as usize;
        if next != current && !predicted.contains(&next) {
            predicted.push(next);
        }
    };

    for delta in [
        1,
        -1,
        page_size.max(1) as isize,
        -(page_size.max(1) as isize),
    ] {
        push_delta(delta);
    }
    predicted
}

fn push_predicted_navigation_delta(
    predicted: &mut Vec<usize>,
    current: usize,
    item_count: usize,
    delta: isize,
) {
    let next = (current as isize + delta).clamp(0, item_count.saturating_sub(1) as isize) as usize;
    if next != current && !predicted.contains(&next) {
        predicted.push(next);
    }
}

pub(crate) fn predicted_navigation_indices_in_direction(
    current: usize,
    item_count: usize,
    delta: isize,
    steps: usize,
) -> Vec<usize> {
    if item_count == 0 || delta == 0 || steps == 0 {
        return Vec::new();
    }

    let mut predicted = Vec::new();
    for step in 1..=steps {
        let step_delta = delta.saturating_mul(step as isize);
        push_predicted_navigation_delta(&mut predicted, current, item_count, step_delta);
    }
    predicted
}

pub(crate) fn predicted_navigation_indices_with_direction_bias(
    current: usize,
    item_count: usize,
    page_size: usize,
    delta: isize,
    leading_direction_steps: usize,
    total_direction_steps: usize,
) -> Vec<usize> {
    if item_count == 0 || delta == 0 || total_direction_steps == 0 {
        return Vec::new();
    }

    let direction = delta.signum();
    let leading_direction_steps = leading_direction_steps.min(total_direction_steps);
    let mut predicted = Vec::new();
    for step in 1..=leading_direction_steps {
        push_predicted_navigation_delta(
            &mut predicted,
            current,
            item_count,
            direction.saturating_mul(step as isize),
        );
    }

    push_predicted_navigation_delta(&mut predicted, current, item_count, -direction);

    let page_delta = direction.saturating_mul(page_size.max(1) as isize);
    push_predicted_navigation_delta(&mut predicted, current, item_count, page_delta);
    push_predicted_navigation_delta(&mut predicted, current, item_count, -page_delta);

    if leading_direction_steps < total_direction_steps {
        for step in (leading_direction_steps + 1)..=total_direction_steps {
            push_predicted_navigation_delta(
                &mut predicted,
                current,
                item_count,
                direction.saturating_mul(step as isize),
            );
        }
    }
    predicted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_with_size_returns_zero_sized_area_unchanged() {
        assert_eq!(
            centered_rect_with_size(10, 10, Rect::new(3, 4, 0, 5)),
            Rect::new(3, 4, 0, 5)
        );
        assert_eq!(
            centered_rect_with_size(10, 10, Rect::new(3, 4, 5, 0)),
            Rect::new(3, 4, 5, 0)
        );
    }

    #[test]
    fn centered_text_block_rect_clamps_large_content_to_area() {
        let area = Rect::new(10, 20, 40, 5);
        let lines = [Line::from("x".repeat(70_000))];

        let rect = centered_text_block_rect(area, " title ", &lines);

        assert_eq!(rect.width, area.width);
        assert_eq!(rect.height, 3);
    }

    #[test]
    fn predicted_navigation_indices_includes_line_and_page_destinations() {
        assert_eq!(predicted_navigation_indices(2, 8, 3), vec![3, 1, 5, 0]);
    }

    #[test]
    fn predicted_navigation_indices_in_direction_returns_two_steps() {
        assert_eq!(
            predicted_navigation_indices_in_direction(2, 10, 3, 2),
            vec![5, 8]
        );
        assert_eq!(
            predicted_navigation_indices_in_direction(2, 10, -1, 2),
            vec![1, 0]
        );
    }

    #[test]
    fn predicted_navigation_indices_with_direction_bias_orders_j_targets() {
        assert_eq!(
            predicted_navigation_indices_with_direction_bias(5, 20, 5, 1, 2, 4),
            vec![6, 7, 4, 10, 0, 8, 9]
        );
    }

    #[test]
    fn predicted_navigation_indices_with_direction_bias_orders_k_targets() {
        assert_eq!(
            predicted_navigation_indices_with_direction_bias(5, 20, 5, -1, 2, 4),
            vec![4, 3, 6, 0, 10, 2, 1]
        );
    }

    #[test]
    fn predicted_navigation_indices_skips_current_and_duplicates() {
        assert_eq!(predicted_navigation_indices(0, 2, 1), vec![1]);
        assert!(predicted_navigation_indices(0, 0, 5).is_empty());
    }
}
