use std::collections::HashSet;

use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    KeyboardContext, KeyboardMmlInput, KeyboardNoteGuide, KeyboardScreen, KeyboardState,
    KeyboardVoicingLookup, PatchPaneFocus, KEYBOARD_NOTES,
};

struct NoVoicing;

impl KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

fn patch_load() -> PatchLoadState {
    let names = ["Leads/Lead 1.fxp", "Leads/Lead 2.fxp"]
        .into_iter()
        .map(str::to_string)
        .chain((0..12).map(|index| format!("Pads/Pad {index:02}.fxp")));
    PatchLoadState::ready(
        names
            .map(|name| (name.clone(), name.to_lowercase()))
            .collect(),
    )
}

fn context<'a>(patch_load: &'a PatchLoadState) -> KeyboardContext<'a> {
    KeyboardContext {
        patch_dirs_configured: true,
        patch_load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    }
}

fn screen(patch: &str, ctx: &KeyboardContext<'_>) -> KeyboardScreen<'static> {
    let mut screen = KeyboardScreen::new(
        None,
        KeyboardState::new(Some(patch.to_string())),
        KeyboardMmlInput::default(),
        KeyboardNoteGuide::new(None),
    );
    screen.sync_patch_catalog(ctx);
    screen
}

fn press(screen: &mut KeyboardScreen<'_>, code: KeyCode, ctx: &KeyboardContext<'_>) {
    screen.handle_key(KeyEvent::new(code, KeyModifiers::NONE), ctx);
}

fn type_text(screen: &mut KeyboardScreen<'_>, text: &str, ctx: &KeyboardContext<'_>) {
    for ch in text.chars() {
        press(screen, KeyCode::Char(ch), ctx);
    }
}

fn listed(screen: &KeyboardScreen<'_>) -> Vec<String> {
    screen
        .state
        .patch_catalog
        .patches()
        .map(|patch| patch.display().to_string())
        .collect()
}

#[test]
fn slash_from_the_role_pane_focuses_patches_and_starts_typing() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Pads/Pad 03.fxp", &ctx);
    press(&mut screen, KeyCode::Char('h'), &ctx);
    press(&mut screen, KeyCode::Char('h'), &ctx);
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Role);

    press(&mut screen, KeyCode::Char('/'), &ctx);

    assert!(screen.patch_filter.is_active());
    assert!(screen.is_typing());
    assert_eq!(screen.state.patch_catalog.focus(), PatchPaneFocus::Patches);
}

#[test]
fn each_keystroke_refilters_and_a_vanished_patch_moves_to_the_first() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Pads/Pad 03.fxp", &ctx);
    assert_eq!(listed(&screen).len(), 14);

    press(&mut screen, KeyCode::Char('/'), &ctx);
    type_text(&mut screen, "le", &ctx);
    assert_eq!(listed(&screen), ["Leads/Lead 1.fxp", "Leads/Lead 2.fxp"]);
    assert_eq!(screen.state.patch(), Some("Leads/Lead 1.fxp"));
    type_text(&mut screen, "ad 2", &ctx);
    assert_eq!(listed(&screen), ["Leads/Lead 2.fxp"]);
    assert_eq!(screen.state.patch(), Some("Leads/Lead 2.fxp"));

    press(&mut screen, KeyCode::Enter, &ctx);
    assert!(!screen.patch_filter.is_active());
    assert_eq!(screen.state.patch_catalog.filter(), "lead 2");
}

#[test]
fn escape_restores_the_condition_from_before_typing() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Leads/Lead 1.fxp", &ctx);
    press(&mut screen, KeyCode::Char('/'), &ctx);
    type_text(&mut screen, "lead", &ctx);
    press(&mut screen, KeyCode::Enter, &ctx);

    press(&mut screen, KeyCode::Char('/'), &ctx);
    assert_eq!(screen.patch_filter.value(), "lead");
    type_text(&mut screen, " 2", &ctx);
    assert_eq!(listed(&screen).len(), 1);
    press(&mut screen, KeyCode::Esc, &ctx);

    assert!(!screen.patch_filter.is_active());
    assert_eq!(screen.state.patch_catalog.filter(), "lead");
    assert_eq!(listed(&screen).len(), 2);
    // 戻した一覧にも今の音色（Lead 2）が残っているので、先頭へは跳ばない。
    assert_eq!(screen.state.patch(), Some("Leads/Lead 2.fxp"));
}

#[test]
fn an_invalid_condition_keeps_the_list_and_enter_does_not_close() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Leads/Lead 1.fxp", &ctx);
    press(&mut screen, KeyCode::Char('/'), &ctx);
    type_text(&mut screen, "lead(", &ctx);

    assert!(screen.patch_filter.is_invalid());
    assert_eq!(screen.state.patch_catalog.filter(), "lead");
    assert_eq!(listed(&screen).len(), 2);
    press(&mut screen, KeyCode::Enter, &ctx);
    assert!(screen.patch_filter.is_active());

    press(&mut screen, KeyCode::Backspace, &ctx);
    assert!(!screen.patch_filter.is_invalid());
    press(&mut screen, KeyCode::Enter, &ctx);
    assert!(!screen.patch_filter.is_active());
}

#[test]
fn while_typing_a_note_press_is_text_and_a_release_is_note_off() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Leads/Lead 1.fxp", &ctx);
    assert!(screen.state.press(KEYBOARD_NOTES[0]).is_some());
    press(&mut screen, KeyCode::Char('/'), &ctx);

    press(&mut screen, KeyCode::Char('d'), &ctx);
    assert_eq!(screen.patch_filter.value(), "d");
    assert_eq!(screen.state.held().len(), 1);

    screen.handle_key(
        KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ),
        &ctx,
    );
    assert!(screen.state.held().is_empty());
    assert!(screen.patch_filter.is_active());
}

#[test]
fn random_draws_only_from_the_filtered_list() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Pads/Pad 01.fxp", &ctx);
    press(&mut screen, KeyCode::Char('/'), &ctx);
    type_text(&mut screen, "pad 1", &ctx);
    press(&mut screen, KeyCode::Enter, &ctx);
    let allowed: HashSet<String> = listed(&screen).into_iter().collect();
    assert_eq!(allowed.len(), 3, "{allowed:?}");

    for _ in 0..20 {
        press(&mut screen, KeyCode::Char('r'), &ctx);
        let patch = screen.state.patch().unwrap().to_string();
        assert!(allowed.contains(&patch), "{patch}");
    }
}

#[test]
fn the_condition_survives_reentering_the_screen() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen("Leads/Lead 1.fxp", &ctx);
    press(&mut screen, KeyCode::Char('/'), &ctx);
    type_text(&mut screen, "lead", &ctx);
    press(&mut screen, KeyCode::Enter, &ctx);

    screen.start(Some("Leads/Lead 2.fxp".to_string()), &ctx);
    screen.sync_patch_catalog(&ctx);

    assert_eq!(screen.state.patch_catalog.filter(), "lead");
    assert_eq!(listed(&screen).len(), 2);
    assert_eq!(
        screen.state.patch_catalog.selected_patch(),
        Some("Leads/Lead 2.fxp")
    );
}
