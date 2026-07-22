use super::super::MEASURES;
use super::{
    effective_measure_count, format_measure_list, loop_measure_summary_label, play_start_log_lines,
};
use crate::daw::AbRepeatState;

#[test]
fn effective_measure_count_all_empty_returns_none() {
    let mmls = vec!["".to_string(); MEASURES];
    assert_eq!(effective_measure_count(&mmls), None);
}

#[test]
fn effective_measure_count_skips_trailing_empty_measures() {
    let mut mmls = vec!["".to_string(); MEASURES];
    mmls[0] = "cccccccc".to_string();
    mmls[1] = "ffffffff".to_string();
    assert_eq!(effective_measure_count(&mmls), Some(2));
}

#[test]
fn effective_measure_count_includes_internal_empty_measures() {
    let mut mmls = vec!["".to_string(); MEASURES];
    mmls[0] = "cde".to_string();
    mmls[2] = "fga".to_string();
    assert_eq!(effective_measure_count(&mmls), Some(3));
}

#[test]
fn effective_measure_count_single_non_empty_measure() {
    let mut mmls = vec!["".to_string(); MEASURES];
    mmls[0] = "c".to_string();
    assert_eq!(effective_measure_count(&mmls), Some(1));
}

#[test]
fn effective_measure_count_all_measures_non_empty() {
    let mmls: Vec<String> = (0..MEASURES).map(|i| format!("c{}", i)).collect();
    assert_eq!(effective_measure_count(&mmls), Some(MEASURES));
}

#[test]
fn effective_measure_count_whitespace_only_treated_as_empty() {
    let mut mmls = vec!["".to_string(); MEASURES];
    mmls[0] = "cde".to_string();
    mmls[1] = "   ".to_string();
    assert_eq!(effective_measure_count(&mmls), Some(1));
}

#[test]
fn format_measure_list_merges_consecutive_ranges() {
    assert_eq!(
        format_measure_list(&[1, 2, 3, 5, 7, 8]),
        Some("meas 1～3, meas 5, meas 7～8".to_string())
    );
}

#[test]
fn loop_measure_summary_label_lists_loop_and_empty_ranges() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();
    mmls[2] = "g".to_string();

    assert_eq!(
        loop_measure_summary_label(&mmls, AbRepeatState::Off),
        Some("loop meas : meas 1～3, empty meas : meas 2, meas 4～8".to_string())
    );
}

#[test]
fn loop_measure_summary_label_uses_ab_repeat_range_when_active() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();
    mmls[2] = "g".to_string();

    assert_eq!(
        loop_measure_summary_label(
            &mmls,
            AbRepeatState::FixEnd {
                start_measure_index: 1,
                end_measure_index: 2,
            }
        ),
        Some("loop meas : meas 2～3, empty meas : meas 2, meas 4～8".to_string())
    );
}

#[test]
fn play_start_log_lines_describe_active_and_empty_measures() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();

    assert_eq!(
        play_start_log_lines(&mmls, AbRepeatState::Off),
        vec![
            "meas1 : 内容があります".to_string(),
            "meas2 : empty".to_string(),
            "meas3 : empty".to_string(),
            "meas4 : empty".to_string(),
            "meas5 : empty".to_string(),
            "meas6 : empty".to_string(),
            "meas7 : empty".to_string(),
            "meas8 : empty".to_string(),
            "有効meas : meas 1".to_string(),
            "empty meas : meas 2～8".to_string(),
            "loop start meas : meas1".to_string(),
            "loop end meas : meas1".to_string(),
        ]
    );
}

#[test]
fn play_start_log_lines_use_ab_repeat_start_and_end_when_active() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();
    mmls[1] = "d".to_string();
    mmls[2] = "e".to_string();

    let loop_lines = play_start_log_lines(
        &mmls,
        AbRepeatState::FixEnd {
            start_measure_index: 1,
            end_measure_index: 2,
        },
    );

    assert_eq!(
        &loop_lines[loop_lines.len() - 2..],
        &[
            "loop start meas : meas2".to_string(),
            "loop end meas : meas3".to_string(),
        ]
    );
}
