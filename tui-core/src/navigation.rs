//! j/k などのカーソル移動に関する共通ロジック（画面横断で共有）。
//!
//! - `NavigationCount`: 移動量にプレフィックス数字を掛けるためのカウンタ。
//! - `predicted_navigation_indices*`: 次に押されるキーを先読みして prefetch 対象を決めるヘルパ。

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct NavigationCount {
    value: Option<usize>,
}

impl NavigationCount {
    pub fn push_digit(&mut self, digit: char) -> bool {
        let Some(digit) = digit.to_digit(10).map(|digit| digit as usize) else {
            return false;
        };
        if self.value.is_none() && digit == 0 {
            return false;
        }
        self.value = Some(
            self.value
                .unwrap_or_default()
                .saturating_mul(10)
                .saturating_add(digit),
        );
        true
    }

    pub fn value(&self) -> Option<usize> {
        self.value
    }

    pub fn take_delta(&mut self, unit: isize) -> isize {
        let count = self.value.take().unwrap_or(1);
        let count = isize::try_from(count).unwrap_or(isize::MAX);
        unit.saturating_mul(count)
    }

    pub fn clear(&mut self) {
        self.value = None;
    }
}

/// 現在位置から `j` / `k` / `PageDown` / `PageUp` が次に押されると仮定し、
/// その移動先 index を返す。
///
/// 現在位置そのものや重複した候補は除外する。
pub fn predicted_navigation_indices(
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

pub fn predicted_navigation_indices_in_direction(
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

pub fn predicted_navigation_indices_with_direction_bias(
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
