//! 見出しの音色名が、host が渡した段階（Lite / Full 読み込み中 / Full）に従う。

use super::*;
use crate::Instrument;

fn top(screen: &GuitarArticulationScreen) -> String {
    squeezed(&rows_in(&render(screen), Rect::new(0, 0, WIDTH, 1)))
}

#[test]
fn the_title_names_the_instrument_the_host_is_playing() {
    let mut screen = GuitarArticulationScreen::default();
    assert!(
        top(&screen).contains("METAL-GTXLite[起動:"),
        "{}",
        top(&screen)
    );

    screen.set_instrument(Instrument::LiteLoadingFull);
    assert!(
        top(&screen).contains("METAL-GTXLite（Full読み込み中）"),
        "{}",
        top(&screen)
    );

    screen.set_instrument(Instrument::Full);
    assert!(
        top(&screen).contains("METAL-GTXFull[起動:"),
        "{}",
        top(&screen)
    );
}

#[test]
fn the_effect_chain_overlay_names_the_same_instrument() {
    let mut screen =
        GuitarArticulationScreen::with_effect_plugins(crate::test_effects::amp_plugins());
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));
    screen.set_instrument(Instrument::Full);

    screen.handle_key_event(key(KeyCode::Char('x')));
    let chain = squeezed(&rows_in(&render(&screen), Rect::new(0, 0, WIDTH, HEIGHT)));
    assert!(chain.contains("instrument:METAL-GTXFull"), "{chain}");
}

#[test]
fn the_title_marks_dry_while_keeping_the_chain() {
    let mut screen = GuitarArticulationScreen::default();
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!top(&screen).contains("[dry]"), "{}", top(&screen));

    screen.handle_key_event(key(KeyCode::Char('w')));

    assert!(top(&screen).contains("[dry]"), "{}", top(&screen));
}

#[test]
fn the_title_names_the_startup_instrument() {
    let screen = GuitarArticulationScreen::default();
    assert!(
        top(&screen).contains("[起動:Lite→Full]─"),
        "{}",
        top(&screen)
    );

    let screen = screen.with_startup_instrument(crate::StartupInstrument::Full);
    assert!(top(&screen).contains("[起動:Full]─"), "{}", top(&screen));
}
