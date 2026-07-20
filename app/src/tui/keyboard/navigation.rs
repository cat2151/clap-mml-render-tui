#[derive(Default)]
pub(in crate::tui) struct NavigationCount {
    value: Option<usize>,
}

impl NavigationCount {
    pub(in crate::tui) fn push_digit(&mut self, digit: char) -> bool {
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

    pub(in crate::tui) fn value(&self) -> Option<usize> {
        self.value
    }

    pub(in crate::tui) fn take_delta(&mut self, unit: isize) -> isize {
        let count = self.value.take().unwrap_or(1);
        let count = isize::try_from(count).unwrap_or(isize::MAX);
        unit.saturating_mul(count)
    }

    pub(in crate::tui) fn clear(&mut self) {
        self.value = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_starts_at_one_and_accepts_zero_afterwards() {
        let mut count = NavigationCount::default();

        assert!(!count.push_digit('0'));
        assert_eq!(count.value(), None);
        assert!(count.push_digit('1'));
        assert!(count.push_digit('0'));
        assert_eq!(count.value(), Some(10));
        assert_eq!(count.take_delta(-1), -10);
        assert_eq!(count.value(), None);
    }

    #[test]
    fn count_and_delta_saturate_instead_of_overflowing() {
        let mut count = NavigationCount::default();
        for _ in 0..100 {
            assert!(count.push_digit('9'));
        }

        assert_eq!(count.value(), Some(usize::MAX));
        assert_eq!(count.take_delta(10), isize::MAX);
    }
}
