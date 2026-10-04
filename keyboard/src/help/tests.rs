use cmrt_core::EffectPlugins;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::effect_pane::tests::{catalog, context, patch_load, screen_with};
use crate::{KeyboardAction, KeyboardContext, KeyboardScreen, PatchPaneFocus, KEYBOARD_NOTES};

fn press(screen: &mut KeyboardScreen<'_>, code: KeyCode, ctx: &KeyboardContext<'_>) {
    screen.handle_key(KeyEvent::new(code, KeyModifiers::NONE), ctx);
}

#[test]
fn each_pane_opens_help_with_question_mark_and_both_shift_forms() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    press(&mut screen, KeyCode::Home, &ctx);
    for (delta, focus) in [
        (-2, PatchPaneFocus::Role),
        (1, PatchPaneFocus::Preset),
        (1, PatchPaneFocus::Patches),
        (1, PatchPaneFocus::Effect),
    ] {
        screen.state.patch_catalog.move_focus(delta);
        assert_eq!(screen.state.patch_catalog.focus(), focus);
        for modifiers in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
            screen.handle_key(KeyEvent::new(KeyCode::Char('?'), modifiers), &ctx);
            assert!(screen.help_open());
            assert!(screen.blocks_screen_switch());
            screen.handle_key(KeyEvent::new(KeyCode::Char('?'), modifiers), &ctx);
            assert!(!screen.help_open());
        }
    }
    press(&mut screen, KeyCode::Char('?'), &ctx);
    press(&mut screen, KeyCode::Esc, &ctx);
    assert!(!screen.help_open());
}

#[test]
fn modifiers_repeat_and_release_do_not_toggle_help() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    for open in [false, true] {
        if open {
            press(&mut screen, KeyCode::Char('?'), &ctx);
        }
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
            screen.handle_key(KeyEvent::new(KeyCode::Char('?'), modifiers), &ctx);
            assert_eq!(screen.help_open(), open);
        }
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            for code in [KeyCode::Char('?'), KeyCode::Esc] {
                screen.handle_key(
                    KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind),
                    &ctx,
                );
                assert_eq!(screen.help_open(), open);
            }
        }
    }
}

#[test]
fn start_and_resume_close_help() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    press(&mut screen, KeyCode::Char('?'), &ctx);
    screen.start(Some("Leads/Lead 1.fxp".to_string()), &ctx);
    assert!(!screen.help_open());
    press(&mut screen, KeyCode::Char('?'), &ctx);
    screen.resume(&ctx);
    assert!(!screen.help_open());
}

#[test]
fn help_consumes_navigation_note_press_edit_and_quit_but_forwards_release() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    assert!(screen.state.press(KEYBOARD_NOTES[0]).is_some());
    press(&mut screen, KeyCode::Char('?'), &ctx);
    let patch = screen.state.patch().map(str::to_string);
    for code in [
        KeyCode::Char('j'),
        KeyCode::Char('l'),
        KeyCode::Char('c'),
        KeyCode::Char('i'),
        KeyCode::Char('q'),
    ] {
        assert!(matches!(
            screen.handle_key(KeyEvent::new(code, KeyModifiers::NONE), &ctx),
            KeyboardAction::Continue
        ));
    }
    assert_eq!(screen.state.patch(), patch.as_deref());
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Patches);
    assert_eq!(screen.state.held(), [KEYBOARD_NOTES[0]]);
    assert!(!screen.mml_input.is_active());
    assert!(!screen.periodic_timeline.is_stopped());
    screen.handle_key(
        KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ),
        &ctx,
    );
    assert!(screen.state.held().is_empty());
    assert!(screen.help_open());
    press(&mut screen, KeyCode::Esc, &ctx);
    assert!(matches!(
        screen.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE), &ctx),
        KeyboardAction::Quit
    ));
}

#[test]
fn help_leaves_periodic_playback_running_and_consumes_effect_keys() {
    let load = patch_load();
    let ctx = context(&load);
    let stage = catalog().presets()[0].json_element();
    let mut screen = screen_with(
        EffectPlugins::with_catalog(catalog()),
        vec![stage.clone()],
        &ctx,
    );
    press(&mut screen, KeyCode::Char('l'), &ctx);
    let now = std::time::Instant::now();
    screen.state.cycle_note_playback(now);
    let generation = screen.state.periodic_generation();
    let mode = screen.state.note_playback_mode();
    press(&mut screen, KeyCode::Char('?'), &ctx);
    for ch in ['a', 'b', 'd', 'd', 'r', 't'] {
        press(&mut screen, KeyCode::Char(ch), &ctx);
    }
    assert!(!screen.effect.is_adding());
    assert_eq!(screen.effect.chain(), [stage]);
    assert_eq!(screen.state.note_playback_mode(), mode);
    assert_eq!(screen.state.periodic_generation(), generation);
    assert!(!screen.periodic_timeline.is_stopped());
    press(&mut screen, KeyCode::Esc, &ctx);
    assert_eq!(screen.state.periodic_generation(), generation);
}

#[test]
fn opening_help_clears_navigation_count_and_pending_effect_delete() {
    let load = cmrt_tui_core::patch_load::PatchLoadState::ready(
        ["Leads/Lead 1.fxp", "Leads/Lead 2.fxp", "Leads/Lead 3.fxp"]
            .iter()
            .map(|name| (name.to_string(), name.to_lowercase()))
            .collect(),
    );
    let ctx = context(&load);
    let mut screen = screen_with(
        EffectPlugins::with_catalog(catalog()),
        vec![catalog().presets()[0].json_element()],
        &ctx,
    );
    for ch in ['9', '?'] {
        press(&mut screen, KeyCode::Char(ch), &ctx);
    }
    press(&mut screen, KeyCode::Esc, &ctx);
    press(&mut screen, KeyCode::Char('j'), &ctx);
    assert_eq!(screen.state.patch(), Some("Leads/Lead 2.fxp"));
    press(&mut screen, KeyCode::Char('l'), &ctx);
    press(&mut screen, KeyCode::Char('d'), &ctx);
    assert!(screen.effect.editor().pending_delete);
    press(&mut screen, KeyCode::Char('?'), &ctx);
    press(&mut screen, KeyCode::Esc, &ctx);
    press(&mut screen, KeyCode::Char('d'), &ctx);
    assert_eq!(screen.effect.chain().len(), 1);
    assert!(screen.effect.editor().pending_delete);
}

#[test]
fn existing_input_and_selection_overlays_keep_question_mark() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(
        EffectPlugins::with_catalog(catalog()),
        vec![catalog().presets()[0].json_element()],
        &ctx,
    );
    press(&mut screen, KeyCode::Char('i'), &ctx);
    press(&mut screen, KeyCode::Char('?'), &ctx);
    assert_eq!(screen.mml_input.value(), "?");
    assert!(!screen.help_open());
    press(&mut screen, KeyCode::Esc, &ctx);
    press(&mut screen, KeyCode::Char('/'), &ctx);
    press(&mut screen, KeyCode::Char('?'), &ctx);
    assert_eq!(screen.patch_filter.value(), "?");
    assert!(!screen.help_open());
    press(&mut screen, KeyCode::Esc, &ctx);
    press(&mut screen, KeyCode::Char('x'), &ctx);
    press(&mut screen, KeyCode::Char('?'), &ctx);
    assert!(screen.state.numeric_input().is_some());
    assert!(!screen.help_open());
    press(&mut screen, KeyCode::Esc, &ctx);
    press(&mut screen, KeyCode::Char('M'), &ctx);
    press(&mut screen, KeyCode::Char('?'), &ctx);
    assert!(screen.plugin_menu().is_some());
    assert!(!screen.help_open());
    press(&mut screen, KeyCode::Esc, &ctx);
    press(&mut screen, KeyCode::Char('l'), &ctx);
    for opening in ['a', 'r'] {
        press(&mut screen, KeyCode::Char(opening), &ctx);
        assert!(screen.effect.is_adding());
        press(&mut screen, KeyCode::Char('?'), &ctx);
        assert!(screen.effect.is_adding());
        assert!(!screen.help_open());
        press(&mut screen, KeyCode::Char('/'), &ctx);
        press(&mut screen, KeyCode::Char('?'), &ctx);
        assert!(screen.effect.is_typing());
        assert_eq!(screen.effect.editor().add.query, "?");
        assert!(!screen.help_open());
        press(&mut screen, KeyCode::Esc, &ctx);
        press(&mut screen, KeyCode::Esc, &ctx);
    }
}
