//! UI ユーティリティ（TUI / DAW 共通）

// 中央配置矩形ヘルパ（centered_rect / centered_rect_with_size / centered_text_block_rect）と
// 音出し確認ガイドのオーバーレイは、画面横断で共有するため `cmrt-tui-core` へ切り出した。
// 従来の `crate::ui_utils::*` パスは再エクスポートで維持する。
pub(crate) use cmrt_tui_core::ui::*;

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
    use ratatui::{layout::Rect, text::Line};

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
