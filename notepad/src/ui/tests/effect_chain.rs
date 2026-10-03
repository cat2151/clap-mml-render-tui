use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

const PAD_LINE: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#;
const PAD_WITH_HALL_LINE: &str = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Dragonfly Hall Reverb preset":"Medium Clear Hall"}]} l8cdef"#;
const WIDTH: u16 = 240;
const HEIGHT: u16 = 48;

fn screen_on(line: &str) -> NotepadScreen<'static> {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.effect_plugins = crate::tests::test_effect_plugins();
    app.editor.lines = vec![line.to_string()];
    app
}

fn press(app: &mut NotepadScreen<'_>, code: KeyCode) {
    app.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

/// 全角文字の後ろに入る詰め物の空白ごと、空白を除いた画面。
fn screen(app: &mut NotepadScreen<'static>) -> String {
    render_lines(app, WIDTH, HEIGHT).join("\n").replace(' ', "")
}

fn squeezed(text: &str) -> String {
    text.replace(' ', "")
}

#[test]
fn x_draws_the_effect_chain_overlay_with_the_instrument_name() {
    let mut app = screen_on(PAD_LINE);
    press(&mut app, KeyCode::Char('x'));

    let screen = screen(&mut app);

    assert!(screen.contains("┌EFFECTCHAIN─"), "{screen}");
    assert!(screen.contains("instrument:Pads/Pad1.fxp"), "{screen}");
}

#[test]
fn a_draws_the_add_overlay() {
    let mut app = screen_on(PAD_LINE);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('a'));

    let screen = screen(&mut app);

    assert!(screen.contains("EFFECTCHAIN:addpreset"), "{screen}");
    assert!(
        screen.contains(&squeezed(cmrt_effect_chain_select::messages::ADD_FOOTER)),
        "{screen}"
    );
}

#[test]
fn r_draws_the_replace_overlay_and_the_replace_footer() {
    let mut app = screen_on(PAD_WITH_HALL_LINE);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('r'));
    assert!(matches!(app.mode, Mode::EffectChainAdd));

    let screen = screen(&mut app);

    assert!(screen.contains("EFFECTCHAIN:replacepreset"), "{screen}");
    assert_eq!(
        screen.lines().last(),
        Some(squeezed(cmrt_effect_chain_select::messages::REPLACE_FOOTER).as_str())
    );
}

#[test]
fn help_from_the_chain_list_shows_its_page_over_the_overlay() {
    let mut app = screen_on(PAD_LINE);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('?'));
    assert!(matches!(app.mode, Mode::Help));

    let screen = screen(&mut app);

    assert!(screen.contains("EFFECTCHAINoverlay(x)"), "{screen}");
    assert!(screen.contains("行へ書き戻して閉じ"), "{screen}");
    assert!(screen.contains("┌EFFECTCHAIN─"), "{screen}");
}

#[test]
fn help_from_the_add_pane_shows_its_page_over_the_overlay() {
    let mut app = screen_on(PAD_LINE);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Char('?'));
    assert!(matches!(app.mode, Mode::Help));

    let screen = screen(&mut app);

    assert!(
        screen.contains("EFFECTCHAINadd/replaceoverlay(x→a/r)"),
        "{screen}"
    );
    assert!(screen.contains("EFFECTCHAIN:addpreset"), "{screen}");
}

#[test]
fn normal_footer_and_help_mention_x_effect() {
    let mut app = screen_on(PAD_LINE);
    assert!(screen(&mut app).contains("x:effect"));

    press(&mut app, KeyCode::Char('?'));
    let help = screen(&mut app);
    assert!(help.contains("x:effectchain"), "{help}");
}
