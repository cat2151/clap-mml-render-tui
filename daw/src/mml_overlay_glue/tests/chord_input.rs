use std::time::Instant;

use cmrt_mml_overlay::{MmlOverlayAction, MmlOverlaySyntax};
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::KeyCode;

use super::super::super::{DawMode, CHORD_TRACK};
use super::{ctrl, key, plain};
use crate::input::tests::build_test_app;

const FIRST_GENERATED_TRACK: usize = 2;
const SECOND_GENERATED_TRACK: usize = 3;
const PAD_PATCH: &str = "Pads/Snapshot Pad.fxp";
const BASS_PATCH: &str = "Bass/Snapshot Bass.fxp";

fn generated_init(patch: &str, directive: &str) -> String {
    format!(r#"{{"Surge XT patch":"{patch}","generate from chord track":"{directive}"}}"#)
}

fn open_chord_overlay(
    return_track: Option<usize>,
) -> (crate::DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    app.editor.data[CHORD_TRACK][0] = "key:G".to_string();
    app.editor.data[CHORD_TRACK][1].clear();
    app.editor.data[FIRST_GENERATED_TRACK][0] = generated_init(PAD_PATCH, "close");
    app.editor.cursor_track = CHORD_TRACK;
    app.editor.cursor_measure = 1;
    app.editor.chord_jump_return_track = return_track;
    assert!(app.open_mml_overlay());
    (app, cache_rx)
}

fn sent_pitches(action: MmlOverlayAction) -> Vec<u8> {
    let MmlOverlayAction::Send(notes) = action else {
        panic!("expected note preview, got {action:?}");
    };
    notes
        .messages
        .iter()
        .filter(|message| message[0] == 0x90)
        .map(|message| message[1])
        .collect()
}

#[test]
fn chord_input_borrows_the_return_tracks_patch_and_generation_context() {
    let (app, _cache_rx) = open_chord_overlay(Some(FIRST_GENERATED_TRACK));

    assert_eq!(app.mml_overlay.patch(), Some(PAD_PATCH));
    assert!(matches!(
        app.mml_overlay.syntax(),
        MmlOverlaySyntax::Chord(Some(context))
            if context.chord_init == "key:G"
                && context.track_directive == "close"
                && context.target_label == "T1"
    ));
}

#[test]
fn typing_ii_previews_it_in_the_borrowed_tracks_key() {
    let (mut app, _cache_rx) = open_chord_overlay(Some(FIRST_GENERATED_TRACK));

    app.mml_overlay.handle_key(plain('I'), Instant::now());
    let action = app.mml_overlay.handle_key(plain('I'), Instant::now());

    assert_eq!(sent_pitches(action), vec![69, 73, 76]);
}

#[test]
fn preview_also_uses_the_borrowed_tracks_non_json_init_mml() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[CHORD_TRACK][0] = "key:G".to_string();
    app.editor.data[FIRST_GENERATED_TRACK][0] = format!("{}o4", generated_init(PAD_PATCH, "close"));
    app.editor.cursor_track = CHORD_TRACK;
    app.editor.cursor_measure = 1;
    app.editor.chord_jump_return_track = Some(FIRST_GENERATED_TRACK);
    assert!(app.open_mml_overlay());

    app.mml_overlay.handle_key(plain('I'), Instant::now());
    let action = app.mml_overlay.handle_key(plain('I'), Instant::now());

    assert_eq!(sent_pitches(action), vec![57, 61, 64]);
    assert!(matches!(
        app.mml_overlay.syntax(),
        MmlOverlaySyntax::Chord(Some(context)) if context.mml_prefix == "o4"
    ));
}

#[test]
fn enter_commits_to_the_chord_row_and_continues_with_the_next_measure() {
    let (mut app, _cache_rx) = open_chord_overlay(Some(FIRST_GENERATED_TRACK));

    app.handle_mml_overlay_key_event(plain('I'));
    app.handle_mml_overlay_key_event(plain('I'));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[CHORD_TRACK][1], "II");
    assert_eq!(app.editor.cursor_measure, 2);
    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(matches!(
        app.mml_overlay.syntax(),
        MmlOverlaySyntax::Chord(Some(_))
    ));
}

#[test]
fn ctrl_t_updates_the_borrowed_track_without_touching_chord_init_or_directive() {
    let (mut app, _cache_rx) = open_chord_overlay(Some(FIRST_GENERATED_TRACK));
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(
        [PAD_PATCH, BASS_PATCH]
            .into_iter()
            .map(|patch| (patch.to_string(), patch.to_lowercase()))
            .collect(),
    );

    app.handle_mml_overlay_key_event(ctrl('t'));
    for ch in "snapshot bass".chars() {
        app.handle_mml_overlay_key_event(plain(ch));
    }
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(
        app.track_patch_name(FIRST_GENERATED_TRACK).as_deref(),
        Some(BASS_PATCH)
    );
    assert_eq!(
        crate::mml::init_cell_chord_directive(&app.editor.data[FIRST_GENERATED_TRACK][0])
            .as_deref(),
        Some("close")
    );
    assert_eq!(app.editor.data[CHORD_TRACK][0], "key:G");
}

#[test]
fn the_return_track_wins_when_several_tracks_generate_from_chords() {
    let (mut app, _cache_rx) = open_chord_overlay(None);
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));
    app.editor.data[SECOND_GENERATED_TRACK][0] = generated_init(BASS_PATCH, "drop2");
    app.editor.chord_jump_return_track = Some(SECOND_GENERATED_TRACK);

    assert!(app.open_mml_overlay());

    assert_eq!(app.mml_overlay.patch(), Some(BASS_PATCH));
    assert!(matches!(
        app.mml_overlay.syntax(),
        MmlOverlaySyntax::Chord(Some(context))
            if context.track_directive == "drop2" && context.target_label == "T2"
    ));
}

#[test]
fn direct_navigation_falls_back_to_the_first_generated_track() {
    let (app, _cache_rx) = open_chord_overlay(None);

    assert!(matches!(
        app.mml_overlay.syntax(),
        MmlOverlaySyntax::Chord(Some(context)) if context.target_label == "T1"
    ));
}

#[test]
fn no_generated_track_keeps_chord_editing_available_and_rejects_ctrl_t() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = CHORD_TRACK;
    app.editor.cursor_measure = 1;
    assert!(app.open_mml_overlay());

    app.handle_mml_overlay_key_event(ctrl('t'));

    assert!(matches!(
        app.mml_overlay.syntax(),
        MmlOverlaySyntax::Chord(None)
    ));
    assert!(!app.mml_overlay.is_patch_select_open());
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line.contains("演奏 track がありません")));
}
