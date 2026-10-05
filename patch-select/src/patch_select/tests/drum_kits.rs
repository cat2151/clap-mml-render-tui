use super::*;

const DRUM_GROUP: usize = 4;

fn kit() -> PatchLoadMeasurement {
    PatchLoadMeasurement {
        drum_kit: true,
        ..Default::default()
    }
}

fn open_with_kits(kits: &[&str]) -> PatchSelect<'static> {
    open_on(None, kits)
}

fn open_on(current: Option<&str>, kits: &[&str]) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        current: current.map(str::to_string),
        patches: pairs(&[
            "Drums/Kick 1.fxp",
            "Drums/Full Drums.sfz",
            "Kits/909.sfz",
            "Pads/Warm Pad.fxp",
        ]),
        load_measurements: kits
            .iter()
            .map(|patch| (patch.to_string(), kit()))
            .collect(),
        ..Default::default()
    })
    .expect("patch list is not empty")
}

fn labels(select: &PatchSelect<'_>, group: usize) -> Vec<String> {
    select
        .prepared_presets
        .for_role(group)
        .iter()
        .map(|preset| preset.label.clone())
        .collect()
}

#[test]
fn drum_group_has_drum_kit_right_below_favorite() {
    let select = open_with_kits(&[]);

    assert_eq!(
        FilterGroup::ALL[DRUM_GROUP],
        FilterGroup::Role(PatchRole::Drum)
    );
    assert_eq!(
        labels(&select, DRUM_GROUP)[..3],
        ["ALL", "★ Favorite", "Drum kit"]
    );
}

#[test]
fn drum_kit_is_only_in_the_drum_group() {
    let select = open_with_kits(&["Drums/Full Drums.sfz"]);

    for group in (0..FilterGroup::ALL.len()).filter(|group| *group != DRUM_GROUP) {
        assert!(
            !select
                .prepared_presets
                .for_role(group)
                .iter()
                .any(|preset| preset.is_drum_kit),
            "group {group}"
        );
    }
}

#[test]
fn drum_kit_lists_every_kit_even_outside_the_drum_role() {
    let mut select = open_with_kits(&["Drums/Full Drums.sfz", "Kits/909.sfz"]);

    select_group(&mut select, DRUM_GROUP);
    select.handle_key(press(KeyCode::Right));
    select.handle_key(press(KeyCode::Down));
    select.handle_key(press(KeyCode::Down));

    assert!(select.presets()[select.preset_cursor()].is_drum_kit);
    assert_eq!(filtered(&select), ["Drums/Full Drums.sfz", "Kits/909.sfz"]);
}

#[test]
fn drum_kit_is_not_the_start_preset_for_a_drum_kit_patch() {
    let select = open_on(Some("Drums/Full Drums.sfz"), &["Drums/Full Drums.sfz"]);

    assert!(!select.presets()[select.preset_cursor()].is_drum_kit);
}

fn fixed() -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: pairs(&[
            "Drums/Kick 1.fxp",
            "Drums/Full Drums.sfz",
            "Kits/909.sfz",
            "Pads/Warm Pad.fxp",
        ]),
        current: Some("Pads/Warm Pad.fxp".to_string()),
        drum_kit_only: true,
        load_measurements: ["Drums/Full Drums.sfz", "Kits/909.sfz"]
            .into_iter()
            .map(|patch| (patch.to_string(), kit()))
            .collect(),
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn fixed_mode_keeps_every_kit_and_cannot_change_role_or_preset() {
    let mut select = fixed();
    let initial = (select.group_cursor(), select.preset_cursor());
    assert_eq!(initial.0, DRUM_GROUP);
    assert!(select.presets()[initial.1].is_drum_kit);
    assert!(select.captures_all_keys());

    // Left/Right and hjkl cannot move focus to an editable Role/Preset pane.
    for code in [
        KeyCode::Left,
        KeyCode::Left,
        KeyCode::Up,
        KeyCode::Home,
        KeyCode::Right,
        KeyCode::Char('h'),
        KeyCode::Char('j'),
        KeyCode::End,
        KeyCode::PageDown,
    ] {
        assert_eq!(select.handle_key(press(code)), PatchSelectAction::Continue);
        assert_eq!((select.group_cursor(), select.preset_cursor()), initial);
        assert_eq!(select.focus(), PatchSelectFocus::Patches);
        assert_eq!(filtered(&select), ["Drums/Full Drums.sfz", "Kits/909.sfz"]);
    }
    assert_eq!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Confirm("Kits/909.sfz".to_string())
    );
}

#[test]
fn fixed_mode_search_and_actions_never_broaden_or_audition_the_kit_set() {
    let mut select = fixed();
    for ch in ['a', 'r', 'm', 'e', 'E', 's', ' '] {
        assert_eq!(
            select.handle_key(press(KeyCode::Char(ch))),
            PatchSelectAction::Continue
        );
    }
    assert_eq!(select.handle_key(ctrl('l')), PatchSelectAction::Continue);
    assert!(select.plugin_menu.is_none());
    assert!(select.auto_reverb_panel().is_none());
    assert_eq!(filtered(&select), ["Drums/Full Drums.sfz", "Kits/909.sfz"]);

    type_text(&mut select, "warm|909");
    assert_eq!(filtered(&select), ["Kits/909.sfz"]);
    select.handle_key(press(KeyCode::Enter));
    select.set_favorites(vec!["Pads/Warm Pad.fxp".to_string()]);
    assert_eq!(filtered(&select), ["Kits/909.sfz"]);
    select.handle_key(press(KeyCode::Char('/')));
    for _ in 0.."warm|909".len() {
        select.handle_key(press(KeyCode::Backspace));
    }
    type_text(&mut select, "warm");
    assert_eq!(select.filtered_len(), 0);
    assert_eq!(select.previewed(), Some("Pads/Warm Pad.fxp"));
}

#[test]
fn fixed_mode_ignores_release_and_repeat_confirmation() {
    use crossterm::event::KeyEventKind;
    let mut select = fixed();
    for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
        let mut key = press(KeyCode::Enter);
        key.kind = kind;
        assert_eq!(select.handle_key(key), PatchSelectAction::Continue);
    }
    assert_eq!(
        select.handle_key(press(KeyCode::Esc)),
        PatchSelectAction::Cancel
    );
}
