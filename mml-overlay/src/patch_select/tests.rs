use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn all() -> Vec<(String, String)> {
    [
        "Leads/Lead 1.fxp",
        "Leads/Lead 2.fxp",
        "Pads/Pad 1.fxp",
        "Basses/Bass 1.fxp",
    ]
    .into_iter()
    .map(|patch| (patch.to_string(), patch.to_lowercase()))
    .collect()
}

fn opened(current: Option<&str>) -> PatchSelect<'static> {
    PatchSelect::open(all(), current).expect("patch list is not empty")
}

fn previewed(action: PatchSelectAction) -> Option<String> {
    match action {
        PatchSelectAction::Preview(patch) => Some(patch),
        _ => None,
    }
}

#[test]
fn an_empty_patch_list_does_not_open() {
    assert!(PatchSelect::open(Vec::new(), None).is_none());
}

#[test]
fn it_starts_on_the_current_patch() {
    assert_eq!(
        opened(Some("Pads/Pad 1.fxp")).selected(),
        Some("Pads/Pad 1.fxp")
    );
    assert_eq!(opened(None).selected(), Some("Leads/Lead 1.fxp"));
    // 一覧に無い音色（設定を変えた後など）は先頭へ落とす。
    assert_eq!(
        opened(Some("gone.fxp")).selected(),
        Some("Leads/Lead 1.fxp")
    );
}

#[test]
fn moving_previews_the_newly_selected_patch() {
    let mut select = opened(None);

    assert_eq!(
        previewed(select.handle_key(press(KeyCode::Down))).as_deref(),
        Some("Leads/Lead 2.fxp")
    );
    assert_eq!(
        previewed(select.handle_key(press(KeyCode::Up))).as_deref(),
        Some("Leads/Lead 1.fxp")
    );
}

#[test]
fn the_first_move_from_the_current_patch_does_not_preview_it_again() {
    let mut select = opened(Some("Leads/Lead 2.fxp"));

    // 開いた時点の音色は既に鳴っているので、そこへ戻っても読み込み直さない。
    select.handle_key(press(KeyCode::Down));
    assert!(matches!(
        select.handle_key(press(KeyCode::Up)),
        PatchSelectAction::Preview(_)
    ));
}

#[test]
fn moving_past_the_end_stays_on_the_last_row_without_previewing_again() {
    let mut select = opened(None);
    for _ in 0..10 {
        select.handle_key(press(KeyCode::Down));
    }

    assert_eq!(select.selected(), Some("Basses/Bass 1.fxp"));
    assert!(matches!(
        select.handle_key(press(KeyCode::Down)),
        PatchSelectAction::Continue
    ));
}

#[test]
fn typing_filters_the_list_and_previews_the_new_head() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));

    // 拡張子 `.fxp` があるので、`p` の1文字ではまだ全件残る。
    assert!(matches!(
        select.handle_key(press(KeyCode::Char('p'))),
        PatchSelectAction::Continue
    ));
    assert_eq!(
        previewed(select.handle_key(press(KeyCode::Char('a')))).as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert_eq!(select.filtered(), ["Pads/Pad 1.fxp"]);
}

#[test]
fn a_filter_that_keeps_the_selection_does_not_preview_again() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));

    assert!(matches!(
        select.handle_key(press(KeyCode::Char('l'))),
        PatchSelectAction::Continue
    ));
    assert_eq!(select.selected(), Some("Leads/Lead 1.fxp"));
}

#[test]
fn a_filter_that_matches_nothing_leaves_the_list_empty() {
    let mut select = opened(None);
    for ch in "zzz".chars() {
        select.handle_key(press(KeyCode::Char(ch)));
    }

    assert!(select.filtered().is_empty());
    assert_eq!(select.selected(), None);
    // 選べるものが無いので、確定しても取り消し扱いにする。
    assert!(matches!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Cancel
    ));
}

#[test]
fn page_down_uses_the_page_size_reported_by_the_drawing() {
    let mut select = opened(None);
    select.set_page_size(2);

    assert_eq!(
        previewed(select.handle_key(press(KeyCode::PageDown))).as_deref(),
        Some("Pads/Pad 1.fxp")
    );
}

#[test]
fn enter_confirms_the_selection() {
    let mut select = opened(None);
    select.handle_key(press(KeyCode::Down));

    assert!(matches!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Confirm(patch) if patch == "Leads/Lead 2.fxp"
    ));
}

#[test]
fn ctrl_t_is_the_trigger() {
    assert!(is_patch_select_trigger(KeyEvent::new(
        KeyCode::Char('t'),
        KeyModifiers::CONTROL
    )));
    assert!(!is_patch_select_trigger(press(KeyCode::Char('t'))));
}
