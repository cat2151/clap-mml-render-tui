use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::test_effects::{amp_plugins, amp_stage};
use crate::{convert, ColumnRuleAnchor, RowRule, Rule, RuleTable, Take};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// `i` → 文字 → `Enter` で MML を確定した画面。
fn screen_with_mml(mml: &str) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));
    for ch in mml.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    screen
}

fn entry(mml: &str, rules: RuleTable) -> GuitarArticulationHistoryEntry {
    GuitarArticulationHistoryEntry {
        mml: mml.to_string(),
        rules,
        effect_chain: Vec::new(),
        arp: Default::default(),
        anchor: None,
    }
}

#[test]
fn committing_mml_then_toggling_rules_stacks_each_state_newest_first() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Char('e')));

    let mut after_a = RuleTable::default();
    after_a.toggle(1, Rule::HammerPull);
    let mut after_e = after_a.clone();
    after_e.toggle_row(RowRule::EconomyPicking);
    let anchor = Some(ColumnRuleAnchor::new(
        "o3 l8 e g a",
        &Default::default(),
        &after_a,
    ));
    assert_eq!(
        screen.history().entries,
        vec![
            GuitarArticulationHistoryEntry {
                anchor: anchor.clone(),
                ..entry("o3 l8 e g a", after_e)
            },
            GuitarArticulationHistoryEntry {
                anchor,
                ..entry("o3 l8 e g a", after_a)
            },
            entry("o3 l8 e g a", RuleTable::default()),
        ]
    );
}

#[test]
fn playing_does_not_stack() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('b')));
    screen.handle_key_event(key(KeyCode::Char(' ')));

    assert_eq!(screen.history().entries.len(), 1);
}

#[test]
fn entering_with_the_default_mml_stacks_it() {
    let mut screen = GuitarArticulationScreen::default();
    screen.enter();

    assert_eq!(
        screen.history().entries,
        vec![entry(crate::DEFAULT_MML, RuleTable::default())]
    );
}

#[test]
fn committing_the_effect_chain_stacks_it_but_previewing_does_not() {
    let mut screen = GuitarArticulationScreen::with_effect_plugins(amp_plugins());
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));
    screen.handle_key_event(key(KeyCode::Char('x')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.history().entries.len(), 1);

    screen.handle_key_event(key(KeyCode::Enter));

    let entries = &screen.history().entries;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].effect_chain, vec![amp_stage()]);
    assert!(entries[1].effect_chain.is_empty());
}

#[test]
fn applying_an_entry_rebuilds_the_takes_and_keeps_the_column_rules() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle_row(RowRule::EconomyPicking);
    let target = GuitarArticulationHistoryEntry {
        mml: "o3 l8 e g a".to_string(),
        rules: rules.clone(),
        effect_chain: vec![amp_stage()],
        arp: Default::default(),
        anchor: Some(ColumnRuleAnchor::new(
            "o3 l8 e g a",
            &Default::default(),
            &rules,
        )),
    };
    let mut screen = screen_with_mml("o4 c");

    screen.apply_history_entry(&target);

    let plain = cmrt_chord::timed_performance("o3 l8 e g a").unwrap().events;
    assert_eq!(screen.mml(), "o3 l8 e g a");
    assert_eq!(screen.events(Take::Plain), plain.as_slice());
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, &rules).as_slice()
    );
    assert!(screen.rules().is_on(1, Rule::HammerPull));
    assert_eq!(screen.effect_chain(), [amp_stage()]);
    assert_eq!(screen.history_entry(), target);
    assert!(screen.error.is_none());
}

#[test]
fn applying_an_unparsable_entry_keeps_the_state_and_explains() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    let before = screen.history_entry();
    let converted = screen.events(Take::Converted).to_vec();

    screen.apply_history_entry(&entry("[[[", RuleTable::default()));

    assert_eq!(screen.history_entry(), before);
    assert_eq!(screen.events(Take::Converted), converted.as_slice());
    assert!(screen.error.is_some());
}

fn shift_h() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT)
}

/// MML 確定 → `l` → `a` → `e` の後、MML 欄を閉じて matrix 操作中にした画面。履歴は 3 件。
fn screen_with_three_states() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Char('e')));
    assert!(!screen.input_open());
    assert_eq!(screen.history().entries.len(), 3);
    screen
}

#[test]
fn shift_h_in_the_mml_field_types_instead_of_opening() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));

    let action = screen.handle_key_event(shift_h());

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert_eq!(screen.history_overlay_selected(), None);
    let input = screen.input().expect("MML 欄は開いたまま");
    assert_eq!(cmrt_tui_core::text_input::textarea_value(input), "H");
}

#[test]
fn shift_h_in_the_matrix_opens_with_the_state_just_before_on_top() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    // 画面の状態を、履歴の先頭と違うものにしてから開く（履歴は MML 確定の 1 件だけ）。
    screen.apply_history_entry(&entry("o4 c d", RuleTable::default()));
    let just_before = screen.history_entry();

    let action = screen.handle_key_event(shift_h());

    assert_eq!(action, GuitarArticulationAction::SaveHistory);
    assert_eq!(screen.history_overlay_selected(), Some(0));
    assert_eq!(screen.history().entries.len(), 2);
    assert_eq!(screen.history().entries[0], just_before);
}

#[test]
fn j_applies_the_next_older_entry_and_asks_to_play_it() {
    let mut screen = screen_with_three_states();
    screen.handle_key_event(shift_h());
    let older = screen.history().entries[1].clone();

    let action = screen.handle_key_event(key(KeyCode::Char('j')));

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert_eq!(screen.history_overlay_selected(), Some(1));
    assert_eq!(screen.history_entry(), older);
    let plain = cmrt_chord::timed_performance(&older.mml).unwrap().events;
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, &older.rules).as_slice()
    );

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('k'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.history_overlay_selected(), Some(0));
}

#[test]
fn j_on_the_oldest_and_k_on_the_newest_do_nothing() {
    let mut screen = screen_with_three_states();
    screen.handle_key_event(shift_h());

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('k'))),
        GuitarArticulationAction::Continue
    );
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.history_overlay_selected(), Some(2));

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('j'))),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.history_overlay_selected(), Some(2));
}

#[test]
fn esc_and_shift_h_restore_the_state_just_before_opening() {
    for close in [key(KeyCode::Esc), shift_h()] {
        let mut screen = screen_with_three_states();
        let before = screen.history_entry();
        let converted = screen.events(Take::Converted).to_vec();
        screen.handle_key_event(shift_h());
        screen.handle_key_event(key(KeyCode::Char('j')));
        screen.handle_key_event(key(KeyCode::Char('j')));
        assert_ne!(screen.history_entry(), before);

        let action = screen.handle_key_event(close);

        assert_eq!(action, GuitarArticulationAction::Continue);
        assert_eq!(screen.history_overlay_selected(), None);
        assert_eq!(screen.history_entry(), before);
        assert_eq!(screen.events(Take::Converted), converted.as_slice());
    }
}

#[test]
fn enter_keeps_the_chosen_entry_and_moves_it_to_the_top() {
    let mut screen = screen_with_three_states();
    screen.handle_key_event(shift_h());
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    let chosen = screen.history().entries[2].clone();

    let action = screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(action, GuitarArticulationAction::SaveHistory);
    assert_eq!(screen.history_overlay_selected(), None);
    assert_eq!(screen.history_entry(), chosen);
    assert_eq!(screen.history().entries[0], chosen);
    assert_eq!(screen.history().entries.len(), 3);
}

#[test]
fn screen_keys_do_nothing_while_the_overlay_is_open() {
    let mut screen = screen_with_three_states();
    screen.handle_key_event(shift_h());
    let rules = screen.rules().clone();

    for code in [KeyCode::Char('a'), KeyCode::Char('x'), KeyCode::Char('q')] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            GuitarArticulationAction::Continue
        );
    }

    assert_eq!(screen.rules(), &rules);
    assert!(!screen.effect_overlay_open());
    assert_eq!(screen.history_overlay_selected(), Some(0));
}

#[test]
fn with_history_restores_the_newest_entry_and_entering_does_not_overwrite_it() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    let history = GuitarArticulationHistory {
        entries: vec![
            entry("o3 l8 e g a", rules.clone()),
            entry("o4 c d", RuleTable::default()),
        ],
    };
    let mut screen = GuitarArticulationScreen::default().with_history(history.clone());
    screen.enter();

    assert_eq!(screen.mml(), "o3 l8 e g a");
    assert_eq!(*screen.rules(), rules);
    let flat = cmrt_chord::timed_performance("o3 l8 e g a").unwrap().events;
    assert_eq!(
        screen.events(Take::Converted),
        convert(&flat, &rules).as_slice()
    );
    assert_eq!(*screen.history(), history, "起動時の復帰では積まない");
}

#[test]
fn with_empty_history_entering_falls_back_to_the_default_mml() {
    let mut screen =
        GuitarArticulationScreen::default().with_history(GuitarArticulationHistory::default());
    screen.enter();

    assert_eq!(screen.mml(), crate::DEFAULT_MML);
}
