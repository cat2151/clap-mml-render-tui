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
    ] {
        assert!(matrix.contains(label), "{label} が matrix に無い: {matrix}");
    }
    let status = squeezed(&rows_in(&buffer, layout.status));
    assert!(status.contains("mp/cvg:奏法"), "{status}");
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
