use super::*;

fn catalog() -> Vec<PatchCatalogEntry> {
    pairs(&[
        "Basses/Deep Bass.fxp",
        "Basses/Plain Bass.fxp",
        "Leads/Lead 1.fxp",
        "Pads/Warm Pad.fxp",
    ])
}

fn open_with_favorites(favorites: &[&str]) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: catalog(),
        favorites: favorites.iter().map(|patch| patch.to_string()).collect(),
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

fn favorite_names<'a>(select: &'a PatchSelect<'_>, group: usize) -> Vec<&'a str> {
    let preset = &select.prepared_presets.for_role(group)[1];
    assert!(preset.is_favorite);
    preset
        .matches
        .iter()
        .map(|index| select.all[*index].display())
        .collect()
}

fn select_favorite_preset(select: &mut PatchSelect<'_>, group: usize) {
    select_group(select, group);
    select.handle_key(press(KeyCode::Right));
    select.handle_key(press(KeyCode::Down));
    assert!(select.presets()[select.preset_cursor()].is_favorite);
}

#[test]
fn every_group_has_favorite_right_below_all_in_favorite_order() {
    let select = open_with_favorites(&[
        "Pads/Warm Pad.fxp",
        "Basses/Plain Bass.fxp",
        "Basses/Deep Bass.fxp",
    ]);

    for group in 0..FilterGroup::ALL.len() {
        assert_eq!(labels(&select, group)[..2], ["ALL", "★ Favorite"]);
    }
    assert_eq!(
        favorite_names(&select, 1),
        ["Basses/Plain Bass.fxp", "Basses/Deep Bass.fxp"]
    );
    assert_eq!(favorite_names(&select, 2), ["Pads/Warm Pad.fxp"]);
    assert!(favorite_names(&select, 3).is_empty());
}

#[test]
fn the_all_group_has_one_favorite_row_with_every_favorite() {
    let select = open_with_favorites(&["Leads/Lead 1.fxp", "Basses/Deep Bass.fxp"]);

    assert_eq!(
        favorite_names(&select, 0),
        ["Leads/Lead 1.fxp", "Basses/Deep Bass.fxp"]
    );
    let all_labels = labels(&select, 0);
    assert_eq!(
        all_labels
            .iter()
            .filter(|label| label.contains("★ Favorite"))
            .count(),
        1
    );
    assert!(!all_labels.iter().any(|label| label.contains("› ★")));
}

#[test]
fn favorites_that_are_not_in_the_catalog_or_repeat_are_skipped() {
    let select = open_with_favorites(&["gone.fxp", "Leads/Lead 1.fxp", "Leads/Lead 1.fxp"]);

    assert_eq!(favorite_names(&select, 0), ["Leads/Lead 1.fxp"]);
}

#[test]
fn without_favorites_the_row_still_exists_and_selects_an_empty_list() {
    let mut select = open_with_favorites(&[]);

    select_favorite_preset(&mut select, 0);

    assert!(filtered(&select).is_empty());
    assert_eq!(select.selected(), None);
}

#[test]
fn favorite_and_the_typed_regex_are_combined_with_and() {
    let mut select = open_with_favorites(&[
        "Basses/Plain Bass.fxp",
        "Basses/Deep Bass.fxp",
        "Leads/Lead 1.fxp",
    ]);
    select_favorite_preset(&mut select, 1);
    assert_eq!(
        filtered(&select),
        ["Basses/Plain Bass.fxp", "Basses/Deep Bass.fxp"]
    );

    type_text(&mut select, "deep");

    assert_eq!(filtered(&select), ["Basses/Deep Bass.fxp"]);
}

#[test]
fn set_favorites_puts_the_new_favorite_first_and_keeps_the_cursor_on_the_selected_patch() {
    let mut select = open_with_favorites(&["Leads/Lead 1.fxp", "Basses/Deep Bass.fxp"]);
    select_favorite_preset(&mut select, 0);
    select.handle_key(press(KeyCode::Right));
    select.handle_key(press(KeyCode::Down));
    assert_eq!(select.selected(), Some("Basses/Deep Bass.fxp"));

    select.set_favorites(vec![
        "Pads/Warm Pad.fxp".to_string(),
        "Leads/Lead 1.fxp".to_string(),
        "Basses/Deep Bass.fxp".to_string(),
    ]);

    assert_eq!(
        filtered(&select),
        [
            "Pads/Warm Pad.fxp",
            "Leads/Lead 1.fxp",
            "Basses/Deep Bass.fxp"
        ]
    );
    assert_eq!(select.selected(), Some("Basses/Deep Bass.fxp"));
    assert_eq!(favorite_names(&select, 2), ["Pads/Warm Pad.fxp"]);
}

#[test]
fn set_favorites_outside_the_favorite_preset_keeps_the_list() {
    let mut select = open_with_favorites(&[]);
    let before = filtered(&select)
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

    select.set_favorites(vec!["Pads/Warm Pad.fxp".to_string()]);

    assert_eq!(filtered(&select), before);
    assert_eq!(favorite_names(&select, 0), ["Pads/Warm Pad.fxp"]);
}

#[test]
fn is_favorite_follows_set_favorites() {
    let mut select = open_with_favorites(&["Leads/Lead 1.fxp"]);
    assert!(select.is_favorite("Leads/Lead 1.fxp"));
    assert!(!select.is_favorite("Pads/Warm Pad.fxp"));

    select.set_favorites(vec!["Pads/Warm Pad.fxp".to_string()]);

    assert!(!select.is_favorite("Leads/Lead 1.fxp"));
    assert!(select.is_favorite("Pads/Warm Pad.fxp"));
}

#[test]
fn adding_a_user_preset_keeps_the_favorite_row() {
    let mut select = open_with_favorites(&["Basses/Deep Bass.fxp"]);
    select_group(&mut select, 1);
    type_text(&mut select, "deep");
    select.handle_key(press(KeyCode::Enter));

    assert!(matches!(
        select.handle_key(press(KeyCode::Char('a'))),
        PatchSelectAction::SaveUserPresets { .. }
    ));
    assert_eq!(favorite_names(&select, 1), ["Basses/Deep Bass.fxp"]);
    assert!(labels(&select, 1).contains(&"deep".to_string()));
}
