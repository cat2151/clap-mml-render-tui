use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{AddKeyAction, EffectAddPane, EffectChainEditor};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn opening_lists_every_selectable_preset_with_the_list_pane_focused() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();

    editor.open_add(&catalog, None);

    assert_eq!(editor.add.focus, EffectAddPane::List);
    assert_eq!(editor.add.list.len(), 2);
    assert_eq!(editor.add.categories[0], "all");
}

#[test]
fn moving_the_list_cursor_asks_for_a_preview_with_its_direction() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();
    editor.open_add(&catalog, None);

    assert_eq!(
        editor.handle_add_key(Some(&catalog), key(KeyCode::Char('j'))),
        AddKeyAction::Preview {
            preferred_delta: Some(1)
        }
    );
    // 末尾で止まり、候補が変わらないので鳴らし直させない。
    assert_eq!(
        editor.handle_add_key(Some(&catalog), key(KeyCode::Char('j'))),
        AddKeyAction::None
    );
}

#[test]
fn choosing_the_amp_kind_narrows_the_list_to_amps() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();
    editor.open_add(&catalog, None);
    editor.add.focus = EffectAddPane::Kinds;
    let amp_kind = editor
        .add
        .kinds
        .iter()
        .position(|kind| kind == "Amp Simulator")
        .unwrap();

    for _ in 0..amp_kind {
        editor.handle_add_key(Some(&catalog), key(KeyCode::Char('j')));
    }

    assert_eq!(editor.add.list.len(), 1);
    assert_eq!(catalog.presets()[editor.add.list[0]].name, "Clean");
}

#[test]
fn enter_appends_the_candidate_and_asks_to_go_back() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();
    editor.open_add(&catalog, None);

    assert_eq!(
        editor.handle_add_key(Some(&catalog), key(KeyCode::Enter)),
        AddKeyAction::Committed
    );
    assert_eq!(editor.chain.len(), 1);
}

#[test]
fn enter_on_an_empty_list_does_nothing() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();
    editor.open_add(&catalog, None);
    editor.add.list.clear();

    assert_eq!(
        editor.handle_add_key(Some(&catalog), key(KeyCode::Enter)),
        AddKeyAction::None
    );
    assert!(editor.chain.is_empty());
}

#[test]
fn typing_a_filter_that_changes_the_candidate_asks_for_a_preview() {
    let catalog = crate::test_catalog::catalog();
    let mut editor = EffectChainEditor::default();
    editor.open_add(&catalog, None);
    editor.handle_add_key(Some(&catalog), key(KeyCode::Char('/')));
    // `a` はどちらの候補にも当たるので候補は変わらず、`am` で plugin 名 `Test Amp` だけに絞れる。
    assert_eq!(
        editor.handle_add_key(Some(&catalog), key(KeyCode::Char('a'))),
        AddKeyAction::None
    );

    let action = editor.handle_add_key(Some(&catalog), key(KeyCode::Char('m')));

    assert_eq!(
        action,
        AddKeyAction::Preview {
            preferred_delta: None
        }
    );
    assert_eq!(editor.add.list.len(), 1);
}
