use crossterm::event::KeyCode;
use ratatui::{
    backend::TestBackend,
    buffer::Cell,
    style::{Color, Modifier},
    Terminal,
};

use cmrt_tui_core::buffer_test::find_text_ignoring_spaces;
use cmrt_tui_core::theme::{cursor_highlight_bg, MONOKAI_BG, MONOKAI_FG};

use super::*;

#[test]
fn scrolling_keeps_cursor_inside_quarter_margins() {
    let mut scroll = 0;
    assert_eq!(visible_range(7, 100, 12, &mut scroll), 0..12);
    assert_eq!(visible_range(9, 100, 12, &mut scroll), 1..13);
    assert_eq!(visible_range(20, 100, 12, &mut scroll), 12..24);
    assert_eq!(visible_range(13, 100, 12, &mut scroll), 10..22);
}

#[test]
fn scrolling_clamps_at_both_ends_without_blank_rows() {
    let mut scroll = 40;
    assert_eq!(visible_range(0, 50, 12, &mut scroll), 0..12);
    assert_eq!(visible_range(49, 50, 12, &mut scroll), 38..50);
}

#[test]
fn breadcrumb_keeps_the_deepest_segments_when_narrow() {
    let segments = ["loops", "Drums", "Kicks", "Acoustic"].map(str::to_string);
    assert_eq!(
        format_breadcrumb(&segments, 80),
        "loops › Drums › Kicks › Acoustic"
    );
    assert_eq!(format_breadcrumb(&segments, 20), "… › Kicks › Acoustic");
    assert_eq!(format_breadcrumb(&segments, 8), "…coustic");
    assert_eq!(format_breadcrumb(&["ループ".to_string()], 5), "…ープ");
}

#[test]
fn breadcrumb_reserves_space_for_the_direct_category() {
    let segments = ["loops", "Drums", "Acoustic"].map(str::to_string);
    assert_eq!(
        format_breadcrumb_with_category(&segments, Some("drum"), 24),
        ("… › Acoustic".to_string(), " [drum]".to_string())
    );
    assert_eq!(
        format_breadcrumb_with_category(&segments, Some("ドラム"), 7),
        (String::new(), " [ドラ".to_string())
    );
}

/// 描画テスト用の端末。タイトル（`[LOOP TREE] WAV loops  filter: ...`）が
/// 途中で切られない幅にしてある。
const TEST_WIDTH: u16 = 60;
const TEST_HEIGHT: u16 = 16;

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn terminal_for(browser: &mut LoopBrowser) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(TEST_WIDTH, TEST_HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            draw(browser, frame, Rect::new(0, 0, TEST_WIDTH, TEST_HEIGHT));
        })
        .unwrap();
    terminal
}

fn render(browser: &mut LoopBrowser) -> String {
    buffer_to_string(&terminal_for(browser))
}

/// 全角文字は 1 セル目に文字・2 セル目に空白として入るので、行ごとに空白を落として探す
/// （`find_text_ignoring_spaces` と同じ理屈の、行内 `contains` 版）。
fn contains_ignoring_spaces(rendered: &str, text: &str) -> bool {
    let needle = without_spaces(text);
    rendered
        .lines()
        .any(|line| without_spaces(line).contains(&needle))
}

fn without_spaces(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn type_filter(browser: &mut LoopBrowser, text: &str) {
    browser.handle_key(KeyCode::Char('/'));
    for character in text.chars() {
        browser.handle_key(KeyCode::Char(character));
    }
}

#[test]
fn the_filter_input_is_drawn_only_while_typing_and_the_title_keeps_the_hit_count() {
    let mut browser = crate::tests::browser();

    let before = render(&mut browser);
    assert!(
        contains_ignoring_spaces(&before, "[LOOP TREE] WAV loops"),
        "{before}"
    );
    assert!(!contains_ignoring_spaces(&before, "filter:"), "{before}");
    assert!(!contains_ignoring_spaces(&before, "絞り込み"), "{before}");

    type_filter(&mut browser, "kick");
    let typing = render(&mut browser);
    assert!(
        contains_ignoring_spaces(&typing, "絞り込み Enter:確定 Esc:取消"),
        "{typing}"
    );
    assert!(contains_ignoring_spaces(&typing, "kick"), "{typing}");
    assert!(
        contains_ignoring_spaces(&typing, "[LOOP TREE] WAV loops  filter: kick (1 wav)"),
        "{typing}"
    );

    browser.handle_key(KeyCode::Enter);
    let committed = render(&mut browser);
    assert!(
        !contains_ignoring_spaces(&committed, "絞り込み Enter:確定"),
        "{committed}"
    );
    assert!(
        contains_ignoring_spaces(&committed, "[LOOP TREE] WAV loops  filter: kick (1 wav)"),
        "{committed}"
    );
    assert!(
        contains_ignoring_spaces(&committed, "Kick.wav"),
        "{committed}"
    );
    assert!(
        !contains_ignoring_spaces(&committed, "a.wav"),
        "{committed}"
    );
}

#[test]
fn the_terminal_cursor_sits_in_the_filter_input_while_typing() {
    let mut browser = crate::tests::browser();
    type_filter(&mut browser, "ki");

    let mut terminal = terminal_for(&mut browser);
    let cursor = terminal.get_cursor_position().unwrap();
    let (input_x, input_y) =
        find_text_ignoring_spaces(terminal.backend().buffer(), "絞り込みEnter:確定");

    // 入力欄は breadcrumb 行の下の枠付き 3 行。カーソルはその枠の内側、打った 2 文字の右。
    assert_eq!(cursor.y, input_y + 1);
    assert_eq!(cursor.x, input_x - 1 + u16::try_from("ki".len()).unwrap());
}

#[test]
fn a_query_with_no_hits_replaces_the_list_with_a_no_match_line() {
    let mut browser = crate::tests::browser();
    type_filter(&mut browser, "zzz");
    browser.handle_key(KeyCode::Enter);

    let rendered = render(&mut browser);

    assert!(browser.visible.is_empty());
    assert!(
        contains_ignoring_spaces(&rendered, "該当なし"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "[LOOP TREE] WAV loops  filter: zzz (0 wav)"),
        "{rendered}"
    );
    assert!(!contains_ignoring_spaces(&rendered, "/loops"), "{rendered}");
}

#[test]
fn an_invalid_condition_reddens_the_input_frame_and_keeps_the_last_result() {
    let mut browser = crate::tests::browser();
    type_filter(&mut browser, "kick(");

    let terminal = terminal_for(&mut browser);
    let rendered = buffer_to_string(&terminal);

    assert!(
        contains_ignoring_spaces(&rendered, "不正な条件"),
        "{rendered}"
    );
    // 直前の有効な条件（`kick`）の結果が残っている。
    assert!(
        contains_ignoring_spaces(&rendered, "Kick.wav"),
        "{rendered}"
    );
    let (title_x, title_y) =
        find_text_ignoring_spaces(terminal.backend().buffer(), "絞り込みEnter:確定");
    assert_eq!(
        terminal
            .backend()
            .buffer()
            .cell((title_x - 2, title_y))
            .unwrap()
            .style()
            .fg,
        Some(Color::Red)
    );
}

#[test]
fn the_favorites_only_title_survives_the_filter_suffix() {
    let mut browser = crate::tests::browser();
    let favorite = crate::LoopDirId::new(
        std::path::Path::new("/loops"),
        std::path::Path::new("Pack/Drums"),
    );
    browser.metadata.value.toggle_favorite(&favorite);
    browser.rebuild_visible(None);
    browser.handle_key(KeyCode::Char('V'));
    type_filter(&mut browser, "kick");
    browser.handle_key(KeyCode::Enter);

    let rendered = render(&mut browser);

    assert!(
        contains_ignoring_spaces(&rendered, "[LOOP TREE] Favorite dirs  filter: kick (1 wav)"),
        "{rendered}"
    );
}

#[test]
fn a_narrow_pane_drops_the_screen_name_before_the_hit_count() {
    let mut browser = crate::tests::browser();
    type_filter(&mut browser, "kick");
    browser.handle_key(KeyCode::Enter);

    // 実画面のツリーペインは全体幅の 40%。80 桁端末なら 32 桁しかない。
    let mut terminal = Terminal::new(TestBackend::new(32, TEST_HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            draw(&mut browser, frame, Rect::new(0, 0, 32, TEST_HEIGHT));
        })
        .unwrap();
    let rendered = buffer_to_string(&terminal);

    assert!(
        contains_ignoring_spaces(&rendered, "filter: kick (1 wav)"),
        "{rendered}"
    );
    assert!(
        !contains_ignoring_spaces(&rendered, "WAV loops"),
        "{rendered}"
    );
}

/// 現在行（`▶ ` が付く行）の、記号の右隣のセル。
fn cursor_row_cell(terminal: &Terminal<TestBackend>) -> Cell {
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == "\u{25b6}" {
                return buffer.cell((x + 2, y)).unwrap().clone();
            }
        }
    }
    panic!("現在行が見つからない:\n{}", buffer_to_string(terminal));
}

/// issue #334: 絞り込み入力中に見えるカーソルは入力欄の 1 つだけにする。
/// list 側は bg 強調も BOLD も落とす（`▶ ` の行頭記号だけ残る）。
#[test]
fn the_list_row_highlight_goes_away_while_the_filter_input_is_open() {
    let mut browser = crate::tests::browser();

    let normal = cursor_row_cell(&terminal_for(&mut browser));
    assert_eq!(normal.bg, cursor_highlight_bg(MONOKAI_FG));
    assert!(normal.modifier.contains(Modifier::BOLD));

    type_filter(&mut browser, "kick");
    let typing = cursor_row_cell(&terminal_for(&mut browser));
    assert_eq!(typing.bg, MONOKAI_BG);
    assert!(!typing.modifier.contains(Modifier::BOLD));

    // 確定したら強調は戻る（絞り込み自体は続いている）。
    browser.handle_key(KeyCode::Enter);
    let committed = cursor_row_cell(&terminal_for(&mut browser));
    assert_eq!(committed.bg, cursor_highlight_bg(MONOKAI_FG));
    assert!(committed.modifier.contains(Modifier::BOLD));
}

/// 右ペイン（tracks / used wavs / waveform）の強調は `focus == Tracks` が条件で
/// （`loop-browser/src/ui/tracks.rs:45`）、絞り込み入力は tree に focus があるときしか
/// 開けない。＝入力中に右ペインの強調が残る経路が無いことを状態で固定しておく。
#[test]
fn the_filter_input_can_only_be_open_while_the_tree_pane_has_focus() {
    let mut browser = crate::tests::browser();
    type_filter(&mut browser, "kick");

    assert!(browser.filter_input_active());
    assert_eq!(browser.focus, LoopBrowserPane::Tree);

    // Tab も入力欄へ入るので、入力中に focus は動かせない。
    browser.handle_key(KeyCode::Tab);
    assert_eq!(browser.focus, LoopBrowserPane::Tree);
}
