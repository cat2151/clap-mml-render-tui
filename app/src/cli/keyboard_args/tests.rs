use crate::cli::{parse_cli_from, CliAction};
use cmrt_tui_core::keyboard_session_state::{
    KeyboardControllerState, KeyboardSessionState, NotePlaybackMode,
};

fn parse_kb(args: &[&str]) -> anyhow::Result<KeyboardSessionState> {
    let mut argv = vec!["cmrt", "kb"];
    argv.extend_from_slice(args);
    match parse_cli_from(argv)? {
        CliAction::Keyboard(state) => Ok(state),
        other => panic!("kb should return Keyboard action: {other:?}"),
    }
}

#[test]
fn shared_command_example_matches_saved_session_example() {
    let parsed = parse_kb(&[
        "-p",
        "patches_3rdparty/John Valentine/Plucks/Guitarp.fxp",
        "-t",
        "auto",
        "-n",
        "60,65,67",
        "-e",
        "Surge XT Effects preset=Delay/Rhythmic 1.srgfx",
    ])
    .unwrap();
    let saved: KeyboardSessionState = serde_json::from_str(
        r#"{"patch":"patches_3rdparty/John Valentine/Plucks/Guitarp.fxp","buffer_multiplier":4,"note_playback_mode":"auto","repeat_chords":[[60,65,67]],"mml":"","effect_chain":[{"Surge XT Effects preset":"Delay/Rhythmic 1.srgfx"}]}"#,
    )
    .unwrap();
    assert_eq!(parsed, saved);
}

#[test]
fn kb_without_arguments_is_default_state() {
    assert_eq!(parse_kb(&[]).unwrap(), KeyboardSessionState::default());
}

#[test]
fn mml_fills_both_mml_and_repeat_chords() {
    let parsed = parse_kb(&["-t", "repeat", "-m", "'ceg'"]).unwrap();
    assert_eq!(parsed.note_playback_mode, NotePlaybackMode::Repeat);
    assert_eq!(parsed.mml, "'ceg'");
    assert_eq!(parsed.repeat_chords, vec![vec![60, 64, 67]]);
}

#[test]
fn notes_split_chords_by_slash_and_notes_by_comma() {
    let parsed = parse_kb(&["-n", "60,65,67/62,67,71"]).unwrap();
    assert_eq!(
        parsed.repeat_chords,
        vec![vec![60, 65, 67], vec![62, 67, 71]]
    );
    assert_eq!(parsed.mml, "");
}

#[test]
fn effects_keep_their_order_and_split_at_first_equals() {
    let parsed = parse_kb(&["-e", "A preset=x=y", "-e", "B preset=z"]).unwrap();
    assert_eq!(
        parsed.effect_chain,
        vec![
            serde_json::json!({"A preset": "x=y"}),
            serde_json::json!({"B preset": "z"}),
        ]
    );
}

#[test]
fn invalid_arguments_are_errors() {
    for args in [
        &["-m", "@@@"][..],
        &["-m", ""][..],
        &["-e", "no equals"][..],
        &["-n", "128"][..],
        &["-n", "-1"][..],
        &["-n", "60,,67"][..],
        &["-n", "c"][..],
        &["-t", "loop"][..],
        &["-m", "c", "-n", "60"][..],
    ] {
        assert!(parse_kb(args).is_err(), "{args:?} should be rejected");
    }
}

/// shell と同じく空白で区切り、`"..."` の中の空白は区切らずに囲みだけを外す。
fn split_like_shell(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut has_arg = false;
    for c in command.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                has_arg = true;
            }
            ' ' if !quoted => {
                if has_arg {
                    args.push(std::mem::take(&mut current));
                    has_arg = false;
                }
            }
            _ => {
                current.push(c);
                has_arg = true;
            }
        }
    }
    if has_arg {
        args.push(current);
    }
    args
}

#[test]
fn share_command_round_trips_through_kb_parse() {
    let bypassed = cmrt_effect_chain_select::stage_with_bypass(
        &serde_json::json!({"Dragonfly Hall Reverb preset": "Large Hall.fxp"}),
        true,
    );
    let effect = serde_json::json!({"Surge XT Effects preset": "Delay/Rhythmic 1.srgfx"});
    let cases = [
        (
            KeyboardSessionState {
                patch: Some("patches_3rdparty/John Valentine/Plucks/Guitarp.fxp".to_string()),
                buffer_multiplier: 8,
                note_playback_mode: NotePlaybackMode::Auto,
                repeat_chords: vec![vec![60, 64, 67], vec![62, 65, 69]],
                mml: String::new(),
                effect_chain: vec![effect.clone(), bypassed.clone()],
                patch_filter: String::new(),
                controllers: KeyboardControllerState::default(),
            },
            None,
        ),
        (
            KeyboardSessionState {
                note_playback_mode: NotePlaybackMode::Repeat,
                repeat_chords: vec![vec![60, 64, 67]],
                mml: "'ceg'".to_string(),
                effect_chain: vec![bypassed, effect.clone()],
                ..KeyboardSessionState::default()
            },
            None,
        ),
        (
            KeyboardSessionState {
                note_playback_mode: NotePlaybackMode::Off,
                repeat_chords: vec![vec![48]],
                ..KeyboardSessionState::default()
            },
            None,
        ),
        // mode の既定の音は書かれず、parse 側では空（復元時に同じ既定へ戻る）になる。
        (
            KeyboardSessionState {
                note_playback_mode: NotePlaybackMode::Arp,
                repeat_chords: vec![vec![60, 65, 67]],
                ..KeyboardSessionState::default()
            },
            Some(Vec::new()),
        ),
    ];
    for (original, expected_chords) in cases {
        let command = cmrt_keyboard::share_command(&original);
        let argv = split_like_shell(&command);
        assert_eq!(&argv[..2], ["cmrt", "kb"], "{command}");
        let args: Vec<&str> = argv[2..].iter().map(String::as_str).collect();
        let parsed = parse_kb(&args).unwrap_or_else(|e| panic!("{command}: {e}"));
        let expected = KeyboardSessionState {
            buffer_multiplier: KeyboardSessionState::default().buffer_multiplier,
            repeat_chords: expected_chords.unwrap_or_else(|| original.repeat_chords.clone()),
            effect_chain: original
                .effect_chain
                .iter()
                .filter(|stage| !cmrt_effect_chain_select::stage_is_bypassed(stage))
                .cloned()
                .collect(),
            ..original.clone()
        };
        assert_eq!(parsed, expected, "{command}");
    }
}
