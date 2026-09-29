use super::{
    draw_in, format_load_time, list_title, patch_label, sample_size_label, scroll_offset,
    PatchSelectDrawOptions,
};
use crate::{patch_select::PatchSelect, PatchCatalogEntry, PatchSelectRequest};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, text::Span, Terminal};

const SCREEN: Rect = Rect::new(0, 0, 120, 30);

fn patch_select_with_favorites(patches: &[&str], favorites: &[&str]) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: patches
            .iter()
            .map(|patch| PatchCatalogEntry::from_display((*patch).to_string()))
            .collect(),
        favorites: favorites.iter().map(|patch| patch.to_string()).collect(),
        ..Default::default()
    })
    .expect("patch list is not empty")
}

fn render(select: &PatchSelect<'_>, area: Rect, options: &PatchSelectDrawOptions<'_>) -> Buffer {
    let mut terminal =
        Terminal::new(TestBackend::new(SCREEN.width, SCREEN.height)).expect("test terminal");
    terminal
        .draw(|frame| draw_in(select, frame, area, options))
        .expect("draw");
    terminal.backend().buffer().clone()
}

/// 画面の文字列。全角文字の後ろの継続セルは読み飛ばし、見えるとおりの並びにする。
fn lines(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += (Span::raw(symbol).width() as u16).max(1);
            }
            line
        })
        .collect()
}

fn line_with<'a>(lines: &'a [String], text: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(text))
        .unwrap_or_else(|| {
            panic!(
                "{text} is not drawn:
{}",
                lines.join(
                    "
"
                )
            )
        })
}

/// 音色 pane の中の、`patch` の行の左側（音色名より前）だけを返す。
fn patch_row_prefix<'a>(lines: &'a [String], patch: &str) -> &'a str {
    let line = line_with(lines, patch);
    let patch_column = line.find(patch).expect("patch is on the line");
    let pane_start = line[..patch_column]
        .rfind('│')
        .map_or(0, |index| index + '│'.len_utf8());
    &line[pane_start..patch_column]
}

fn patch_select(patches: &[&str]) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: patches
            .iter()
            .map(|patch| PatchCatalogEntry::from_display((*patch).to_string()))
            .collect(),
        ..Default::default()
    })
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

fn weighed(bytes: u64) -> cmrt_tui_core::patch_load::PatchLoadMeasurement {
    cmrt_tui_core::patch_load::PatchLoadMeasurement {
        sfz_sample_bytes: Some(bytes),
        ..Default::default()
    }
}

#[test]
fn sample_size_is_blank_without_a_weight_and_rounds_down_to_megabytes() {
    assert_eq!(sample_size_label(None), "");
    assert_eq!(
        sample_size_label(Some(
            &cmrt_tui_core::patch_load::PatchLoadMeasurement::default()
        )),
        ""
    );
    assert_eq!(sample_size_label(Some(&weighed(999_999))), "<1MB");
    assert_eq!(sample_size_label(Some(&weighed(780_731_839))), "780MB");
}

/// 色が付くのは先読みしない重い音色だけ。軽い sfz は容量だけ見せて色を付けない。
#[test]
fn only_heavy_patches_get_a_colored_size() {
    use cmrt_tui_core::patch_load::HEAVY_OFFLINE_LOAD_BYTES;
    use cmrt_tui_core::theme::MONOKAI_PINK;
    let select = PatchSelect::open(PatchSelectRequest {
        patches: ["Heavy.sfz", "Light.sfz"]
            .iter()
            .map(|patch| PatchCatalogEntry::from_display((*patch).to_string()))
            .collect(),
        load_measurements: [
            ("Heavy.sfz".to_string(), weighed(HEAVY_OFFLINE_LOAD_BYTES)),
            (
                "Light.sfz".to_string(),
                weighed(HEAVY_OFFLINE_LOAD_BYTES - 1_000_000),
            ),
        ]
        .into(),
        ..Default::default()
    })
    .expect("patch list is not empty");

    let buffer = render(&select, SCREEN, &PatchSelectDrawOptions::default());
    let drawn = lines(&buffer);

    let size_color = |text: &str| {
        let y = drawn.iter().position(|line| line.contains(text)).unwrap() as u16;
        let x = (0..SCREEN.width)
            .find(|&x| buffer[(x, y)].symbol() == "M" && buffer[(x + 1, y)].symbol() == "B")
            .expect("size is drawn");
        buffer[(x, y)].fg
    };
    line_with(&drawn, "64MB");
    line_with(&drawn, "63MB");
    assert_eq!(size_color("Heavy.sfz"), MONOKAI_PINK);
    assert_ne!(size_color("Light.sfz"), MONOKAI_PINK);
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

#[test]
fn merged_patch_label_shows_the_merged_count() {
    let merged = PatchCatalogEntry::from_display("A.syx/00 BELL".to_string())
        .with_merged(10, vec!["BELL".to_string()]);
    let single = PatchCatalogEntry::from_display("A.syx/01 PAD".to_string());

    assert_eq!(patch_label(&merged), "A.syx/00 BELL ×10");
    assert_eq!(patch_label(&single), "A.syx/01 PAD");
}

#[test]
fn favorite_column_marks_only_favorite_patches() {
    let select = patch_select_with_favorites(&["Bass 1.fxp", "Lead 1.fxp"], &["Lead 1.fxp"]);

    let lines = lines(&render(&select, SCREEN, &PatchSelectDrawOptions::default()));

    assert!(patch_row_prefix(&lines, "Lead 1.fxp").contains('★'));
    assert!(!patch_row_prefix(&lines, "Bass 1.fxp").contains('★'));
}

#[test]
fn preset_pane_lists_the_favorite_preset() {
    let select = patch_select_with_favorites(&["Bass 1.fxp"], &[]);

    let lines = lines(&render(&select, SCREEN, &PatchSelectDrawOptions::default()));

    line_with(&lines, "★ Favorite");
}

#[test]
fn patch_marker_is_drawn_at_the_row_start_only_when_given() {
    let select = patch_select_with_favorites(&["Bass 1.fxp", "Lead 1.fxp"], &[]);
    let marker = |patch: &str| Span::raw(if patch == "Lead 1.fxp" { "♪ " } else { ". " });
    let with_marker = PatchSelectDrawOptions {
        patch_marker: Some(&marker),
        ..Default::default()
    };

    let marked = lines(&render(&select, SCREEN, &with_marker));
    let plain = lines(&render(&select, SCREEN, &PatchSelectDrawOptions::default()));

    assert!(patch_row_prefix(&marked, "Lead 1.fxp").contains('♪'));
    assert!(patch_row_prefix(&marked, "Bass 1.fxp").contains('.'));
    assert!(!patch_row_prefix(&plain, "Lead 1.fxp").contains('♪'));
    assert!(!patch_row_prefix(&plain, "Bass 1.fxp").contains('.'));
}

#[test]
fn patch_marker_is_asked_only_for_the_rows_on_screen() {
    let names: Vec<String> = (0..500).map(|i| format!("Lead {i:03}.fxp")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let select = patch_select_with_favorites(&names, &[]);
    let asked = std::cell::RefCell::new(Vec::new());
    let marker = |patch: &str| {
        asked.borrow_mut().push(patch.to_string());
        Span::raw("♪ ")
    };
    let options = PatchSelectDrawOptions {
        patch_marker: Some(&marker),
        ..Default::default()
    };

    let drawn = lines(&render(&select, SCREEN, &options));

    let asked = asked.into_inner();
    assert!(asked.len() < usize::from(SCREEN.height), "{}", asked.len());
    for patch in &asked {
        assert!(patch_row_prefix(&drawn, patch).contains('♪'), "{patch}");
    }
}

#[test]
fn play_settings_hint_can_be_hidden_from_the_regex_title() {
    let select = patch_select_with_favorites(&["Bass 1.fxp"], &[]);
    let hidden = PatchSelectDrawOptions {
        show_play_settings_hint: false,
        ..Default::default()
    };

    let shown = lines(&render(&select, SCREEN, &PatchSelectDrawOptions::default())).join(
        "
",
    );
    let hidden = lines(&render(&select, SCREEN, &hidden)).join(
        "
",
    );

    assert!(shown.contains("S:演奏設定"));
    assert!(hidden.contains("Space:試聴"));
    assert!(!hidden.contains("S:演奏設定"));
}

#[test]
fn draw_in_stays_inside_the_given_area() {
    let select = patch_select_with_favorites(&["Bass 1.fxp", "Lead 1.fxp"], &["Lead 1.fxp"]);
    let area = Rect::new(5, 3, 100, 20);

    let buffer = render(&select, area, &PatchSelectDrawOptions::default());

    let blank = Buffer::empty(SCREEN);
    for y in 0..SCREEN.height {
        for x in 0..SCREEN.width {
            if !area.contains((x, y).into()) {
                assert_eq!(buffer[(x, y)], blank[(x, y)], "drawn outside at ({x}, {y})");
            }
        }
    }
    assert!(lines(&buffer)
        .join(
            "
"
        )
        .contains("Lead 1.fxp"));
}

#[test]
fn plugin_menu_lists_keys_and_marks_the_muted_plugin() {
    let mut select = PatchSelect::open(PatchSelectRequest {
        patches: vec![
            PatchCatalogEntry::new(
                "Bell.syx/00".into(),
                "bell.syx/00".into(),
                "Dexed".into(),
                None,
            ),
            PatchCatalogEntry::new(
                "Harp.floe-preset".into(),
                "harp.floe-preset".into(),
                "Floe".into(),
                None,
            ),
        ],
        ..Default::default()
    })
    .expect("patch list is not empty");
    let press = |ch| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
    select.handle_key(press('m'));
    select.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    select.handle_key(press('m'));

    let lines = lines(&render(&select, SCREEN, &PatchSelectDrawOptions::default()));

    assert!(line_with(&lines, " d  dexed").contains("mute"));
    assert!(!line_with(&lines, " f  floe").contains("mute"));
    line_with(&lines, "a-z:solo  A-Z:mute");
}
