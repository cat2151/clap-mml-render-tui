use crossterm::event::KeyCode;

use super::{key, render, row_marks, rows_in, screen_with_mml, squeezed};
use crate::ui::layout_for;

#[test]
fn every_articulation_rule_has_its_own_row() {
    let screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    let matrix = squeezed(&rows_in(&buffer, layout.matrix));
    for label in [
        "a:hammer/pull",
        "m:palmmute",
        "p:pinchharmonic",
        "/:slide",
        "c:bend",
        "v:vibrato",
        "g:pickscratch",
        "t:harmonics",
        "t:brush",
        "t:fretmute",
        "t:slideout",
        "t:pseudolegato",
        "t:portamento",
        "t:slidein",
        "t:trillhalf",
        "t:trillwhole",
        "t:trillmin3",
        "t:trillmaj3",
        "t:unisonbend",
        "t:chromaticrun",
        "t:slidefxdown",
        "t:slidefxup",
        "t:slidefxwow",
        "t:fxhello",
        "t:fxresonance",
        "t:fxslidenoise",
        "t:fxhardstop",
        "t:long/extra",
        "t:powerchord",
        "t:positionrel",
    ] {
        assert!(matrix.contains(label), "{label} が matrix に無い: {matrix}");
    }
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

    assert_eq!(row_marks(&buffer, layout.matrix, "a:hammer/pull"), "");
    assert_eq!(row_marks(&buffer, layout.matrix, "m:palm mute"), "●");
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    assert!(converted.contains("KSMute_Down"), "{converted}");
}

#[test]
fn a_rule_that_changes_nothing_shows_a_gray_dash() {
    // o6 g（79）はミュートの sample（30〜76）の外。2 列目は前の列から +11 半音でスライドもしない。
    let mut screen = screen_with_mml("o3 l8 g o6 g");
    screen.handle_key_event(key(KeyCode::Char('m')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('m')));
    screen.handle_key_event(key(KeyCode::Char('v')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "m:palm mute"), "●-");
    assert_eq!(row_marks(&buffer, layout.matrix, "v:vibrato"), "●");
    let rows = rows_in(&buffer, layout.matrix);
    let y = layout.matrix.y + rows.iter().position(|r| r.contains("m:palm mute")).unwrap() as u16;
    let dash = (layout.matrix.x..layout.matrix.x + layout.matrix.width)
        .find(|&x| buffer.cell((x, y)).unwrap().symbol() == "-")
        .unwrap();
    assert_eq!(
        buffer.cell((dash, y)).unwrap().fg,
        cmrt_tui_core::theme::MONOKAI_GRAY
    );

    screen.handle_key_event(key(KeyCode::Char('/')));
    let buffer = render(&screen);
    assert_eq!(row_marks(&buffer, layout.matrix, "/:slide"), "-");
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
    assert_eq!(row_marks(&buffer, layout.matrix, "m:palm mute"), "●●");
}

#[test]
fn g_turns_the_cursor_column_into_a_folded_pick_scratch() {
    // o5 c（60）は Pick_Scratch の sample（30〜42）の外なので、C2（36）で鳴らす。
    let mut screen = screen_with_mml("o3 l8 e o5 c");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('g')));
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    assert_eq!(row_marks(&buffer, layout.matrix, "g:pick scratch"), "●");
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
    // brush（音域 31〜51）を ON にする。o6 g = 79 は音域外なので灰色の -。
    for _ in 0..8 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    let buffer = render(&screen);

    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(all.contains("奏法リスト列2"), "{all}");
    for label in [
        "a:hammer/pull",
        "g:pickscratch",
        "t:harmonics",
        "t:portamento",
    ] {
        assert!(all.contains(label), "{label}: {all}");
    }
    assert!(all.contains("-t:brush"), "{all}");

    screen.handle_key_event(key(KeyCode::Esc));
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;
    assert_eq!(row_marks(&buffer, matrix, "t:brush"), "-");
    assert!(!squeezed(&rows_in(&buffer, buffer.area)).contains("奏法リスト"));
}

/// 奏法リストで `name` の行まで降りて ON にし、閉じる。
fn toggle_from_the_rule_list(screen: &mut crate::GuitarArticulationScreen, name: &str) {
    let index = crate::ui::RULE_ROWS
        .iter()
        .position(|(_, _, row)| *row == name)
        .unwrap();
    screen.handle_key_event(key(KeyCode::Char('t')));
    for _ in 0..index {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    screen.handle_key_event(key(KeyCode::Enter));
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

    assert_eq!(row_marks(&buffer, layout.matrix, "t:chromatic run"), "-●");
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

    assert_eq!(row_marks(&buffer, layout.matrix, "t:fx hard stop"), "●");
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

    assert_eq!(row_marks(&buffer, layout.matrix, "t:long/extra"), "●");
    assert_eq!(row_marks(&buffer, layout.matrix, "t:power chord"), "●");
    assert_eq!(row_marks(&buffer, layout.matrix, "t:position rel"), "●-");
    // イベント一覧はカーソル列の note on を上から 1/3 に置くので、1 列目へ戻して見る
    // （頭の CC23 は画面の上へ送られるので、列の終わりの戻しで見る）。
    screen.handle_key_event(key(KeyCode::Char('h')));
    let buffer = render(&screen);
    let converted = squeezed(&rows_in(&buffer, layout.converted));
    for name in ["long/extra0", "powerchord127", "releasetypeposition72"] {
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
