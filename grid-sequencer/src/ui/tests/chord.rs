//! chord mode の描画（和音の行 / コード進行行 / 再起動アナウンス / ヘルプ）。

use super::*;

/// chord mode 中は grid の上にコード進行行が入るので、grid が1行ぶん下がる。
const CHORD_FIRST_ROW_Y: usize = FIRST_ROW_Y + 1;

/// 枠線が文字の間に入るので、空白を無視して探す。
fn contains_ignoring_spaces(rendered: &str, text: &str) -> bool {
    let needle = text.replace(' ', "");
    rendered
        .lines()
        .any(|line| line.replace(' ', "").contains(&needle))
}

fn chord_playback() -> crate::ChordPlayback {
    crate::ChordPlayback::new(
        "C#",
        "I-IV-V-I".to_string(),
        vec![
            vec![61, 65, 68],
            vec![66, 70, 73],
            vec![68, 72, 75],
            vec![61, 65, 68],
        ],
    )
    .unwrap()
}

fn chord_screen() -> GridSequencerScreen {
    let mut screen = screen_with_first_row(60, StepDuration::Quarter, &[0, 3, 7]);
    screen
        .state
        .set_chord(Some(chord_playback()), Instant::now());
    screen
}

/// 和音の行はセルの設定値ではなく「先頭ステップだけ全音符」を見せる。
#[test]
fn the_chord_row_shows_a_whole_note_on_the_first_step_only() {
    let screen = chord_screen();

    let rendered = render(&screen);
    let first_row = rendered.lines().nth(CHORD_FIRST_ROW_Y).unwrap();

    assert!(first_row.contains("1/1"), "{first_row}");
    assert!(first_row.contains("  61"), "ルート音を出す: {first_row}");
    assert_eq!(
        slice_chars(first_row, FIRST_CELL_X, CELLS_WIDTH),
        format!("# {}", ". ".repeat(GRID_STEPS - 1)),
    );
}

#[test]
fn other_rows_keep_their_own_cells_while_the_chord_row_is_playing() {
    let mut screen = chord_screen();
    screen.state.rows_mut()[1].cells[2] = true;

    let rendered = render(&screen);
    let second_row = rendered.lines().nth(CHORD_FIRST_ROW_Y + 1).unwrap();

    assert_eq!(
        slice_chars(second_row, FIRST_CELL_X + 4, 2),
        "# ",
        "{second_row}"
    );
}

/// 進行は画面下部のステータス行ではなく、grid の直上の1行目に出す。
#[test]
fn the_progression_is_drawn_above_the_grid() {
    let screen = chord_screen();

    let rendered = render(&screen);

    let first_line = rendered.lines().next().unwrap();
    assert!(
        contains_ignoring_spaces(first_line, "chord Key:C# I-IV-V-I [1/4]"),
        "{rendered}"
    );
}

#[test]
fn the_reason_the_chord_mode_could_not_start_is_drawn_above_the_grid() {
    let mut screen = GridSequencerScreen::new(None);
    screen.chord_error = Some("poly patch が見つかりません".to_string());

    let rendered = render(&screen);

    let first_line = rendered.lines().next().unwrap();
    assert!(
        contains_ignoring_spaces(first_line, "chord: poly patch が見つかりません"),
        "{rendered}"
    );
}

/// off のときはコード進行行そのものを出さず、grid が最上段から始まる。
#[test]
fn no_chord_line_is_reserved_while_the_chord_mode_is_off() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render(&screen);

    assert!(!rendered.contains("chord Key:"), "{rendered}");
    assert!(
        rendered.lines().next().unwrap().contains("Grid Sequencer"),
        "{rendered}"
    );
}

#[test]
fn the_restart_notice_overlay_announces_the_updated_progression_data() {
    let mut screen = GridSequencerScreen::new(None);
    screen.restart_notice = Some(Instant::now());

    let rendered = render(&screen);

    assert!(
        contains_ignoring_spaces(&rendered, "コード進行データが更新されました"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "アプリを再起動します"),
        "{rendered}"
    );
}

#[test]
fn the_help_overlay_explains_the_chord_mode() {
    let mut screen = GridSequencerScreen::new(None);
    screen.help_open = true;

    let rendered = render(&screen);

    assert!(
        contains_ignoring_spaces(&rendered, "chord mode の on/off"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "全音符の和音で鳴らし(+6dB)"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "chord_patch_categories"),
        "24行の端末でも help の末尾まで表示できること: {rendered}"
    );
}

#[test]
fn the_keybind_line_mentions_the_chord_mode() {
    let screen = GridSequencerScreen::new(None);

    assert!(render(&screen).contains("c:chord"));
}
