use super::*;

mod metadata;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

fn entry(patch: &str, plugin: &str, category: Option<&str>) -> PatchCatalogEntry {
    PatchCatalogEntry::new(
        patch.to_string(),
        patch.to_lowercase(),
        plugin.to_string(),
        category.map(str::to_string),
    )
}

fn pairs(patches: &[&str]) -> Vec<PatchCatalogEntry> {
    patches
        .iter()
        .map(|patch| entry(patch, "Plugin", None))
        .collect()
}

fn all() -> Vec<PatchCatalogEntry> {
    pairs(&[
        "Leads/Lead 1.fxp",
        "Leads/Lead 2.fxp",
        "Pads/Pad 1.fxp",
        "Basses/Bass 1.fxp",
    ])
}

fn open_with(
    patches: Vec<PatchCatalogEntry>,
    current: Option<&str>,
    user_presets: Vec<(String, String)>,
) -> PatchSelect<'static> {
    PatchSelect::open(
        patches,
        current,
        user_presets,
        Default::default(),
        Vec::new(),
        Default::default(),
    )
    .expect("patch list is not empty")
}

fn opened(current: Option<&str>) -> PatchSelect<'static> {
    open_with(all(), current, Vec::new())
}

fn previewed(action: PatchSelectAction) -> Option<String> {
    match action {
        PatchSelectAction::Preview(patch) => Some(patch),
        _ => None,
    }
}

fn filtered<'a>(select: &'a PatchSelect<'_>) -> Vec<&'a str> {
    select.filtered().map(PatchCatalogEntry::display).collect()
}

fn type_text(select: &mut PatchSelect<'_>, text: &str) {
    for ch in text.chars() {
        select.handle_key(press(KeyCode::Char(ch)));
    }
}

fn select_group(select: &mut PatchSelect<'_>, index: usize) {
    select.handle_key(press(KeyCode::Left));
    select.handle_key(press(KeyCode::Left));
    for _ in 0..index {
        select.handle_key(press(KeyCode::Down));
    }
    assert_eq!(select.focus(), PatchSelectFocus::Groups);
    assert_eq!(select.group_cursor(), index);
}

#[test]
fn an_empty_patch_list_does_not_open() {
    assert!(PatchSelect::open(
        Vec::new(),
        None,
        Vec::new(),
        Default::default(),
        Vec::new(),
        Default::default()
    )
    .is_none());
}

#[test]
fn it_starts_on_the_current_patch() {
    assert_eq!(
        opened(Some("Pads/Pad 1.fxp")).selected(),
        Some("Pads/Pad 1.fxp")
    );
    assert_eq!(opened(None).selected(), Some("Basses/Bass 1.fxp"));
    assert_eq!(
        opened(Some("gone.fxp")).selected(),
        Some("Basses/Bass 1.fxp")
    );
}

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
fn role_groups_use_an_exclusive_priority_cascade() {
    let catalog = pairs(&[
        "Pads/Arp Pad.fxp",
        "Drums/Bass Drum.fxp",
        "Basses/Synth Bass.fxp",
        "Pads/Lead Pad.fxp",
        "Leads/Flute.fxp",
        "Other/Synth Cloud.fxp",
    ]);
    let expected = [
        (1, "Basses/Synth Bass.fxp"),
        (2, "Pads/Lead Pad.fxp"),
        (3, "Leads/Flute.fxp"),
        (4, "Drums/Bass Drum.fxp"),
        (5, "Pads/Arp Pad.fxp"),
        (6, "Other/Synth Cloud.fxp"),
    ];

    for (group_index, patch) in expected {
        let mut select = open_with(catalog.clone(), None, Vec::new());
        select_group(&mut select, group_index);
        assert_eq!(filtered(&select), [patch], "group index {group_index}");
    }
}

#[test]
fn etc_contains_every_patch_left_unclassified_by_the_cascade() {
    let mut select = open_with(
        pairs(&["Other/Synth Cloud.fxp", "Unknown/Plain Noise.fxp"]),
        None,
        Vec::new(),
    );

    select_group(&mut select, 6);

    assert_eq!(
        filtered(&select),
        ["Other/Synth Cloud.fxp", "Unknown/Plain Noise.fxp"]
    );
}

#[test]
fn bass_word_boundary_rejects_sbs_but_accepts_super_bs() {
    let mut select = open_with(
        pairs(&[
            "Misc/Sbs Tone.fxp",
            "Misc/Super-Bs Tone.fxp",
            "Misc/Bass Tone.fxp",
        ]),
        None,
        Vec::new(),
    );

    select_group(&mut select, 1);

    assert_eq!(
        filtered(&select),
        ["Misc/Bass Tone.fxp", "Misc/Super-Bs Tone.fxp"]
    );
}

#[test]
fn hat_is_the_only_builtin_hi_hat_spelling() {
    let catalog = pairs(&[
        "Drums/Closed Hat.fxp",
        "Drums/Hi-Hat.fxp",
        "Drums/Hihat.fxp",
        "Drums/HH.fxp",
        "Drums/OHH.fxp",
        "Drums/CHH.fxp",
    ]);
    let mut select = open_with(catalog, None, Vec::new());

    select_group(&mut select, 4);
    select.handle_key(press(KeyCode::Right));
    for _ in 0..3 {
        select.handle_key(press(KeyCode::Down));
    }

    assert_eq!(
        filtered(&select),
        ["Drums/Closed Hat.fxp", "Drums/Hi-Hat.fxp"]
    );
}

#[test]
fn brass_is_classified_as_chord_instead_of_lead() {
    let catalog = pairs(&["Brass/Trumpet.fxp", "Leads/Flute.fxp"]);

    let mut chord = open_with(catalog.clone(), None, Vec::new());
    select_group(&mut chord, 2);
    assert_eq!(filtered(&chord), ["Brass/Trumpet.fxp"]);

    let mut lead = open_with(catalog, None, Vec::new());
    select_group(&mut lead, 3);
    assert_eq!(filtered(&lead), ["Leads/Flute.fxp"]);
}

#[test]
fn a_preset_and_the_typed_regex_are_combined_with_and() {
    let mut select = open_with(
        pairs(&[
            "Pads/Warm Pad.fxp",
            "Pads/Bright Pad.fxp",
            "Strings/Warm Strings.fxp",
        ]),
        None,
        Vec::new(),
    );
    select_group(&mut select, 2);
    select.handle_key(press(KeyCode::Right));
    select.handle_key(press(KeyCode::Down));
    select.handle_key(press(KeyCode::Down));

    type_text(&mut select, "warm");

    assert_eq!(filtered(&select), ["Pads/Warm Pad.fxp"]);
}

#[test]
fn typed_terms_are_case_insensitive_regular_expressions_with_and_between_spaces() {
    let mut select = opened(None);
    type_text(&mut select, "LEAD 2|9");

    assert_eq!(filtered(&select), ["Leads/Lead 2.fxp"]);
}

#[test]
fn an_invalid_regular_expression_is_reported_and_matches_nothing() {
    let mut select = opened(None);

    select.handle_key(press(KeyCode::Char('[')));

    assert!(filtered(&select).is_empty());
    assert!(select.filter_error().is_some());
    select.handle_key(press(KeyCode::Backspace));
    assert_eq!(filtered(&select).len(), all().len());
    assert!(select.filter_error().is_none());
}

#[test]
fn ctrl_a_adds_the_query_to_the_selected_role_and_requests_persistence() {
    let mut select = open_with(
        pairs(&["Instruments/Violin.fxp", "Pads/Warm Pad.fxp"]),
        None,
        Vec::new(),
    );
    select_group(&mut select, 3);
    type_text(&mut select, "violin");

    let action = select.handle_key(ctrl('a'));

    assert!(matches!(
        action,
        PatchSelectAction::SaveUserPresets { presets, preview }
            if presets == [("lead".to_string(), "violin".to_string())]
                && preview.as_deref() == Some("Instruments/Violin.fxp")
    ));
    assert_eq!(filtered(&select), ["Instruments/Violin.fxp"]);
    assert!(select
        .presets()
        .iter()
        .any(|preset| preset.is_user && preset.label == "violin"));
}

#[test]
fn ctrl_a_in_the_all_group_puts_the_query_in_etc() {
    let mut select = opened(None);
    type_text(&mut select, "custom");

    assert!(matches!(
        select.handle_key(ctrl('a')),
        PatchSelectAction::SaveUserPresets { presets, preview: None }
            if presets == [("etc".to_string(), "custom".to_string())]
    ));
}

#[test]
fn ctrl_a_ignores_empty_invalid_and_builtin_duplicate_queries() {
    let mut select = opened(None);
    assert!(matches!(
        select.handle_key(ctrl('a')),
        PatchSelectAction::Continue
    ));
    type_text(&mut select, "[");
    assert!(matches!(
        select.handle_key(ctrl('a')),
        PatchSelectAction::Continue
    ));

    let mut select = opened(None);
    type_text(&mut select, r"\bpad");
    assert!(matches!(
        select.handle_key(ctrl('a')),
        PatchSelectAction::Continue
    ));
}

#[test]
fn persisted_user_presets_are_loaded_into_their_role() {
    let mut select = open_with(
        pairs(&["Instruments/Violin.fxp", "Other/Noise.fxp"]),
        None,
        vec![("lead".to_string(), "violin".to_string())],
    );

    select_group(&mut select, 3);

    assert_eq!(filtered(&select), ["Instruments/Violin.fxp"]);
    assert!(select
        .presets()
        .iter()
        .any(|preset| preset.label == "violin"));
}

#[test]
fn ctrl_r_jumps_to_a_different_random_row_within_the_filter() {
    let mut select = opened(None);

    for _ in 0..32 {
        let before = select.selected().unwrap().to_string();
        assert!(matches!(
            select.handle_key(ctrl('r')),
            PatchSelectAction::Preview(_)
        ));
        assert_ne!(select.selected(), Some(before.as_str()));
    }
    assert_eq!(select.focus(), PatchSelectFocus::Patches);
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
fn enter_confirms_the_selection() {
    let mut select = opened(Some("Leads/Lead 1.fxp"));
    select.handle_key(press(KeyCode::Down));

    assert!(matches!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Confirm(patch) if patch == "Leads/Lead 2.fxp"
    ));
}

#[test]
fn every_builtin_condition_is_a_valid_regular_expression() {
    for pattern in presets::builtin_patterns() {
        assert!(is_valid_condition(pattern), "{pattern}");
    }
}

#[test]
fn every_builtin_condition_has_an_explicit_leading_word_boundary() {
    for pattern in presets::builtin_patterns() {
        assert!(pattern.starts_with(r"\b"), "{pattern}");
    }
}

#[test]
fn ctrl_t_is_the_trigger() {
    assert!(is_patch_select_trigger(ctrl('t')));
    assert!(!is_patch_select_trigger(press(KeyCode::Char('t'))));
}
