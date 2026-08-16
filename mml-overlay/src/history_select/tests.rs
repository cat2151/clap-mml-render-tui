use super::*;

const PAD_JSON: &str = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn opened() -> HistorySelect<'static> {
    HistorySelect::open(
        vec!["cdefg".to_string(), format!("{PAD_JSON} gfedc")],
        vec!["'ceg'".to_string()],
    )
    .unwrap()
}

fn picked(action: HistorySelectAction) -> HistoryPick {
    match action {
        HistorySelectAction::Preview(pick) | HistorySelectAction::Confirm(pick) => pick,
        _ => panic!("expected a pick"),
    }
}

#[test]
fn nothing_to_show_means_nothing_to_open() {
    assert!(HistorySelect::open(Vec::new(), Vec::new()).is_none());
}

#[test]
fn an_empty_history_starts_on_the_favorites() {
    let select = HistorySelect::open(Vec::new(), vec!["cde".to_string()]).unwrap();

    assert_eq!(select.focus(), HistoryPane::Favorites);
}

/// 履歴の行は notepad が書いた形なので、音色と MML 本体へ分けて渡す。
#[test]
fn a_pick_splits_the_patch_off_the_phrase() {
    let mut select = opened();

    let pick = picked(select.handle_key(press(KeyCode::Down)));

    assert_eq!(pick.mml, "gfedc");
    assert_eq!(pick.patch.as_deref(), Some("Pads/Pad 1.fxp"));
}

#[test]
fn a_phrase_without_a_patch_has_none() {
    let mut select = opened();

    let pick = picked(select.handle_key(press(KeyCode::Enter)));

    assert_eq!(pick.mml, "cdefg");
    assert_eq!(pick.patch, None);
}

#[test]
fn tab_switches_panes_and_previews_the_selection_there() {
    let mut select = opened();

    let pick = picked(select.handle_key(press(KeyCode::Tab)));

    assert_eq!(select.focus(), HistoryPane::Favorites);
    assert_eq!(pick.mml, "'ceg'");
}

#[test]
fn the_cursor_stops_at_the_end_of_the_list() {
    let mut select = opened();
    select.handle_key(press(KeyCode::Down));

    assert!(matches!(
        select.handle_key(press(KeyCode::Down)),
        HistorySelectAction::Continue
    ));
    assert_eq!(select.cursor(HistoryPane::History), 1);
}

#[test]
fn filtering_narrows_both_panes() {
    let mut select = opened();

    select.handle_key(press(KeyCode::Char('g')));
    select.handle_key(press(KeyCode::Char('f')));

    assert_eq!(select.items(HistoryPane::History).len(), 1);
    assert!(select.items(HistoryPane::Favorites).is_empty());
    assert_eq!(select.total(HistoryPane::History), 2);
}

/// 絞り込んでも選択が残っていれば、そこへ留まる（試聴が鳴り直さない）。
#[test]
fn filtering_keeps_the_selection_when_it_survives() {
    let mut select = opened();

    assert!(matches!(
        select.handle_key(press(KeyCode::Char('c'))),
        HistorySelectAction::Continue
    ));
    assert_eq!(select.cursor(HistoryPane::History), 0);
}

#[test]
fn esc_cancels() {
    let mut select = opened();

    assert!(matches!(
        select.handle_key(press(KeyCode::Esc)),
        HistorySelectAction::Cancel
    ));
}

/// 試聴していなければ、取り消しで音色を戻す必要もない。
#[test]
fn previewed_tracks_whether_anything_was_heard() {
    let mut select = opened();
    assert!(!select.previewed());

    select.handle_key(press(KeyCode::Down));

    assert!(select.previewed());
}

#[test]
fn ctrl_o_is_the_trigger() {
    assert!(is_history_select_trigger(KeyEvent::new(
        KeyCode::Char('o'),
        KeyModifiers::CONTROL
    )));
    assert!(!is_history_select_trigger(press(KeyCode::Char('o'))));
}
