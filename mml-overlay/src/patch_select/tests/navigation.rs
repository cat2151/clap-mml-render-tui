use super::*;

#[test]
fn left_and_right_move_between_the_three_panes() {
    let mut select = opened(None);

    assert_eq!(select.focus(), PatchSelectFocus::Patches);
    select.handle_key(press(KeyCode::Left));
    assert_eq!(select.focus(), PatchSelectFocus::Presets);
    select.handle_key(press(KeyCode::Left));
    assert_eq!(select.focus(), PatchSelectFocus::Groups);
    select.handle_key(press(KeyCode::Right));
    assert_eq!(select.focus(), PatchSelectFocus::Presets);
    select.handle_key(press(KeyCode::Right));
    assert_eq!(select.focus(), PatchSelectFocus::Patches);
}

#[test]
fn hjkl_move_like_the_arrow_keys() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));

    select.handle_key(press(KeyCode::Char('h')));
    assert_eq!(select.focus(), PatchSelectFocus::Presets);
    select.handle_key(press(KeyCode::Char('h')));
    assert_eq!(select.focus(), PatchSelectFocus::Groups);
    select.handle_key(press(KeyCode::Char('l')));
    assert_eq!(select.focus(), PatchSelectFocus::Presets);
    select.handle_key(press(KeyCode::Char('l')));
    assert_eq!(select.focus(), PatchSelectFocus::Patches);

    assert_eq!(
        previewed(select.handle_key(press(KeyCode::Char('j')))).as_deref(),
        Some("Leads/Lead 2.fxp")
    );
    assert_eq!(
        previewed(select.handle_key(press(KeyCode::Char('k')))).as_deref(),
        Some("Leads/Lead 1.fxp")
    );
}

#[test]
fn home_and_end_move_to_the_edges_of_the_focused_pane() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));

    assert_eq!(
        previewed(select.handle_key(press(KeyCode::End))).as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert_eq!(
        previewed(select.handle_key(press(KeyCode::Home))).as_deref(),
        Some("Basses/Bass 1.fxp")
    );

    select.handle_key(press(KeyCode::Char('h')));
    select.handle_key(press(KeyCode::End));
    assert_eq!(select.preset_cursor(), select.presets().len() - 1);
    select.handle_key(press(KeyCode::Home));
    assert_eq!(select.preset_cursor(), 0);

    select.handle_key(press(KeyCode::Char('h')));
    select.handle_key(press(KeyCode::End));
    assert_eq!(select.group_cursor(), select.groups().len() - 1);
    select.handle_key(press(KeyCode::Home));
    assert_eq!(select.group_cursor(), 0);
}

#[test]
fn page_up_and_page_down_always_move_ten_rows() {
    let patches = (0..12)
        .map(|index| entry(&format!("Patch {index:02}.fxp"), "Plugin", None))
        .collect();
    let mut select = open_with(patches, None, Vec::new());

    assert_eq!(
        previewed(select.handle_key(press(KeyCode::PageDown))).as_deref(),
        Some("Patch 10.fxp")
    );
    assert_eq!(
        previewed(select.handle_key(press(KeyCode::PageUp))).as_deref(),
        Some("Patch 00.fxp")
    );
}

#[test]
fn moving_in_the_patch_pane_previews_the_new_patch() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));

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
fn space_previews_the_current_line_and_ctrl_space_does_not() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));

    assert!(matches!(
        select.handle_key(press(KeyCode::Char(' '))),
        PatchSelectAction::PlayLine(patch) if patch == "Leads/Lead 1.fxp"
    ));
    assert!(matches!(
        select.handle_key(ctrl(' ')),
        PatchSelectAction::Continue
    ));
}
