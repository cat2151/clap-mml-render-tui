use std::time::{Duration, Instant};

use ratatui::{backend::TestBackend, style::Color, Terminal};

use cmrt_tui_core::{
    buffer_test::find_text_ignoring_spaces,
    theme::{cursor_highlight_bg, MONOKAI_DARK_GRAY, MONOKAI_FG},
};

use super::*;
use crate::{GridPatchStatus, GridProgress, StepDuration, GRID_ROWS, STEP_INTERVAL};

/// 情報欄(40桁) + 枠線(1桁) のぶんだけ右にある、grid の先頭セルの列。
const FIRST_CELL_X: usize = 41;
/// 枠線(1行) + ヘッダ(1行) のぶんだけ下にある、grid の1行目の行。
const FIRST_ROW_Y: usize = 2;
/// 1セルは記号+空白の2桁。
const CELLS_WIDTH: usize = GRID_STEPS * 2;

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

/// 枠線が multi-byte なので、バイトではなく文字数で切り出す。
fn slice_chars(line: &str, start: usize, len: usize) -> String {
    line.chars().skip(start).take(len).collect()
}

fn terminal_for(screen: &GridSequencerScreen) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    let connection = screen.connection_status();
    terminal.draw(|f| draw(screen, &connection, f)).unwrap();
    terminal
}

fn render(screen: &GridSequencerScreen) -> String {
    buffer_to_string(&terminal_for(screen))
}

fn render_with_connection(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
) -> String {
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    terminal.draw(|f| draw(screen, connection, f)).unwrap();
    buffer_to_string(&terminal)
}

fn terminal_with_connection(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    terminal.draw(|f| draw(screen, connection, f)).unwrap();
    terminal
}

/// 指定した grid 行の、左情報欄（行番号の桁）の前景色。
/// 中央 overlay は画面中央に出るので、左端のこの列とは重ならない。
fn row_label_fg(terminal: &Terminal<TestBackend>, row: usize) -> Color {
    terminal
        .backend()
        .buffer()
        .cell((3u16, (FIRST_ROW_Y + row) as u16))
        .unwrap()
        .fg
}

/// 進捗 overlay が出ているか。バーの記号は overlay にしか現れない。
fn has_progress_overlay(rendered: &str) -> bool {
    rendered.contains('█') || rendered.contains('░')
}

/// 1行目だけを鳴らす、決め打ちの grid。
fn screen_with_first_row(note: u8, duration: StepDuration, cells: &[usize]) -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::new(None);
    let row = &mut screen.state.rows_mut()[0];
    row.patch = Some("Keys/Piano.fxp".to_string());
    row.note = note;
    row.duration = duration;
    for step in cells {
        row.cells[*step] = true;
    }
    screen
}

#[test]
fn every_row_is_drawn_with_its_left_hand_columns() {
    let mut screen = screen_with_first_row(60, StepDuration::Quarter, &[0]);
    let last = &mut screen.state.rows_mut()[GRID_ROWS - 1];
    last.patch = Some("Leads/Saw.fxp".to_string());
    last.note = 84;

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();

    assert!(rendered.contains("PATCH"), "{rendered}");
    assert!(rendered.contains("NOTE"), "{rendered}");
    assert!(rendered.contains("DUR"), "{rendered}");
    assert!(rendered.contains("Keys/Piano.fxp"), "{rendered}");
    assert!(rendered.contains("Leads/Saw.fxp"), "{rendered}");
    assert!(rendered.contains("1/4"), "{rendered}");
    assert!(rendered.contains("1/16"), "{rendered}");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y], 1, 4), "  1 ");
    assert_eq!(
        slice_chars(lines[FIRST_ROW_Y + GRID_ROWS - 1], 1, 4),
        " 16 "
    );
}

#[test]
fn note_on_cells_are_marked_and_rests_are_dots() {
    let on_steps = [0usize, 4, 8];
    let screen = screen_with_first_row(60, StepDuration::Sixteenth, &on_steps);

    let rendered = render(&screen);
    let first_row = rendered.lines().nth(FIRST_ROW_Y).unwrap();

    let expected = (0..GRID_STEPS)
        .map(|step| if on_steps.contains(&step) { "# " } else { ". " })
        .collect::<String>();
    assert_eq!(
        slice_chars(first_row, FIRST_CELL_X, CELLS_WIDTH),
        expected,
        "{rendered}"
    );
}

#[test]
fn the_playhead_column_follows_the_step_progression() {
    let now = Instant::now();
    let mut screen = screen_with_first_row(60, StepDuration::Sixteenth, &[]);
    screen.state.start(now);
    screen.state.poll_steps(now, Duration::ZERO);
    screen.state.poll_steps(now + STEP_INTERVAL, Duration::ZERO);
    assert_eq!(screen.state.step_index(), 1);

    let terminal = terminal_for(&screen);
    let buffer = terminal.backend().buffer();

    let first = buffer
        .cell((FIRST_CELL_X as u16, FIRST_ROW_Y as u16))
        .unwrap();
    let second = buffer
        .cell((FIRST_CELL_X as u16 + 2, FIRST_ROW_Y as u16))
        .unwrap();
    assert_ne!(first.bg, cursor_highlight_bg(first.fg));
    assert_eq!(second.bg, cursor_highlight_bg(second.fg));
}

#[test]
fn the_step_ruler_marks_every_fourth_step() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render(&screen);
    let header = rendered.lines().nth(FIRST_ROW_Y - 1).unwrap();

    assert_eq!(
        slice_chars(header, FIRST_CELL_X, CELLS_WIDTH).trim_end(),
        "1       5       9       13"
    );
}

#[test]
fn the_status_line_shows_instances_and_limiter_reduction() {
    let mut screen = screen_with_first_row(60, StepDuration::Sixteenth, &[]);
    screen.patch_status = GridPatchStatus::Ready(42);

    let rendered = render(&screen);

    assert!(rendered.contains("SHM idle"), "{rendered}");
    assert!(rendered.contains("130bpm"), "{rendered}");
    assert!(rendered.contains("16tr"), "{rendered}");
    assert!(rendered.contains("step 1/16"), "{rendered}");
    assert!(rendered.contains("GR0.0"), "{rendered}");
    assert!(rendered.contains("p:42"), "{rendered}");
}

#[test]
fn compact_grid_draws_only_the_selected_tracks() {
    let screen = GridSequencerScreen::with_track_count(None, 2);

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();

    assert_eq!(slice_chars(lines[FIRST_ROW_Y], 1, 4), "  1 ");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y + 1], 1, 4), "  2 ");
    assert_ne!(slice_chars(lines[FIRST_ROW_Y + 2], 1, 4), "  3 ");
    assert!(rendered.contains("2tr"), "{rendered}");
}

#[test]
fn the_status_line_shows_adaptive_buffer_and_current_level_underruns() {
    let screen = screen_with_first_row(60, StepDuration::Sixteenth, &[]);
    let connection = GridConnectionStatus {
        buffer_multiplier: 8,
        underrun_frames: 1_536,
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("buffer x8 auto"), "{rendered}");
    assert!(rendered.contains("underrun 1536 frames"), "{rendered}");
}

#[test]
fn the_status_line_shows_instance_startup_progress() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Connecting,
        server_startup: Some(GridProgress {
            completed: 6,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("SHM starting server 6/16"), "{rendered}");
}

#[test]
fn the_status_line_shows_patch_setting_progress() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::PatchSetting,
        patch_setting: Some(GridProgress {
            completed: 11,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("SHM patches 11/16"), "{rendered}");
}

#[test]
fn the_progress_overlay_shows_both_stages_while_patches_load() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::PatchSetting,
        patch_setting: Some(GridProgress {
            completed: 5,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let terminal = terminal_with_connection(&screen, &connection);
    let buffer = terminal.backend().buffer();

    // 全角文字はセル2つぶんを占めるため、空白を無視して探す。
    find_text_ignoring_spaces(buffer, "準備中");
    find_text_ignoring_spaces(buffer, "サーバー起動");
    find_text_ignoring_spaces(buffer, "音色ロード");
    // 段階1は完了、段階2が 5/16 まで進んでいる。
    find_text_ignoring_spaces(buffer, "16/16");
    find_text_ignoring_spaces(buffer, "5/16");
}

#[test]
fn rows_lose_their_grey_out_one_by_one_as_patches_load() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::PatchSetting,
        patch_setting: Some(GridProgress {
            completed: 5,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let terminal = terminal_with_connection(&screen, &connection);

    for row in 0..5 {
        assert_eq!(row_label_fg(&terminal, row), MONOKAI_FG, "row {row}");
    }
    for row in 5..GRID_ROWS {
        assert_eq!(row_label_fg(&terminal, row), MONOKAI_GRAY, "row {row}");
    }
}

#[test]
fn rows_stay_dark_until_the_server_has_built_their_instance() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Connecting,
        server_startup: Some(GridProgress {
            completed: 3,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let terminal = terminal_with_connection(&screen, &connection);

    for row in 0..3 {
        assert_eq!(row_label_fg(&terminal, row), MONOKAI_GRAY, "row {row}");
    }
    for row in 3..GRID_ROWS {
        assert_eq!(row_label_fg(&terminal, row), MONOKAI_DARK_GRAY, "row {row}");
    }
}

#[test]
fn the_error_overlay_shows_the_reason_and_greys_out_every_row() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Error("grid row 3 patch prepare failed".to_string()),
        ..GridConnectionStatus::default()
    };

    let terminal = terminal_with_connection(&screen, &connection);
    let buffer = terminal.backend().buffer();

    find_text_ignoring_spaces(buffer, "準備エラー");
    find_text_ignoring_spaces(buffer, "gridrow3patchpreparefailed");
    for row in 0..GRID_ROWS {
        assert_eq!(row_label_fg(&terminal, row), MONOKAI_DARK_GRAY, "row {row}");
    }
}

#[test]
fn the_overlay_disappears_and_rows_regain_their_colour_once_ready() {
    let screen = screen_with_first_row(60, StepDuration::Sixteenth, &[0]);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);
    let terminal = terminal_with_connection(&screen, &connection);

    assert!(!has_progress_overlay(&rendered), "{rendered}");
    for row in 0..GRID_ROWS {
        assert_eq!(row_label_fg(&terminal, row), MONOKAI_FG, "row {row}");
    }
}

/// MIDI sender を持たないテストモード（`Idle`）は「準備中」ではないので、
/// overlay を出さず grid も通常色で描く。
#[test]
fn the_idle_test_mode_draws_the_grid_without_any_overlay() {
    let screen = screen_with_first_row(60, StepDuration::Sixteenth, &[0]);

    let rendered = render(&screen);
    let terminal = terminal_for(&screen);

    assert!(!has_progress_overlay(&rendered), "{rendered}");
    assert_eq!(row_label_fg(&terminal, 0), MONOKAI_FG);
}

#[test]
fn the_keybind_line_is_always_visible() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render(&screen);

    assert!(rendered.contains("r:randomize"), "{rendered}");
    assert!(rendered.contains("R:randomize-notes"), "{rendered}");
    assert!(rendered.contains("t:tracks"), "{rendered}");
    assert!(rendered.contains("Ctrl+G:screen"), "{rendered}");
    assert!(rendered.contains("q:quit"), "{rendered}");
}

#[test]
fn the_help_overlay_lists_the_keybinds() {
    let mut screen = GridSequencerScreen::new(None);
    screen.help_open = true;

    let terminal = terminal_for(&screen);
    let buffer = terminal.backend().buffer();

    // 全角文字はセル2つぶんを占めるため、空白を無視して探す。
    find_text_ignoring_spaces(buffer, "ヘルプ(Keybinds)");
    find_text_ignoring_spaces(buffer, "画面切替メニュー");
    find_text_ignoring_spaces(buffer, "Ctrl+G");
}

/// メモリ行は overlay の先頭に出す。ヘルプが端末より長いと下が切り落とされるため。
#[test]
fn the_help_overlay_shows_the_memory_usage_at_the_top() {
    let mut screen = GridSequencerScreen::new(None);
    screen.help_open = true;

    let terminal = terminal_for(&screen);
    let buffer = terminal.backend().buffer();
    let (_, top, _, _) = cmrt_tui_core::buffer_test::help_overlay_bounds(buffer);

    let memory_line = buffer_to_string(&terminal)
        .lines()
        .nth(usize::from(top) + 1)
        .unwrap()
        .replace(' ', "");

    assert!(memory_line.contains("実メモリ合計"), "{memory_line}");
    assert!(memory_line.contains("OS空き"), "{memory_line}");
}

/// 情報欄は幅が限られるので先頭を省略するが、ステータス行にはフルパスが残る。
#[test]
fn a_long_patch_name_is_truncated_from_the_head_in_the_grid() {
    let mut screen = GridSequencerScreen::new(None);
    screen.state.rows_mut()[0].patch =
        Some("patches_factory/Very/Deeply/Nested/Directory/Bright Lead.fxp".to_string());

    let rendered = render(&screen);
    let first_row = rendered.lines().nth(FIRST_ROW_Y).unwrap();

    assert!(first_row.contains('…'), "{first_row}");
    assert!(first_row.contains("Bright Lead.fxp"), "{first_row}");
    assert!(!first_row.contains("patches_factory"), "{first_row}");
}
