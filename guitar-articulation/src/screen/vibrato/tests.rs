use crossterm::event::KeyModifiers;

use super::*;
use crate::{Rule, VibratoSettings};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

fn screen_with_mml() -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.commit_mml("o3 t60 l1 e g").unwrap();
    screen
}

#[test]
fn four_settings_change_independently_of_column_vibrato_and_survive_esc() {
    let mut screen = screen_with_mml();
    screen.handle_key_event(key(KeyCode::Char('l')));
    let history_len = screen.history().entries.len();
    assert_eq!(
        screen.rules().vibrato_settings(),
        VibratoSettings::default()
    );
    assert_eq!(screen.rules().param(21), 95);
    assert_eq!(
        screen.handle_key_event(shift('V')),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.vibrato_selected(), Some(0));
    assert_eq!(screen.history().entries.len(), history_len);

    screen.handle_key_event(key(KeyCode::Up));
    assert_eq!(screen.vibrato_selected(), Some(0));
    for (row, change) in [
        KeyCode::Char('l'),
        KeyCode::Right,
        KeyCode::Char('l'),
        KeyCode::Right,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(screen.vibrato_selected(), Some(row));
        assert_eq!(
            screen.handle_key_event(key(change)),
            GuitarArticulationAction::Play(Take::Converted)
        );
        screen.handle_key_event(key(if row % 2 == 0 {
            KeyCode::Char('j')
        } else {
            KeyCode::Down
        }));
    }
    assert_eq!(screen.vibrato_selected(), Some(3));
    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.vibrato_selected(), Some(2));
    assert_eq!(
        screen.rules().vibrato_settings(),
        VibratoSettings {
            delay_ms: 350,
            rise_ms: 450,
            depth: 72
        }
    );
    assert_eq!(screen.rules().param(21), 103);
    assert_eq!(screen.history().entries.len(), history_len + 4);
    assert!(!screen.rules().is_on(1, Rule::Vibrato));
    assert_eq!(screen.cursor(), 1, "左右キーは背面の列移動にならない");

    for blocked in [
        key(KeyCode::Char('v')),
        key(KeyCode::Char('m')),
        key(KeyCode::Char('n')),
        key(KeyCode::Char('i')),
        key(KeyCode::Char('u')),
        key(KeyCode::Char('q')),
        shift('R'),
        key(KeyCode::Char('?')),
    ] {
        assert_eq!(
            screen.handle_key_event(blocked),
            GuitarArticulationAction::Continue
        );
    }
    assert!(!screen.note_preview());
    assert!(!screen.repeat());
    assert!(!screen.input_open());
    assert!(!screen.help_open());
    assert!(screen.rules().is_empty());

    let retained = screen.rules().clone();
    let history = screen.history().clone();
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Esc)),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.vibrato_selected(), None);
    assert_eq!(screen.rules(), &retained);
    assert_eq!(screen.history(), &history);
    screen.handle_key_event(key(KeyCode::Char('v')));
    assert!(
        screen.rules().is_on(1, Rule::Vibrato),
        "閉じた後の v は列の ON/OFF"
    );
}

#[test]
fn all_settings_clamp_without_replaying_or_recording_at_the_limits() {
    for (settings, speed, direction, expected) in [
        (
            VibratoSettings {
                delay_ms: 4975,
                rise_ms: 4975,
                depth: 124,
            },
            124,
            KeyCode::Char('l'),
            VibratoSettings {
                delay_ms: 5000,
                rise_ms: 5000,
                depth: 127,
            },
        ),
        (
            VibratoSettings {
                delay_ms: 25,
                rise_ms: 25,
                depth: 3,
            },
            3,
            KeyCode::Left,
            VibratoSettings {
                delay_ms: 0,
                rise_ms: 0,
                depth: 0,
            },
        ),
    ] {
        let mut screen = screen_with_mml();
        screen.rules.set_vibrato_settings(settings);
        screen.rules.step_param(21, speed - 95);
        screen.handle_key_event(shift('V'));
        for _ in 0..4 {
            assert_eq!(
                screen.handle_key_event(key(direction)),
                GuitarArticulationAction::Play(Take::Converted)
            );
            let events = screen.events(Take::Converted).to_vec();
            let history = screen.history().clone();
            assert_eq!(
                screen.handle_key_event(key(direction)),
                GuitarArticulationAction::Continue
            );
            assert_eq!(screen.events(Take::Converted), events.as_slice());
            assert_eq!(screen.history(), &history);
            screen.handle_key_event(key(KeyCode::Down));
        }
        assert_eq!(screen.rules().vibrato_settings(), expected);
        assert_eq!(screen.rules().param(21), expected.depth);
    }
}

#[test]
fn changes_request_the_existing_whole_phrase_note_and_repeat_modes() {
    for (note_preview, repeat) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut screen = screen_with_mml();
        screen.handle_key_event(key(KeyCode::Char('l')));
        if note_preview {
            screen.handle_key_event(key(KeyCode::Char('n')));
        }
        if repeat {
            screen.handle_key_event(shift('R'));
        }
        screen.handle_key_event(shift('V'));
        let expected = if note_preview {
            GuitarArticulationAction::PlayNote {
                take: Take::Converted,
                column: 1,
            }
        } else {
            GuitarArticulationAction::Play(Take::Converted)
        };
        let history_len = screen.history().entries.len();
        for _ in 0..2 {
            assert_eq!(screen.handle_key_event(key(KeyCode::Right)), expected);
        }
        assert_eq!(
            screen.history().entries.len(),
            history_len + 2,
            "変更ごとに記録する"
        );
        assert_eq!(screen.repeat(), repeat);
        screen.handle_key_event(key(KeyCode::Esc));
        assert_eq!(
            screen.repeat(),
            repeat,
            "閉じる操作で既存の repeat を止めない"
        );
    }
}

#[test]
fn a_depth_setting_change_rebuilds_the_converted_take_before_auditioning() {
    let mut screen = screen_with_mml();
    screen.handle_key_event(key(KeyCode::Char('v')));
    let before = screen.events(Take::Converted).to_vec();
    screen.handle_key_event(shift('V'));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Right)),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_ne!(screen.events(Take::Converted), before.as_slice());
    let first_positive = screen
        .events(Take::Converted)
        .iter()
        .find(|event| event.message[0] == 0xB0 && event.message[1] == 20 && event.message[2] > 0)
        .unwrap();
    assert!(
        first_positive.seconds > 0.350,
        "新しい待機 350ms を反映する"
    );
    assert!(screen.rules().is_on(0, Rule::Vibrato));
    assert_eq!(screen.history().entries[0].rules, *screen.rules());
}

#[test]
fn text_input_and_existing_overlays_keep_priority_over_shift_v() {
    let mut screen = screen_with_mml();
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(shift('V'));
    assert_eq!(screen.vibrato_selected(), None);
    assert!(cmrt_tui_core::text_input::textarea_value(screen.input().unwrap()).ends_with('V'));
    screen.handle_key_event(key(KeyCode::Esc));

    screen.handle_key_event(shift('O'));
    screen.handle_key_event(shift('V'));
    assert_eq!(screen.vibrato_selected(), None);
    assert_eq!(
        cmrt_tui_core::text_input::textarea_value(screen.smf_input().unwrap()),
        "V"
    );
    screen.handle_key_event(key(KeyCode::Esc));

    screen.handle_key_event(key(KeyCode::Char('u')));
    screen.handle_key_event(shift('V'));
    assert_eq!(screen.vibrato_selected(), None);
    assert_eq!(screen.param_list_selected(), Some(0));
    screen.handle_key_event(key(KeyCode::Esc));
    screen.handle_key_event(key(KeyCode::Char('?')));
    screen.handle_key_event(shift('V'));
    assert!(screen.help_open());
    assert_eq!(screen.vibrato_selected(), None);
    screen.handle_key_event(key(KeyCode::Esc));
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        screen.handle_key_event(KeyEvent::new(
            KeyCode::Char('V'),
            modifiers | KeyModifiers::SHIFT,
        ));
        assert_eq!(screen.vibrato_selected(), None);
    }
}

#[test]
fn speed_is_shared_with_u_and_overlay_changes_are_restored_by_history() {
    let mut screen = screen_with_mml();
    screen.handle_key_event(shift('V'));
    for _ in 0..4 {
        screen.handle_key_event(key(KeyCode::Char('l')));
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    let prepared = screen.rules().clone();
    assert_eq!(prepared.param(21), 103);
    screen.handle_key_event(key(KeyCode::Esc));
    screen.handle_key_event(key(KeyCode::Char('u')));
    assert_eq!(screen.param_list_selected(), Some(0));
    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(screen.rules().param(21), 111);
    assert!(screen
        .events(Take::Converted)
        .iter()
        .any(|event| event.message == [0xB0, 21, 111]));
    screen.handle_key_event(key(KeyCode::Esc));
    screen.handle_key_event(shift('V'));
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(
        screen.rules(),
        &prepared,
        "専用 overlay も u で変更した速度から ±8"
    );
    screen.handle_key_event(key(KeyCode::Esc));

    // 重複履歴は既存の履歴経路で先頭に寄る。1 件前の u の速度 111 を選ぶ。
    screen.handle_key_event(shift('H'));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(
        screen.rules().vibrato_settings(),
        prepared.vibrato_settings()
    );
    assert_eq!(screen.rules().param(21), 111);
    let saved = serde_json::to_string(screen.history()).unwrap();
    let restored =
        GuitarArticulationScreen::default().with_history(serde_json::from_str(&saved).unwrap());
    assert_eq!(restored.rules(), screen.rules());
}

#[test]
fn smf_material_can_change_settings_without_adding_mml_history() {
    let mut screen = screen_with_mml();
    let history = screen.history().clone();
    let material = screen.events(Take::Plain).to_vec();
    screen.load_smf(std::path::PathBuf::from("song.mid"), Ok(material));
    screen.handle_key_event(shift('V'));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Right)),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.rules().vibrato_settings().delay_ms, 350);
    assert_eq!(screen.history(), &history);
}
