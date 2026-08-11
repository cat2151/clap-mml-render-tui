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
    let mut screen = screen_with_first_row(60, &[0, 3, 7]);
    screen
        .state
        .set_chord(Some(chord_playback()), Instant::now());
    screen
}

/// 和音の行は1つのAttackと小節末尾までのTieを見せる。
#[test]
fn the_chord_row_shows_an_attack_tied_through_the_measure() {
    let screen = chord_screen();

    let rendered = render(&screen);
    let first_row = rendered.lines().nth(CHORD_FIRST_ROW_Y).unwrap();

    assert!(!first_row.contains("1/1"), "{first_row}");
    assert!(first_row.contains("  61"), "ルート音を出す: {first_row}");
    assert_eq!(
        slice_chars(first_row, first_cell_x(&screen), CELLS_WIDTH),
        format!("# {}", "- ".repeat(GRID_STEPS - 1)),
    );
}

#[test]
fn other_rows_keep_their_own_cells_while_the_chord_row_is_playing() {
    let mut screen = chord_screen();
    // chord ON の並びは 和音 / bass(octave 上/root) / 4 voice(lane 3〜0) / Single lane…。
    screen.state.rows_mut()[3].pattern.draw_span(2, 2);

    let rendered = render(&screen);
    let single_lane_row = rendered.lines().nth(CHORD_FIRST_ROW_Y + 7).unwrap();

    assert_eq!(
        slice_chars(single_lane_row, first_cell_x(&screen) + 4, 2),
        "# ",
        "{single_lane_row}"
    );
}

/// bass 行は lane の pattern をそのまま鳴らすので、セルは編集結果がそのまま出る。
/// 2 lane あり、octave 上が上段・root が下段。
#[test]
fn the_bass_row_shows_its_own_cells_under_the_chord_row() {
    let mut screen = chord_screen();
    // `rows_mut()[1]` は Deref で lane 0（root）を指す。
    screen.state.rows_mut()[1].pattern.draw_span(2, 2);
    screen.state.instances_mut()[1].lanes[1]
        .pattern
        .draw_span(6, 6);

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();
    let octave_row = lines[CHORD_FIRST_ROW_Y + 1];
    let root_row = lines[CHORD_FIRST_ROW_Y + 2];

    // 行番号と patch は最上段（octave 上）に付く。
    assert!(octave_row.contains("  2 8 "), "{octave_row}");
    assert!(root_row.contains("    B "), "{root_row}");
    assert_eq!(
        slice_chars(root_row, first_cell_x(&screen) + 4, 2),
        "# ",
        "{root_row}"
    );
    assert_eq!(
        slice_chars(octave_row, first_cell_x(&screen) + 12, 2),
        "# ",
        "{octave_row}"
    );
}

#[test]
fn the_chord_rows_render_a_summary_a_bass_and_four_grouped_voice_rows() {
    let mut screen = GridSequencerScreen::with_track_count(None, 4);
    screen.state.instances_mut()[2].patch = Some("Leads/Mono Lead.fxp".to_string());
    screen
        .state
        .set_chord(Some(chord_playback()), Instant::now());

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();
    let rows = &lines[CHORD_FIRST_ROW_Y..CHORD_FIRST_ROW_Y + 7];
    assert!(rows[0].contains("  1 C "), "{}", rows[0]);
    assert!(rows[1].contains("  2 8 "), "bass octave row: {}", rows[1]);
    assert!(rows[2].contains("    B "), "bass root row: {}", rows[2]);
    assert!(rows[3].contains("  3 4 "), "{}", rows[3]);
    assert!(rows[3].contains("  73 "), "triad octave voice: {}", rows[3]);
    assert!(rows[4].contains("    3 "), "{}", rows[4]);
    assert!(rows[4].contains("  68 "), "{}", rows[4]);
    assert!(rows[5].contains("    2 "), "{}", rows[5]);
    assert!(rows[5].contains("  65 "), "{}", rows[5]);
    assert!(rows[6].contains("    1 "), "{}", rows[6]);
    assert!(
        rows[6].contains("  61 "),
        "root must be bottom: {}",
        rows[6]
    );
    assert_eq!(rendered.matches("Leads/Mono Lead.fxp").count(), 1);
    assert!(rendered.contains("4i/8l"), "{rendered}");
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
        contains_ignoring_spaces(&rendered, "行1=和音 行2=bass 行3=4 voice"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "auto voicing"),
        "{rendered}"
    );
    // 48セルに収まらないので2行に割ってある。
    assert!(
        contains_ignoring_spaces(&rendered, "bass行 Whole/8th/8th+Oct/#-##/#-##+Oct/"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "####+Oct"),
        "{rendered}"
    );
    assert!(
        contains_ignoring_spaces(&rendered, "無くなります(自動切替あり)"),
        "24行の端末でも help の末尾まで表示できること: {rendered}"
    );
}

#[test]
fn the_keybind_line_mentions_the_chord_mode() {
    let screen = GridSequencerScreen::new(None);

    assert!(render(&screen).contains("c:chord"));
}
