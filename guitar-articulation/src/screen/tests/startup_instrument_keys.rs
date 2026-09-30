use super::{key, screen_with_mml};
use crate::{GuitarArticulationAction, Instrument, StartupInstrument};
use crossterm::event::KeyCode;

#[test]
fn f_toggles_the_startup_instrument_and_asks_to_save_without_touching_the_sounding_one() {
    let mut screen = screen_with_mml("cde");
    screen.handle_key_event(key(KeyCode::Esc));
    assert_eq!(screen.startup_instrument(), StartupInstrument::LiteThenFull);

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('f'))),
        GuitarArticulationAction::SaveSettings
    );
    assert_eq!(screen.startup_instrument(), StartupInstrument::Full);
    assert_eq!(screen.instrument(), Instrument::Lite);

    screen.handle_key_event(key(KeyCode::Char('f')));
    assert_eq!(screen.startup_instrument(), StartupInstrument::LiteThenFull);
}

#[test]
fn f_while_typing_goes_into_the_mml() {
    let mut screen = screen_with_mml("cde");
    screen.handle_key_event(key(KeyCode::Char('i')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('f'))),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.startup_instrument(), StartupInstrument::LiteThenFull);
}
