//! j/k などの移動量にプレフィックス数字を掛けるためのカウンタ（画面横断で共有）。

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
