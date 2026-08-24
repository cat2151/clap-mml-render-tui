use super::*;

#[test]
fn patches_are_sorted_by_category_then_plugin_then_patch_name() {
    let select = open_with(
        vec![
            entry("Zeta.fxp", "Plugin B", Some("Bass")),
            entry("Zeta.fxp", "Plugin A", Some("Pad")),
            entry("Zulu.fxp", "Plugin A", Some("Bass")),
            entry("Alpha.fxp", "Plugin A", Some("bass")),
            entry("No Category.fxp", "Plugin A", None),
        ],
        None,
        Vec::new(),
    );

    assert_eq!(
        filtered(&select),
        [
            "Alpha.fxp",
            "Zulu.fxp",
            "Zeta.fxp",
            "Zeta.fxp",
            "No Category.fxp"
        ]
    );
}

#[test]
fn expanded_selector_category_participates_in_regex_and_cascade_matching() {
    let catalog = vec![
        entry("BA Admiral.vvp", "Vaporizer2", Some("Bass")),
        entry("PD Archangel.vvp", "Vaporizer2", Some("Pad")),
    ];
    let mut bass = open_with(catalog.clone(), None, Vec::new());
    select_group(&mut bass, 1);
    assert_eq!(filtered(&bass), ["BA Admiral.vvp"]);

    let mut chord = open_with(catalog, None, Vec::new());
    select_group(&mut chord, 2);
    type_text(&mut chord, "archangel");
    assert_eq!(filtered(&chord), ["PD Archangel.vvp"]);
}

#[test]
fn all_labels_are_compact_in_both_left_panes() {
    let select = opened(None);

    assert_eq!(select.groups()[0].label(), "ALL");
    assert_eq!(select.presets()[0].label, "ALL");
}

#[test]
fn all_role_lists_every_preset_in_visible_role_order() {
    let select = open_with(
        all(),
        None,
        vec![("lead".to_string(), "violin".to_string())],
    );
    let labels = select
        .presets()
        .iter()
        .map(|preset| preset.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(labels[0], "ALL");
    let bass = labels
        .iter()
        .position(|label| *label == "Bass › bass|bs")
        .unwrap();
    let chord = labels
        .iter()
        .position(|label| *label == "Chord › strings")
        .unwrap();
    let lead = labels
        .iter()
        .position(|label| *label == "Lead › lead")
        .unwrap();
    let user = labels
        .iter()
        .position(|label| *label == "Lead › violin")
        .unwrap();
    let drum = labels
        .iter()
        .position(|label| *label == "Drum › kick")
        .unwrap();
    let trigger = labels
        .iter()
        .position(|label| *label == "Triggered › chord")
        .unwrap();
    let etc = labels
        .iter()
        .position(|label| *label == "Etc › synth")
        .unwrap();
    assert!(bass < chord && chord < lead && lead < user && user < drum);
    assert!(drum < trigger && trigger < etc);
}

#[test]
fn preset_selected_from_all_uses_its_owning_roles_cascade() {
    let catalog = pairs(&[
        "Pads/Arp Bass Pad.fxp",
        "Basses/Plain Bass.fxp",
        "Basses/Deep Bass.fxp",
        "Other/Plain Tone.fxp",
    ]);
    let mut from_all = open_with(catalog.clone(), None, Vec::new());
    from_all.handle_key(press(KeyCode::Left));
    from_all.handle_key(press(KeyCode::Down));

    let mut from_bass = open_with(catalog, None, Vec::new());
    select_group(&mut from_bass, 1);
    from_bass.handle_key(press(KeyCode::Right));
    from_bass.handle_key(press(KeyCode::Down));

    assert_eq!(
        from_all.presets()[from_all.preset_cursor()].label,
        "Bass › bass|bs"
    );
    assert!(std::sync::Arc::ptr_eq(
        &from_all.filtered,
        &from_all.presets()[from_all.preset_cursor()].matches
    ));
    assert_eq!(filtered(&from_all), filtered(&from_bass));
    assert_eq!(
        filtered(&from_all),
        ["Basses/Deep Bass.fxp", "Basses/Plain Bass.fxp"]
    );

    type_text(&mut from_all, "plain");
    assert_eq!(filtered(&from_all), ["Basses/Plain Bass.fxp"]);
}

#[test]
fn all_role_shortcuts_share_the_owning_roles_precomputed_matches() {
    let select = opened(None);
    let all_bass = &select.prepared_presets.for_role(0)[1];
    let role_bass = &select.prepared_presets.for_role(1)[1];

    assert_eq!(all_bass.label, "Bass › bass|bs");
    assert!(std::sync::Arc::ptr_eq(
        &all_bass.matches,
        &role_bass.matches
    ));
}

#[test]
fn adding_from_an_all_shortcut_uses_the_shortcuts_owning_role() {
    let mut select = opened(None);
    select.handle_key(press(KeyCode::Left));
    select.handle_key(press(KeyCode::Down));
    type_text(&mut select, "sub");

    assert!(matches!(
        select.handle_key(ctrl('a')),
        PatchSelectAction::SaveUserPresets { presets, .. }
            if presets == [("bass".to_string(), "sub".to_string())]
    ));
}
