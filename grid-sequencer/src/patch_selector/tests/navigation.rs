use super::*;

use cmrt_patches::PatchRole;
use cmrt_rhythm::DrumRole;

use crate::{ARPEGGIO_ROW, BASS_ROW};

fn role_label(selector: &PatchSelector) -> &'static str {
    selector.roles()[selector.role_cursor()].label()
}

fn preset_label(selector: &PatchSelector) -> &str {
    &selector.presets()[selector.preset_cursor()].label
}

/// chord mode ON・7 track。行 0〜2 が chord / bass / arpeggio、行 3〜6 が drum。
fn chord_mode_screen() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 7);
    screen.state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    screen
}

fn drum_row(screen: &GridSequencerScreen, role: DrumRole) -> usize {
    (0..screen.state.instance_count())
        .find(|row| screen.state.drum_role(*row) == Some(role))
        .expect("7 track には全 drum 行がある")
}

#[test]
fn focus_moves_between_the_three_panes_and_stops_at_both_ends() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Patches);

    screen.handle_patch_selector_key(press(KeyCode::Char('l')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('l')), &ctx);
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Patches);

    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Preset);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Role);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Role);

    screen.handle_patch_selector_key(press(KeyCode::Right), &ctx);
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Preset);
}

#[test]
fn moving_the_role_resets_the_preset_and_previews_the_first_patch_of_the_new_list() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Pads/Pad 01.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);

    let selector = selector(&screen);
    assert_eq!(role_label(selector), "Bass track");
    assert_eq!(preset_label(selector), "ALL");
    assert_eq!(
        filtered_patches(selector),
        ["Basses/Bass 01.fxp", "Basses/Bass 02.fxp"]
    );
    assert_eq!(selector.patch_cursor, 0);
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Basses/Bass 01.fxp")
    );
}

#[test]
fn moving_the_role_keeps_the_current_patch_when_the_new_list_still_has_it() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Basses/Bass 02.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);

    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);

    let selector = selector(&screen);
    assert_eq!(role_label(selector), "Bass track");
    assert_eq!(selector.selected_patch(), Some("Basses/Bass 02.fxp"));
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Basses/Bass 02.fxp")
    );
}

#[test]
fn moving_the_preset_narrows_the_list_and_the_role_change_resets_it_to_all() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    // Role=Drum tracks、Preset=snare。
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    for _ in 0..4 {
        screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    }
    assert_eq!(role_label(selector(&screen)), "Drum tracks");
    screen.handle_patch_selector_key(press(KeyCode::Char('l')), &ctx);
    let snare = selector(&screen)
        .presets()
        .iter()
        .position(|preset| preset.label == "snare")
        .unwrap();
    for _ in 0..snare {
        screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    }
    assert_eq!(filtered_patches(selector(&screen)), ["Drums/Snare 01.wav"]);

    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('k')), &ctx);

    let selector = selector(&screen);
    assert_eq!(role_label(selector), "Lead / melody");
    assert_eq!(preset_label(selector), "ALL");
    assert_eq!(filtered_patches(selector), ["Leads/Lead 01.fxp"]);
}

#[test]
fn r_draws_only_from_the_current_list_and_moves_focus_to_the_patches() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    assert_eq!(role_label(selector(&screen)), "Bass track");
    let bass = ctx.patch_roles.candidates(PatchRole::Bass);
    assert_eq!(filtered_patches(selector(&screen)).len(), bass.len());

    for _ in 0..20 {
        screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);
        let selector = selector(&screen);
        assert_eq!(selector.focus(), PatchPaneFocus::Patches);
        let selected = selector.selected_patch().unwrap();
        assert!(bass.iter().any(|patch| patch == selected), "{selected}");
    }
}

#[test]
fn r_does_nothing_when_the_list_has_fewer_than_two_patches() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].patch = Some("Leads/Lead 01.fxp".to_string());
    screen.open_patch_selector(0, &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    for _ in 0..3 {
        screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    }
    assert_eq!(filtered_patches(selector(&screen)), ["Leads/Lead 01.fxp"]);

    screen.handle_patch_selector_key(press(KeyCode::Char('r')), &ctx);

    let selector = selector(&screen);
    assert_eq!(selector.selected_patch(), Some("Leads/Lead 01.fxp"));
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Leads/Lead 01.fxp")
    );
}

#[test]
fn page_and_home_end_keys_move_the_focused_pane() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);

    screen.handle_patch_selector_key(press(KeyCode::End), &ctx);
    assert_eq!(selector(&screen).selected_patch(), Some("Pads/Pad 01.fxp"));
    screen.handle_patch_selector_key(press(KeyCode::Home), &ctx);
    assert_eq!(
        selector(&screen).selected_patch(),
        Some("Basses/Bass 01.fxp")
    );
    screen.handle_patch_selector_key(press(KeyCode::PageDown), &ctx);
    assert_eq!(selector(&screen).selected_patch(), Some("Pads/Pad 01.fxp"));

    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::Char('h')), &ctx);
    screen.handle_patch_selector_key(press(KeyCode::End), &ctx);
    assert_eq!(role_label(selector(&screen)), "Etc / unknown");
    screen.handle_patch_selector_key(press(KeyCode::PageUp), &ctx);
    assert_eq!(role_label(selector(&screen)), "ALL");
}

#[test]
fn the_bass_row_opens_in_the_bass_role_at_the_current_patch() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = chord_mode_screen();
    screen.state.instances_mut()[BASS_ROW].patch = Some("Basses/Bass 02.fxp".to_string());

    screen.open_patch_selector(BASS_ROW, &ctx);

    let selector = selector(&screen);
    assert_eq!(role_label(selector), "Bass track");
    assert_eq!(preset_label(selector), "ALL");
    assert_eq!(selector.selected_patch(), Some("Basses/Bass 02.fxp"));
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Basses/Bass 02.fxp")
    );
    assert_eq!(selector.focus(), PatchPaneFocus::Patches);
}

#[test]
fn the_chord_and_arpeggio_rows_open_in_their_roles() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = chord_mode_screen();

    screen.open_patch_selector(ARPEGGIO_ROW, &ctx);
    assert_eq!(role_label(selector(&screen)), "Lead / melody");
    assert_eq!(filtered_patches(selector(&screen)), ["Leads/Lead 01.fxp"]);
    screen.cancel_patch_selector();

    screen.open_patch_selector(CHORD_ROW, &ctx);
    assert_eq!(role_label(selector(&screen)), "Chord track");
    // 和音行は poly の音色だけ。Role / Preset / 音色 のどの pane にも mono は残らない。
    assert_eq!(filtered_patches(selector(&screen)), ["Pads/Pad 01.fxp"]);
    assert_eq!(selector(&screen).total(), 1);
}

#[test]
fn each_drum_row_opens_in_the_drum_role_at_its_own_builtin_preset() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = chord_mode_screen();
    let expectations = [
        (DrumRole::Kick, "kick|bass drum", "Drums/Kick 01.wav"),
        (DrumRole::Snare, "snare", "Drums/Snare 01.wav"),
        (DrumRole::HiHat, "hat", "Drums/Hi Hat 01.wav"),
        (DrumRole::Percussion, "perc", "Drums/Perc Clap 01.wav"),
    ];

    for (role, preset, patch) in expectations {
        screen.open_patch_selector(drum_row(&screen, role), &ctx);
        let selector = selector(&screen);
        assert_eq!(role_label(selector), "Drum tracks", "{role:?}");
        assert_eq!(preset_label(selector), preset, "{role:?}");
        assert_eq!(filtered_patches(selector), [patch], "{role:?}");
        screen.cancel_patch_selector();
    }
}

#[test]
fn a_plain_note_row_opens_in_the_all_role() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);

    screen.open_patch_selector(0, &ctx);

    let selector = selector(&screen);
    assert_eq!(role_label(selector), "ALL");
    assert_eq!(preset_label(selector), "ALL");
    assert_eq!(selector.filtered_len(), selector.total());
}

#[test]
fn clicking_a_role_or_preset_selects_it_and_moves_the_focus_there() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA, false);
    let drum = selector(&screen)
        .roles()
        .iter()
        .position(|role| role.label() == "Drum tracks")
        .unwrap();

    screen.handle_patch_selector_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.role_list.x,
            layout.role_list.y + drum as u16,
        ),
        AREA,
        &ctx,
    );
    assert_eq!(role_label(selector(&screen)), "Drum tracks");
    assert_eq!(selector(&screen).focus(), PatchPaneFocus::Role);
    assert_eq!(
        selector(&screen).previewed_patch.as_deref(),
        Some("Drums/Hi Hat 01.wav")
    );

    let kick = selector(&screen)
        .presets()
        .iter()
        .position(|preset| preset.label == "kick|bass drum")
        .unwrap();
    screen.handle_patch_selector_mouse(
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            layout.preset_list.x,
            layout.preset_list.y + kick as u16,
        ),
        AREA,
        &ctx,
    );
    let selector = selector(&screen);
    assert_eq!(preset_label(selector), "kick|bass drum");
    assert_eq!(selector.focus(), PatchPaneFocus::Preset);
    assert_eq!(filtered_patches(selector), ["Drums/Kick 01.wav"]);
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Drums/Kick 01.wav")
    );
}

#[test]
fn the_wheel_moves_the_pane_under_the_pointer_without_moving_the_focus() {
    let patches = role_patches();
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA, false);

    screen.handle_patch_selector_mouse(
        mouse(
            MouseEventKind::ScrollDown,
            layout.role_pane.x,
            layout.role_pane.y,
        ),
        AREA,
        &ctx,
    );

    let selector = selector(&screen);
    assert_eq!(role_label(selector), "Bass track");
    assert_eq!(selector.focus(), PatchPaneFocus::Patches);
    assert_eq!(
        selector.previewed_patch.as_deref(),
        Some("Basses/Bass 01.fxp")
    );
}

/// j/k のたびに一覧全体が動くと目で追えない。カーソルが viewport の下 30% に
/// 入るまでは scroll せず、入ってからは 1 行ずつ追う。
#[test]
fn the_patch_list_scrolls_only_when_the_cursor_enters_the_lower_margin() {
    let patches = PatchLoadState::ready(
        (0..40)
            .map(|index| {
                let patch = format!("Keys/Key {index:02}.fxp");
                (patch.clone(), patch.to_lowercase())
            })
            .collect(),
    );
    let ctx = context(&patches);
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.open_patch_selector(0, &ctx);
    let layout = PatchSelectorLayout::new(AREA, false);
    let height = usize::from(layout.patch_rows.height);
    let margin = height * 3 / 10;
    assert!(height >= 4, "viewport が余白を持てる高さ: {height}");

    let last_unscrolled = height - margin - 1;
    for _ in 0..last_unscrolled {
        screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    }
    assert_eq!(selector(&screen).patch_cursor, last_unscrolled);
    assert_eq!(selector(&screen).patch_range(&layout).start, 0);

    screen.handle_patch_selector_key(press(KeyCode::Char('j')), &ctx);
    assert_eq!(selector(&screen).patch_range(&layout).start, 1);

    // 戻る向きは上 30% に入るまで scroll しない。
    for _ in 0..(height - 2 * margin - 1) {
        screen.handle_patch_selector_key(press(KeyCode::Char('k')), &ctx);
    }
    assert_eq!(selector(&screen).patch_range(&layout).start, 1);
    screen.handle_patch_selector_key(press(KeyCode::Char('k')), &ctx);
    assert_eq!(selector(&screen).patch_range(&layout).start, 0);
}
