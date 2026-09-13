use super::*;

#[test]
fn printable_keys_do_not_filter_until_slash_starts_editing() {
    let mut select = opened(None);

    select.handle_key(press(KeyCode::Char('p')));

    assert!(!select.filter_editing());
    assert_eq!(text_input::textarea_value(select.query_textarea()), "");
    assert_eq!(filtered(&select).len(), all().len());
}

#[test]
fn enter_commits_the_filter_then_the_next_enter_confirms_the_patch() {
    let mut select = opened(None);
    type_text(&mut select, "pad");

    assert!(select.filter_editing());
    assert!(matches!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Continue
    ));
    assert!(!select.filter_editing());
    assert_eq!(filtered(&select), ["Pads/Pad 1.fxp"]);
    assert!(matches!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Confirm(patch) if patch == "Pads/Pad 1.fxp"
    ));
}

#[test]
fn escape_while_editing_restores_the_previous_filter_and_keeps_the_selector_open() {
    let mut select = opened(None);
    type_text(&mut select, "lead");
    select.handle_key(press(KeyCode::Enter));

    select.handle_key(press(KeyCode::Char('/')));
    for _ in 0..4 {
        select.handle_key(press(KeyCode::Backspace));
    }
    type_text(&mut select, "pad");
    assert_eq!(filtered(&select), ["Pads/Pad 1.fxp"]);

    assert!(matches!(
        select.handle_key(press(KeyCode::Esc)),
        PatchSelectAction::Preview(patch) if patch == "Leads/Lead 1.fxp"
    ));
    assert!(!select.filter_editing());
    assert_eq!(text_input::textarea_value(select.query_textarea()), "lead");
    assert_eq!(filtered(&select), ["Leads/Lead 1.fxp", "Leads/Lead 2.fxp"]);
}

#[test]
fn removing_a_filter_requires_deleting_it_and_committing_the_empty_query() {
    let mut select = opened(None);
    type_text(&mut select, "lead");
    select.handle_key(press(KeyCode::Enter));
    select.handle_key(press(KeyCode::Char('/')));
    for _ in 0..4 {
        select.handle_key(press(KeyCode::Backspace));
    }

    select.handle_key(press(KeyCode::Enter));

    assert!(!select.filter_editing());
    assert_eq!(text_input::textarea_value(select.query_textarea()), "");
    assert_eq!(filtered(&select).len(), all().len());
}
