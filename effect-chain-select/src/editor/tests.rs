use super::*;
use serde_json::json;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn two_stages() -> EffectChainEditor {
    EffectChainEditor::open(vec![json!({"A preset": "1"}), json!({"B preset": "2"})])
}

#[test]
fn cursor_moves_do_not_ask_for_a_preview() {
    let mut editor = two_stages();

    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('j'))),
        ChainKeyAction::None
    );
    assert_eq!(editor.cursor, 1);
}

#[test]
fn dd_deletes_the_cursor_stage_and_asks_for_a_preview() {
    let mut editor = two_stages();

    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('d'))),
        ChainKeyAction::None
    );
    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('d'))),
        ChainKeyAction::Preview
    );
    assert_eq!(editor.chain, vec![json!({"B preset": "2"})]);
}

#[test]
fn another_key_between_the_two_ds_cancels_the_delete() {
    let mut editor = two_stages();

    editor.handle_chain_key(key(KeyCode::Char('d')));
    editor.handle_chain_key(key(KeyCode::Char('j')));
    editor.handle_chain_key(key(KeyCode::Char('d')));

    assert_eq!(editor.chain.len(), 2);
}

#[test]
fn b_toggles_the_bypass_and_asks_for_a_preview() {
    let mut editor = two_stages();

    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('b'))),
        ChainKeyAction::Preview
    );
    assert!(stage_is_bypassed(&editor.chain[0]));
}

#[test]
fn alt_down_swaps_the_stage_and_the_edge_does_nothing() {
    let mut editor = two_stages();
    let alt_down = KeyEvent::new(KeyCode::Down, KeyModifiers::ALT);

    assert_eq!(editor.handle_chain_key(alt_down), ChainKeyAction::Preview);
    assert_eq!(editor.chain[1], json!({"A preset": "1"}));
    assert_eq!(editor.handle_chain_key(alt_down), ChainKeyAction::None);
}

#[test]
fn a_opens_the_add_overlay_and_r_replaces_the_cursor_stage() {
    let mut editor = two_stages();
    editor.cursor = 1;

    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('a'))),
        ChainKeyAction::OpenAdd {
            replace_target: None
        }
    );
    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('r'))),
        ChainKeyAction::OpenAdd {
            replace_target: Some(1)
        }
    );
}

#[test]
fn r_on_an_empty_chain_does_nothing() {
    let mut editor = EffectChainEditor::default();

    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Char('r'))),
        ChainKeyAction::None
    );
}

#[test]
fn enter_and_esc_ask_to_commit_and_to_close() {
    let mut editor = two_stages();

    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Enter)),
        ChainKeyAction::Commit
    );
    assert_eq!(
        editor.handle_chain_key(key(KeyCode::Esc)),
        ChainKeyAction::Close
    );
}

#[test]
fn the_candidate_chain_appends_or_replaces_the_list_cursor_preset() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = two_stages();
    editor.open_add(&catalog, None);
    let first = editor.add.preset_stage(Some(&catalog), 0).unwrap();

    let appended = editor.candidate_chain(Some(&catalog), 0, false).unwrap();
    assert_eq!(appended.len(), 3);
    assert_eq!(appended[2], first);

    editor.open_add(&catalog, Some(0));
    let replaced = editor.candidate_chain(Some(&catalog), 0, true).unwrap();
    assert_eq!(replaced.len(), 2);
    assert_eq!(replaced[0], stage_with_bypass(&first, true));
}
