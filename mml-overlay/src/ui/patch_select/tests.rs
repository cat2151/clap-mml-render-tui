use super::{format_load_time, list_title, scroll_offset};
use crate::{patch_select::PatchSelect, PatchCatalogEntry};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn patch_select(patches: &[&str]) -> PatchSelect<'static> {
    PatchSelect::open(
        patches
            .iter()
            .map(|patch| PatchCatalogEntry::from_display((*patch).to_string()))
            .collect(),
        None,
        Vec::new(),
        Default::default(),
        None,
        Vec::new(),
        Default::default(),
    )
    .expect("patch list is not empty")
}

#[test]
fn load_time_format_uses_readable_truncated_units() {
    assert_eq!(format_load_time(0), "0ms");
    assert_eq!(format_load_time(20), "20ms");
    assert_eq!(format_load_time(99), "99ms");
    assert_eq!(format_load_time(100), "0.1s");
    assert_eq!(format_load_time(999), "0.9s");
    assert_eq!(format_load_time(1_000), "1s");
    assert_eq!(format_load_time(1_999), "1s");
    assert_eq!(format_load_time(9_000), "9s");
}

#[test]
fn scrolling_down_keeps_a_thirty_percent_lower_margin() {
    // 10行中、index 0..=6 までは表示したまま。index 7 へ来たら1行scrollし、
    // 選択行の下に index 8..=10 の3行を残す。
    assert_eq!(scroll_offset(6, 30, 10, 0), 0);
    assert_eq!(scroll_offset(7, 30, 10, 0), 1);
}

#[test]
fn scrolling_up_keeps_a_thirty_percent_upper_margin() {
    // offset 6 の viewport では index 9 が上から30%の境界。index 8 へ来たら
    // offset 5 へ戻し、選択行の上に index 5..=7 の3行を残す。
    assert_eq!(scroll_offset(9, 30, 10, 6), 6);
    assert_eq!(scroll_offset(8, 30, 10, 6), 5);
}

#[test]
fn scroll_margin_clamps_at_ends_and_handles_tiny_viewports() {
    assert_eq!(scroll_offset(0, 30, 10, 20), 0);
    assert_eq!(scroll_offset(29, 30, 10, 0), 20);
    assert_eq!(scroll_offset(2, 30, 3, 0), 0);
    assert_eq!(scroll_offset(0, 30, 0, 8), 0);
}

#[test]
fn list_title_shows_cursor_position_list_length_and_catalog_total() {
    let mut select = patch_select(&["Bass 1.fxp", "Bass 2.fxp", "Lead 1.fxp", "Lead 2.fxp"]);
    select.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert!(list_title(&select).contains("(2/4/4)"));

    select.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    for ch in "lead".chars() {
        select.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    select.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    select.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert!(list_title(&select).contains("(2/2/4)"));
}

#[test]
fn empty_filtered_list_shows_zero_position_and_length() {
    let mut select = patch_select(&["Bass.fxp", "Lead.fxp"]);
    select.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    for ch in "no-match".chars() {
        select.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }

    assert!(list_title(&select).contains("(0/0/2)"));
}
