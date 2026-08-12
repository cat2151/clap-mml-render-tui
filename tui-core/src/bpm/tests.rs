use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn typed(input: &str) -> BpmInput {
    let mut state = BpmInput::default();
    for character in input.chars() {
        assert_eq!(
            state.handle_key(press(KeyCode::Char(character))),
            BpmInputAction::Continue
        );
    }
    state
}

fn enter(input: &str) -> (BpmInput, BpmInputAction) {
    let mut state = typed(input);
    let action = state.handle_key(press(KeyCode::Enter));
    (state, action)
}

fn applied_range(action: BpmInputAction) -> BpmRange {
    match action {
        BpmInputAction::ApplyAuto(Some(range)) => range,
        other => panic!("範囲が確定していない: {other:?}"),
    }
}

#[test]
fn accepts_integer_and_unbounded_decimal_precision() {
    assert_eq!(
        enter("128").1,
        BpmInputAction::Apply(BpmMode::Manual(128.0))
    );
    assert_eq!(
        enter("128.123456789").1,
        BpmInputAction::Apply(BpmMode::Manual(128.123456789))
    );
}

#[test]
fn accepts_the_inclusive_limits() {
    assert_eq!(enter("20").1, BpmInputAction::Apply(BpmMode::Manual(20.0)));
    assert_eq!(
        enter("300").1,
        BpmInputAction::Apply(BpmMode::Manual(300.0))
    );
}

#[test]
fn rejects_empty_invalid_and_out_of_range_values_without_closing() {
    for value in ["", ".", "19.99", "300.01"] {
        let (state, action) = enter(value);
        assert_eq!(action, BpmInputAction::Continue, "value={value}");
        assert!(state.error().is_some(), "value={value}");
    }
}

#[test]
fn ignores_a_second_decimal_point_and_supports_backspace() {
    let mut state = typed("12.3.4");
    assert_eq!(state.buffer(), "12.34");
    state.handle_key(press(KeyCode::Backspace));
    assert_eq!(state.buffer(), "12.3");
}

#[test]
fn a_selects_auto_and_escape_cancels() {
    let mut state = BpmInput::default();
    assert_eq!(
        state.handle_key(press(KeyCode::Char('a'))),
        BpmInputAction::ApplyAuto(None)
    );
    assert_eq!(
        state.handle_key(press(KeyCode::Esc)),
        BpmInputAction::Cancel
    );
}

#[test]
fn a_hyphen_pair_sets_the_automatic_bpm_range() {
    let range = applied_range(enter("80-160").1);
    assert_eq!(range.minimum(), 80.0);
    assert_eq!(range.maximum(), 160.0);
    assert!(!range.is_fixed());

    // 両端が同じでも範囲として通る（＝抽選しても動かない固定値）。
    let fixed = applied_range(enter("130-130").1);
    assert!(fixed.is_fixed());
    assert_eq!(fixed.sample(), 130.0);
}

#[test]
fn a_reversed_or_out_of_range_pair_reports_an_error_without_closing() {
    for value in ["160-80", "10-160", "80-400", "80-", "-", "80-1-6"] {
        let (state, action) = enter(value);
        assert_eq!(action, BpmInputAction::Continue, "value={value}");
        assert!(state.error().is_some(), "value={value}");
    }
}

#[test]
fn the_range_separator_and_decimal_point_never_mix() {
    // 範囲は整数どうしなので、`-` のあとに `.` は入らない。
    assert_eq!(typed("80.5-160").buffer(), "80.5160");
    // 先頭の `-` と2個目の `-` も落とす。
    assert_eq!(typed("-80--160").buffer(), "80-160");
}

#[test]
fn sampling_stays_inside_the_range() {
    let range = BpmRange::new(80.0, 160.0).expect("有効な範囲");
    for _ in 0..64 {
        let bpm = range.sample();
        assert!((80.0..=160.0).contains(&bpm), "bpm={bpm}");
        assert_eq!(bpm.fract(), 0.0, "整数BPMで引く: bpm={bpm}");
    }
    assert_eq!(BpmRange::fixed(130.0).sample(), 130.0);
}

#[test]
fn a_range_only_accepts_integer_ends_inside_the_bpm_limits() {
    assert!(BpmRange::new(80.5, 160.0).is_none());
    assert!(BpmRange::new(19.0, 160.0).is_none());
    assert!(BpmRange::new(80.0, 301.0).is_none());
    assert!(BpmRange::new(160.0, 80.0).is_none());
    assert!(BpmRange::new(80.0, 80.0).is_some());
}

#[test]
fn saved_manual_values_are_validated() {
    assert_eq!(
        BpmMode::from_saved(Some(128.5), 130.0),
        BpmMode::Manual(128.5)
    );
    assert_eq!(BpmMode::from_saved(Some(0.0), 130.0), BpmMode::Auto(130.0));
    assert_eq!(
        BpmMode::from_saved(Some(f64::NAN), 120.0),
        BpmMode::Auto(120.0)
    );
    assert_eq!(BpmMode::from_saved(None, 120.0), BpmMode::Auto(120.0));
}

#[test]
fn auto_modes_with_different_draws_are_not_equal() {
    // A キーでの引き直しを「変化なし」で弾かないための性質。
    assert_ne!(BpmMode::Auto(130.0), BpmMode::Auto(131.0));
    assert_eq!(BpmMode::Auto(130.0).bpm(), 130.0);
    assert_eq!(BpmMode::Auto(130.0).auto_target(), Some(130.0));
    assert_eq!(BpmMode::Manual(90.0).auto_target(), None);
}

#[test]
fn range_labels_collapse_when_there_is_no_width() {
    assert_eq!(BpmRange::fixed(130.0).label(), "130");
    assert_eq!(
        BpmRange::new(80.0, 160.0).expect("有効な範囲").label(),
        "80〜160"
    );
}
