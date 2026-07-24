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
