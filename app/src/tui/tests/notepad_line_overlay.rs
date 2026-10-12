//! notepad の `i` / `o` / `O` が共有 MML overlay で行を編集する配線。

use super::*;
use crate::tui::mml_overlay::{MmlOverlayInputMode, SingleLineFlow};
use cmrt_history::test_support::temp_local_dirs;
use cmrt_notepad::NormalAction;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
}

/// notepad へキーを渡し、`EditLine` なら runtime と同じく overlay を開く。
fn press_notepad(app: &mut TuiApp<'_>, key: KeyEvent) {
    if let NormalAction::EditLine = app.notepad.handle_key_event(key) {
        app.open_notepad_line_overlay();
    }
}

fn notepad_with_lines(lines: &[&str], cursor: usize) -> TuiApp<'static> {
    let mut app = TuiApp::new_for_test(test_config());
    app.notepad
        .set_session_lines_for_test(lines.iter().map(|line| line.to_string()).collect());
    app.notepad.set_session_cursor_for_test(cursor);
    app
}

/// 行頭 JSON の書き方（空白・key の順・他の key）や `;` の多声、読めない JSON まで含めた行。
const ROUND_TRIP_LINES: &[&str] = &[
    r#"{"Surge XT patch":"Pads/Pad 1.fxp"}  l8cdef"#,
    r#"{ "Surge XT patch filter" : "pad",  "Surge XT patch": "Pads/Pad 1.fxp" } o5 c4"#,
    r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"plugin":"Reverb"}]} cde"#,
    r#"{"Surge XT patch":"Pads/A.fxp"} c;{"Surge XT patch":"Pads/B.fxp"} e"#,
    "l8cdef",
    "  o4 c  ",
    r#"{"Surge XT patch": cde"#,
    "",
];

fn json_value(line: &str) -> serde_json::Value {
    let json = mmlabc_to_smf::mml_preprocessor::extract_embedded_json(line)
        .embedded_json
        .expect("行頭 JSON があるはず");
    serde_json::from_str(&json).unwrap()
}

#[test]
fn i_then_esc_without_edits_keeps_the_line_byte_identical() {
    let (_tmp, _env_guard) = temp_local_dirs("notepad_line_overlay_i_esc");
    for &line in ROUND_TRIP_LINES {
        let before = vec!["above".to_string(), line.to_string(), "below".to_string()];
        let mut app = notepad_with_lines(&["above", line, "below"], 1);

        press_notepad(&mut app, plain(KeyCode::Char('i')));
        assert!(app.mml_overlay.is_open(), "{line:?}");
        app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

        assert!(!app.mml_overlay.is_open(), "{line:?}");
        assert_eq!(app.mml_overlay_owner, None, "{line:?}");
        assert_eq!(app.notepad.session_lines(), before.as_slice(), "{line:?}");
    }
}

#[test]
fn i_then_enter_without_edits_keeps_the_line_and_adds_one_blank_line_below() {
    let (_tmp, _env_guard) = temp_local_dirs("notepad_line_overlay_i_enter");
    for &line in ROUND_TRIP_LINES {
        let mut app = notepad_with_lines(&[line], 0);

        press_notepad(&mut app, plain(KeyCode::Char('i')));
        app.handle_mml_overlay_key_event(plain(KeyCode::Enter));

        // 開いたまま、足した空行の編集へ移る。
        assert!(app.mml_overlay.is_open(), "{line:?}");
        assert_eq!(app.mml_overlay.value(), "", "{line:?}");
        app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

        assert!(!app.mml_overlay.is_open(), "{line:?}");
        assert_eq!(
            app.notepad.session_lines(),
            [line.to_string(), String::new()].as_slice(),
            "{line:?}"
        );
    }
}

/// `o` / `O` は入る時点で空行を足す。何も打たずに抜けても、その空行は残る。
#[test]
fn o_and_shift_o_then_esc_keep_other_lines_and_the_inserted_blank_line() {
    let (_tmp, _env_guard) = temp_local_dirs("notepad_line_overlay_o_esc");
    let first = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}  l8cdef"#;
    for (key, expected) in [('o', [first, "", "cde"]), ('O', ["", first, "cde"])] {
        let mut app = notepad_with_lines(&[first, "cde"], 0);

        press_notepad(&mut app, plain(KeyCode::Char(key)));
        assert!(app.mml_overlay.is_open());
        app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

        assert_eq!(
            app.notepad.session_lines(),
            expected.map(str::to_string).as_slice(),
            "{key}"
        );
    }
}

#[test]
fn i_opens_the_line_body_with_the_line_patch_in_advance_mode() {
    let line = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}  l8cdef"#;
    let mut app = notepad_with_lines(&[line], 0);
    app.mml_overlay_patch = Some("Global/Pad.fxp".to_string());

    press_notepad(&mut app, plain(KeyCode::Char('i')));

    assert_eq!(app.mml_overlay_owner, Some(MmlOverlayOwner::Notepad));
    assert_eq!(
        app.mml_overlay.input_mode(),
        MmlOverlayInputMode::SingleLine
    );
    assert_eq!(app.mml_overlay.single_line_flow(), SingleLineFlow::Advance);
    let value = app.mml_overlay.value();
    assert!(
        !value.contains('{') && value.trim() == "l8cdef",
        "{value:?}"
    );
    assert_eq!(app.mml_overlay.patch(), Some("Pads/Pad 1.fxp"));

    app.handle_mml_overlay_key_event(plain(KeyCode::Esc));
    // notepad の音色は行頭 JSON が正本。Global の音色へは書かない。
    assert_eq!(app.mml_overlay_patch.as_deref(), Some("Global/Pad.fxp"));
}

#[test]
fn typed_mml_is_written_after_the_original_json_prefix() {
    let (_tmp, _env_guard) = temp_local_dirs("notepad_line_overlay_typed");
    let prefix = r#"{ "Surge XT patch" : "Pads/Pad 1.fxp" }  "#;
    let mut app = notepad_with_lines(&[&format!("{prefix}l8cdef")], 0);

    press_notepad(&mut app, plain(KeyCode::Char('i')));
    for code in [KeyCode::Char(' '), KeyCode::Char('g')] {
        app.handle_mml_overlay_key_event(plain(code));
    }
    app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

    assert_eq!(
        app.notepad.session_lines(),
        [format!("{prefix}l8cdef g")].as_slice()
    );
}

#[test]
fn ctrl_t_patch_is_written_into_the_json_at_commit_keeping_filter() {
    let (_tmp, _env_guard) = temp_local_dirs("notepad_line_overlay_patch");
    let mut app = notepad_with_lines(
        &[r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pad"} l8cdef"#],
        0,
    );
    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Pads/New Pad.fxp"]));

    press_notepad(&mut app, plain(KeyCode::Char('i')));
    app.handle_mml_overlay_key_event(ctrl('t'));
    app.handle_mml_overlay_key_event(plain(KeyCode::Enter));
    assert_eq!(app.mml_overlay.patch(), Some("Pads/New Pad.fxp"));
    app.handle_mml_overlay_key_event(plain(KeyCode::Esc));

    let line = &app.notepad.session_lines()[0];
    let json = json_value(line);
    assert_eq!(json["Surge XT patch"], "Pads/New Pad.fxp");
    assert_eq!(json["Surge XT patch filter"], "pad");
    assert!(line.ends_with(" l8cdef"), "{line:?}");
}
