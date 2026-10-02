use super::share_command;
use crate::session_state::{KeyboardSessionState, NotePlaybackMode};
use serde_json::json;

const SAVED_EXAMPLE: &str = r#"{"patch":"patches_3rdparty/John Valentine/Plucks/Guitarp.fxp","buffer_multiplier":4,"note_playback_mode":"auto","repeat_chords":[[60,65,67]],"mml":"","effect_chain":[{"Surge XT Effects preset":"Delay/Rhythmic 1.srgfx"}]}"#;

fn state(mode: NotePlaybackMode, chords: Vec<Vec<u8>>, mml: &str) -> KeyboardSessionState {
    KeyboardSessionState {
        note_playback_mode: mode,
        repeat_chords: chords,
        mml: mml.to_string(),
        ..KeyboardSessionState::default()
    }
}

#[test]
fn saved_example_becomes_shortest_command() {
    let saved: KeyboardSessionState = serde_json::from_str(SAVED_EXAMPLE).unwrap();
    assert_eq!(
        share_command(&saved),
        r#"cmrt kb -p "patches_3rdparty/John Valentine/Plucks/Guitarp.fxp" -t auto -e "Surge XT Effects preset=Delay/Rhythmic 1.srgfx""#
    );
}

#[test]
fn saved_example_with_non_default_chord_writes_notes() {
    let mut saved: KeyboardSessionState = serde_json::from_str(SAVED_EXAMPLE).unwrap();
    saved.repeat_chords = vec![vec![60, 64, 67]];
    assert_eq!(
        share_command(&saved),
        r#"cmrt kb -p "patches_3rdparty/John Valentine/Plucks/Guitarp.fxp" -t auto -n 60,64,67 -e "Surge XT Effects preset=Delay/Rhythmic 1.srgfx""#
    );
}

#[test]
fn default_state_is_bare_command_whatever_the_buffer_multiplier() {
    assert_eq!(share_command(&KeyboardSessionState::default()), "cmrt kb");
    let state = KeyboardSessionState {
        buffer_multiplier: 8,
        ..KeyboardSessionState::default()
    };
    assert_eq!(share_command(&state), "cmrt kb");
}

#[test]
fn mml_matching_chords_is_written_as_mml() {
    let state = state(NotePlaybackMode::Repeat, vec![vec![60, 64, 67]], "'ceg'");
    assert_eq!(share_command(&state), r#"cmrt kb -t repeat -m "'ceg'""#);
}

#[test]
fn mml_not_matching_chords_falls_back_to_notes() {
    let state = state(
        NotePlaybackMode::Arp,
        vec![vec![62, 67, 71], vec![60]],
        "'ceg'",
    );
    assert_eq!(share_command(&state), "cmrt kb -t arp -n 62,67,71/60");
}

#[test]
fn bare_mml_needs_no_quotes() {
    let state = state(NotePlaybackMode::Repeat, vec![vec![60, 64, 67]], "C");
    assert_eq!(share_command(&state), "cmrt kb -t repeat -m C");
}

#[test]
fn chords_equal_to_mode_default_are_omitted() {
    for (mode, chords, expected) in [
        (
            NotePlaybackMode::Auto,
            vec![vec![60, 65, 67]],
            "cmrt kb -t auto",
        ),
        (
            NotePlaybackMode::Arp,
            vec![vec![60, 65, 67]],
            "cmrt kb -t arp",
        ),
        (
            NotePlaybackMode::Repeat,
            vec![vec![60]],
            "cmrt kb -t repeat",
        ),
        (NotePlaybackMode::Off, vec![], "cmrt kb"),
    ] {
        assert_eq!(share_command(&state(mode, chords, "")), expected);
    }
}

#[test]
fn off_mode_still_writes_played_chord() {
    let state = state(NotePlaybackMode::Off, vec![vec![60, 65, 67]], "");
    assert_eq!(share_command(&state), "cmrt kb -n 60,65,67");
}

#[test]
fn bypassed_stages_are_dropped_and_order_is_kept() {
    let state = KeyboardSessionState {
        effect_chain: vec![
            json!({"A preset": "x"}),
            cmrt_effect_chain_select::stage_with_bypass(&json!({"B preset": "y"}), true),
            json!({"C preset": "z"}),
        ],
        ..KeyboardSessionState::default()
    };
    assert_eq!(
        share_command(&state),
        r#"cmrt kb -e "A preset=x" -e "C preset=z""#
    );
}

#[test]
fn values_with_only_safe_characters_stay_bare() {
    let state = KeyboardSessionState {
        patch: Some("Surge/Leads/Saw+Sub_1.fxp".to_string()),
        effect_chain: vec![json!({"TONE3000": "a:b-c"})],
        ..KeyboardSessionState::default()
    };
    assert_eq!(
        share_command(&state),
        "cmrt kb -p Surge/Leads/Saw+Sub_1.fxp -e TONE3000=a:b-c"
    );
}
