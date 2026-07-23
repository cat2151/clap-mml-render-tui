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
