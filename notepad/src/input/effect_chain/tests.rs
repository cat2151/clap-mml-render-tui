use cmrt_core::{EffectPlugins, EFFECT_CHAIN_JSON_KEY};
use cmrt_patch_select::auto_reverb::AUTO_REVERB_JSON_KEY;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mmlabc_to_smf::mml_preprocessor;
use serde_json::{json, Value};

use crate::tests::{test_config, test_effect_plugins};
use crate::{Mode, NotepadScreen, PlayState, PATCH_JSON_KEY};

const PAD_LINE: &str =
    r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"} l8cdef"#;
const TWO_PART_LINE: &str =
    r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef;{"Surge XT patch":"Pads/Pad 1.fxp"} o3c1"#;

fn screen_on(line: &str) -> NotepadScreen<'static> {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.effect_plugins = test_effect_plugins();
    app.editor.lines = vec![line.to_string()];
    app
}

fn press(app: &mut NotepadScreen<'_>, code: KeyCode) {
    app.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

/// `;` で区切った各パートの (行頭 JSON, MML 本体)。
fn parts(line: &str) -> Vec<(Value, String)> {
    line.split(';')
        .map(|part| {
            let preprocessed = mml_preprocessor::extract_embedded_json(part.trim_start());
            let json = preprocessed
                .embedded_json
                .as_deref()
                .map(|json| serde_json::from_str(json).expect("JSON"))
                .expect("行頭 JSON");
            (json, preprocessed.remaining_mml.trim().to_string())
        })
        .collect()
}

/// 鳴らしている MML。
fn playing(app: &NotepadScreen<'_>) -> Option<String> {
    match &*app.playback.session.play_state().lock().unwrap() {
        PlayState::Running(mml) => Some(mml.clone()),
        _ => None,
    }
}

/// 出しているエラー文言。
fn error(app: &NotepadScreen<'_>) -> Option<String> {
    match &*app.playback.session.play_state().lock().unwrap() {
        PlayState::Err(message) => Some(message.clone()),
        _ => None,
    }
}

/// `x` → `a` で list カーソルの候補を入れた chain。`Enter` で chain へ入るはずのもの。
fn open_add_and_candidate(app: &mut NotepadScreen<'_>) -> Vec<Value> {
    press(app, KeyCode::Char('x'));
    assert!(matches!(app.mode, Mode::EffectChain));
    press(app, KeyCode::Char('a'));
    assert!(matches!(app.mode, Mode::EffectChainAdd));
    app.effect_chain
        .candidate_chain(
            app.effect_plugins.catalog(),
            app.effect_chain.add.list_cursor,
            false,
        )
        .expect("候補がある")
}

#[test]
fn add_and_commit_writes_the_chain_and_keeps_patch_filter_and_phrase() {
    let mut app = screen_on(PAD_LINE);
    let expected_chain = open_add_and_candidate(&mut app);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.mode, Mode::EffectChain));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.mode, Mode::Normal));

    let [(json, phrase)] = parts(&app.editor.lines[0]).try_into().unwrap();
    assert_eq!(json[EFFECT_CHAIN_JSON_KEY], Value::Array(expected_chain));
    assert_eq!(json[PATCH_JSON_KEY], "Pads/Pad 1.fxp");
    assert_eq!(json["Surge XT patch filter"], "pads");
    assert_eq!(phrase, "l8cdef");
    assert_eq!(playing(&app), Some(app.editor.lines[0].clone()));
}

#[test]
fn add_pane_previews_the_candidate_without_rewriting_the_line() {
    let mut app = screen_on(PAD_LINE);
    let candidate = open_add_and_candidate(&mut app);
    let preview = playing(&app).expect("候補を試聴している");
    let [(json, phrase)] = parts(&preview).try_into().unwrap();
    assert_eq!(json[EFFECT_CHAIN_JSON_KEY], Value::Array(candidate));
    assert_eq!(phrase, "l8cdef");
    assert_eq!(app.editor.lines[0], PAD_LINE);
}

#[test]
fn commit_writes_the_same_chain_into_every_part() {
    let mut app = screen_on(TWO_PART_LINE);
    let expected_chain = Value::Array(open_add_and_candidate(&mut app));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);

    let parts = parts(&app.editor.lines[0]);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].0[EFFECT_CHAIN_JSON_KEY], expected_chain);
    assert_eq!(parts[1].0[EFFECT_CHAIN_JSON_KEY], expected_chain);
    assert_eq!(parts[0].1, "l8cdef");
    assert_eq!(parts[1].1, "o3c1");
}

#[test]
fn esc_discards_the_edit() {
    let mut app = screen_on(PAD_LINE);
    open_add_and_candidate(&mut app);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(app.editor.lines[0], PAD_LINE);
}

#[test]
fn commit_keeps_the_auto_reverb_mark() {
    let hall = json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"});
    let line = format!(
        r#"{{"Surge XT patch":"Pads/Pad 1.fxp","{EFFECT_CHAIN_JSON_KEY}":[{hall}],"{AUTO_REVERB_JSON_KEY}":{hall}}} l8cdef"#
    );
    let mut app = screen_on(&line);
    let expected_chain = open_add_and_candidate(&mut app);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);

    let [(json, _)] = parts(&app.editor.lines[0]).try_into().unwrap();
    assert_eq!(json[EFFECT_CHAIN_JSON_KEY], Value::Array(expected_chain));
    assert_eq!(json[AUTO_REVERB_JSON_KEY], hall);
}

#[test]
fn deleting_every_stage_removes_the_chain_key() {
    let hall = json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"});
    let line = format!(
        r#"{{"Surge XT patch":"Pads/Pad 1.fxp","{EFFECT_CHAIN_JSON_KEY}":[{hall}]}} l8cdef"#
    );
    let mut app = screen_on(&line);
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.effect_chain.chain, vec![hall]);
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('d'));
    assert!(app.effect_chain.chain.is_empty());
    press(&mut app, KeyCode::Enter);

    let [(json, phrase)] = parts(&app.editor.lines[0]).try_into().unwrap();
    assert!(json.get(EFFECT_CHAIN_JSON_KEY).is_none(), "{json}");
    assert_eq!(json[PATCH_JSON_KEY], "Pads/Pad 1.fxp");
    assert_eq!(phrase, "l8cdef");
}

#[test]
fn line_without_patch_json_does_not_open() {
    let mut app = screen_on("l8cdef");
    press(&mut app, KeyCode::Char('x'));
    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(
        error(&app).as_deref(),
        Some("patch name JSON が見つかりません")
    );
}

#[test]
fn backend_without_effect_catalog_does_not_open() {
    let mut app = screen_on(PAD_LINE);
    app.effect_plugins = EffectPlugins::none();
    press(&mut app, KeyCode::Char('x'));
    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(
        error(&app).as_deref(),
        Some(cmrt_effect_chain_select::messages::NOT_AVAILABLE_ON_THIS_BACKEND)
    );
}

#[test]
fn add_filter_input_uses_the_textarea_cursor() {
    let mut app = screen_on(PAD_LINE);
    open_add_and_candidate(&mut app);
    assert!(!app.uses_textarea_cursor());
    press(&mut app, KeyCode::Char('/'));
    assert!(app.uses_textarea_cursor());
}

#[test]
fn help_returns_to_the_effect_chain_overlay() {
    let mut app = screen_on(PAD_LINE);
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('?'));
    assert!(matches!(app.mode, Mode::Help));
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.mode, Mode::EffectChain));
}
