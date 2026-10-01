use crossterm::event::KeyCode;

use cmrt_tui_core::theme::{MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PURPLE};

use super::{key, label_fg, mark_cells, render, row_marks, rows_in, screen_with_mml, squeezed};
use crate::ui::layout_for;

#[test]
fn column_rules_share_five_rows_below_the_row_rules() {
    let screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let rows = rows_in(&buffer, layout.matrix);
    let position = |label: &str| {
        rows.iter()
            .position(|r| r.contains(label))
            .unwrap_or_else(|| panic!("{label} の段が無い: {rows:#?}"))
    };
    let auto = position("s:auto hammer/pull");
    let lanes: Vec<usize> = [
        "t:KS",
        "v:vibrato",
        "t:long/extra",
        "t:power chord",
        "t:release",
    ]
    .into_iter()
    .map(position)
    .collect();
    assert_eq!(lanes, (auto + 1..auto + 6).collect::<Vec<_>>(), "{rows:#?}");
    // 最後の段の下は枠。
    assert_eq!(auto + 6, rows.len() - 1, "{rows:#?}");
    let status = squeezed(&rows_in(&buffer, layout.status));
    assert!(status.contains("mp/cvg/t:奏法"), "{status}");
}

#[test]
fn turning_on_mute_moves_the_mark_off_the_hammer_pull_row() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Char('m')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let matrix = squeezed(&rows_in(&buffer, layout.matrix));
    assert!(!matrix.contains("a:hammer/pull"), "{matrix}");
    assert_eq!(row_marks(&buffer, layout.matrix, "m:palm mute"), "m");
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    assert!(converted.contains("KSMute_Down"), "{converted}");
}

#[test]
fn a_rule_that_changes_nothing_shows_a_gray_dash() {
    // o7 g（91）はミュートの sample（30〜76）の外。スライドの sample（88 まで）の外でスライドもしない。
    let mut screen = screen_with_mml("o3 l8 g o7 g");
    screen.handle_key_event(key(KeyCode::Char('m')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('m')));
    screen.handle_key_event(key(KeyCode::Char('v')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "m:palm mute"), "m-");
    let cells = mark_cells(&buffer, layout.matrix, "m:palm mute");
    assert_eq!(buffer.cell(cells[0]).unwrap().fg, MONOKAI_GREEN);
    assert_eq!(buffer.cell(cells[1]).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(row_marks(&buffer, layout.matrix, "v:vibrato"), "v");

    screen.handle_key_event(key(KeyCode::Char('/')));
    let buffer = render(&screen);
    assert_eq!(row_marks(&buffer, layout.matrix, "/:slide up/down"), "m-");
    let cells = mark_cells(&buffer, layout.matrix, "/:slide up/down");
    assert_eq!(buffer.cell(cells[1]).unwrap().fg, MONOKAI_GRAY);
}

#[test]
fn the_economy_row_keeps_the_strokes_of_muted_notes() {
    let mut screen = screen_with_mml("o3 l8 e f+ g f+");
    screen.handle_key_event(key(KeyCode::Char('e')));
    for _ in 0..2 {
        screen.handle_key_event(key(KeyCode::Char('m')));
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(
        row_marks(&buffer, layout.matrix, "e:economy picking"),
        "DUDU"
    );
    assert_eq!(row_marks(&buffer, layout.matrix, "t:KS"), "mm");
}

#[test]
fn g_turns_the_cursor_column_into_a_folded_pick_scratch() {
    // o5 c（60）は Pick_Scratch の sample（30〜42）の外なので、C2（36）で鳴らす。
    let mut screen = screen_with_mml("o3 l8 e o5 c");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('g')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "g:pick scratch"), "g");
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    assert!(converted.contains("KSPick_Scratch"), "{converted}");
    assert!(converted.contains("C2"), "{converted}");
    assert!(!converted.contains("C4"), "{converted}");
}

#[test]
fn t_lists_every_column_rule_and_marks_the_ones_on_in_the_cursor_column() {
    let mut screen = screen_with_mml("o6 l8 e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('t')));
    // brush（音域 31〜51）を ON にする。o6 g = 79 は音域外なので灰色。
    screen.handle_key_event(key(KeyCode::Char('d')));
    let buffer = render(&screen);

    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(all.contains("奏法リスト列2"), "{all}");
    for label in [
        "a:hammer/pull",
        "g:pickscratch",
        "b:harmonics",
        "t:pseudolegato",
        "s:autoslideout",
    ] {
        assert!(all.contains(label), "{label}: {all}");
    }
    assert_eq!(label_fg(&buffer, buffer.area, "d:brush"), MONOKAI_GRAY);

    screen.handle_key_event(key(KeyCode::Esc));
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;
    assert_eq!(row_marks(&buffer, matrix, "t:brush"), "-");
    let cell = mark_cells(&buffer, matrix, "t:brush")[0];
    assert_eq!(buffer.cell(cell).unwrap().fg, MONOKAI_GRAY);
    assert!(!squeezed(&rows_in(&buffer, buffer.area)).contains("奏法リスト"));
}

/// 奏法リストで `name` の文字を押して ON/OFF し、閉じる。
fn toggle_from_the_rule_list(screen: &mut crate::GuitarArticulationScreen, name: &str) {
    let letter = crate::ui::RULE_ROWS
        .iter()
        .find(|rule_row| rule_row.name == name)
        .unwrap()
        .overlay_key;
    screen.handle_key_event(key(KeyCode::Char('t')));
    screen.handle_key_event(key(KeyCode::Char(letter)));
    screen.handle_key_event(key(KeyCode::Esc));
}

#[test]
fn chromatic_run_is_gray_where_the_folded_key_has_no_phrase() {
    // o3 d+（39）は畳んでも 30〜38 に入らない。o3 c（36）はそのキーのフレーズ。
    let mut screen = screen_with_mml("o3 l8 d+ c");
    toggle_from_the_rule_list(&mut screen, "chromatic run");
    screen.handle_key_event(key(KeyCode::Char('l')));
    toggle_from_the_rule_list(&mut screen, "chromatic run");
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "t:chromatic run"), "-C");
    let cells = mark_cells(&buffer, layout.matrix, "t:chromatic run");
    assert_eq!(buffer.cell(cells[0]).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell(cells[1]).unwrap().fg, MONOKAI_PURPLE);
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    assert!(converted.contains("KSChromatic_Run"), "{converted}");
}

#[test]
fn an_effect_rule_sounds_its_note_in_place_of_the_column() {
    let mut screen = screen_with_mml("o3 l8 e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    toggle_from_the_rule_list(&mut screen, "fx hard stop");
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "t:fx hard stop"), "J");
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    assert!(converted.contains("KSFX_Hard_Stop"), "{converted}");
    assert!(!converted.contains("G2"), "{converted}");
}

#[test]
fn control_change_rules_stack_and_are_gray_where_the_sample_does_not_react() {
    // o7 e（88）は Sus_Down だが、ポジション移動の離し音（30〜76）の外。
    let mut screen = screen_with_mml("o3 l8 g o7 e");
    for name in ["long/extra", "power chord", "position rel"] {
        toggle_from_the_rule_list(&mut screen, name);
    }
    screen.handle_key_event(key(KeyCode::Char('l')));
    toggle_from_the_rule_list(&mut screen, "position rel");
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "t:long/extra"), "f");
    assert_eq!(row_marks(&buffer, layout.matrix, "t:power chord"), "i");
    assert_eq!(row_marks(&buffer, layout.matrix, "t:position rel"), "n-");
    let cells = mark_cells(&buffer, layout.matrix, "t:position rel");
    assert_eq!(buffer.cell(cells[0]).unwrap().fg, MONOKAI_GREEN);
    assert_eq!(buffer.cell(cells[1]).unwrap().fg, MONOKAI_GRAY);
    // 頭の CC23 / CC32 の値は、頭に積む既定値と一緒に画面の上へ送られるので、イベント列で見る。
    let converted_events = screen.events(crate::Take::Converted);
    for controller in [crate::LONG_EXTRA_CC, crate::POWER_CHORD_CC] {
        assert!(
            converted_events
                .iter()
                .any(|e| e.seconds == 0.0 && e.message == [0xB0, controller, 127]),
            "CC{controller}: {converted_events:?}"
        );
    }
    // イベント一覧はカーソル列の note on を上から 1/3 に置くので、1 列目へ戻して見る。
    screen.handle_key_event(key(KeyCode::Char('h')));
    let buffer = render(&screen);
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    for name in ["long/extra0", "powerchord0", "releasetypeposition72"] {
        assert!(converted.contains(name), "{name}: {converted}");
    }
}

#[test]
fn u_lists_every_param_with_its_value_and_marks_the_changed_ones() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('u')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('h')));
    let buffer = render(&screen);
    let rows = rows_in(&buffer, buffer.area);
    let all = squeezed(&rows);
    assert!(all.contains("パラメータ(行全体)"), "{all}");
    for param in crate::PARAMS {
        let row = rows
            .iter()
            .find(|row| row.contains(&format!("CC{:<3} {}", param.cc, param.name)))
            .unwrap_or_else(|| {
                panic!(
                    "CC{} の行が無い
{all}",
                    param.cc
                )
            });
        let changed = param.cc == 22;
        assert_eq!(row.contains('●'), changed, "{row}");
        let value = if changed { 43 } else { param.default };
        let row = squeezed(std::slice::from_ref(row));
        assert!(
            row.contains(&format!(
                "{}{value}(既定{})",
                param.name.replace(' ', ""),
                param.default
            )),
            "{row}"
        );
    }
    // イベント一覧は CC の名前で出す。
    assert!(all.contains("mutelength43"), "{all}");
}
