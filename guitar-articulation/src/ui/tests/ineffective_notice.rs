use crossterm::event::KeyCode;

use cmrt_tui_core::theme::MONOKAI_PINK;

use super::{key, render, rows_in, screen_with_mml, squeezed};
use crate::ui::layout_for;
use crate::GuitarArticulationScreen;

/// 全角の 2 セル目は空白で読めるので、知らせは空白を落として比べる（[`squeezed`]）。
const NOTICE: &str = "は効いていない";

/// 奏法リストを開いて power chord（`i`）を切り替え、閉じる。
fn toggle_power_chord(screen: &mut GuitarArticulationScreen) {
    for code in [KeyCode::Char('t'), KeyCode::Char('i'), KeyCode::Enter] {
        screen.handle_key_event(key(code));
    }
}

fn matrix_top(screen: &GuitarArticulationScreen) -> String {
    let buffer = render(screen);
    let layout = layout_for(buffer.area, screen);
    squeezed(&rows_in(&buffer, layout.matrix)[..1])
}

#[test]
fn power_chord_on_a_slide_names_the_rule_and_the_articulation_in_the_matrix_title() {
    // E2 → G2 を slide にすると Slide_Up で、power chord の層（Sus のみ）に届かない。
    let mut screen = screen_with_mml("o3 l8 e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('/')));
    toggle_power_chord(&mut screen);

    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);
    let top = rows_in(&buffer, layout.matrix).remove(0);
    assert!(
        squeezed(std::slice::from_ref(&top))
            .contains("!i:powerchordは効いていない（この列:Slide_UpG2）"),
        "{top}"
    );
    let x = layout.matrix.x + top[..top.find('!').unwrap()].chars().count() as u16;
    assert_eq!(buffer.cell((x, layout.matrix.y)).unwrap().fg, MONOKAI_PINK);
}

#[test]
fn power_chord_above_its_pitches_is_reported_with_the_pitch() {
    // F6（89）は Sus_P5 の音域の外。
    let mut screen = screen_with_mml("o7 l8 f");
    toggle_power_chord(&mut screen);
    let top = matrix_top(&screen);
    assert!(top.contains("（この列:Sus_DownF6）"), "{top}");
}

#[test]
fn an_effective_power_chord_or_another_column_shows_no_notice() {
    let mut screen = screen_with_mml("o3 l8 e g");
    toggle_power_chord(&mut screen);
    assert!(!matrix_top(&screen).contains(NOTICE));

    // 効いていないルールが在るのは列 2 だけ。カーソルが列 1 に在る間は出さない。
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('/')));
    toggle_power_chord(&mut screen);
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert!(!matrix_top(&screen).contains(NOTICE));
}

#[test]
fn the_rule_list_overlay_shows_the_notice_on_its_bottom_border() {
    let mut screen = screen_with_mml("o3 l8 e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('/')));
    screen.handle_key_event(key(KeyCode::Char('t')));
    screen.handle_key_event(key(KeyCode::Char('i')));

    let buffer = render(&screen);
    let rows = rows_in(&buffer, buffer.area);
    // matrix の見出し（`┌` の行）とは別に、overlay の下の枠（`└` の行）にも出る。
    assert!(
        rows.iter().any(|row| row.contains('└')
            && squeezed(std::slice::from_ref(row)).contains("i:powerchordは効いていない")),
        "{rows:#?}"
    );
}
