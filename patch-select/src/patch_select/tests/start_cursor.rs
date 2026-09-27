use super::*;
use cmrt_patches::PatchRole;

fn open_on(
    patches: &[&str],
    current: Option<&str>,
    initial_role: Option<PatchRole>,
    favorites: &[&str],
) -> PatchSelect<'static> {
    PatchSelect::open(PatchSelectRequest {
        patches: pairs(patches),
        current: current.map(str::to_string),
        initial_role,
        favorites: favorites.iter().map(|patch| patch.to_string()).collect(),
        ..Default::default()
    })
    .expect("patch list is not empty")
}

fn start(select: &PatchSelect<'_>) -> (&'static str, String) {
    (
        FilterGroup::ALL[select.group_cursor()].label(),
        select.presets()[select.preset_cursor()].label.clone(),
    )
}

const CATALOG: [&str; 6] = [
    "Drums/Kick 1.fxp",
    "Drums/Snare 1.fxp",
    "Pads/Warm Pad.fxp",
    "Strings/Warm Strings.fxp",
    "Basses/Deep Bass.fxp",
    "Leads/Lead 1.fxp",
];

#[test]
fn a_kick_opens_on_the_kick_preset() {
    let select = open_on(&CATALOG, Some("Drums/Kick 1.fxp"), None, &[]);

    assert_eq!(
        start(&select),
        ("Drum tracks", "kick|bass drum".to_string())
    );
    assert_eq!(select.selected(), Some("Drums/Kick 1.fxp"));
}

#[test]
fn a_pad_opens_on_the_first_chord_preset_that_contains_it() {
    let select = open_on(&CATALOG, Some("Pads/Warm Pad.fxp"), None, &[]);

    assert_eq!(start(&select), ("Chord track", "pad".to_string()));
    assert_eq!(select.selected(), Some("Pads/Warm Pad.fxp"));
}

#[test]
fn a_patch_that_no_preset_of_the_role_contains_opens_on_the_roles_all() {
    let select = open_on(
        &CATALOG,
        Some("Leads/Lead 1.fxp"),
        Some(PatchRole::Bass),
        &[],
    );

    assert_eq!(start(&select), ("Bass track", "ALL".to_string()));
}

#[test]
fn an_unknown_role_opens_on_all_all() {
    for current in [None, Some("gone.fxp")] {
        let select = open_on(&CATALOG, current, None, &[]);

        assert_eq!(start(&select), ("ALL", "ALL".to_string()), "{current:?}");
    }
}

#[test]
fn a_favorite_patch_does_not_open_on_the_favorite_preset() {
    let favorites = ["Drums/Kick 1.fxp", "Pads/Warm Pad.fxp", "Leads/Lead 1.fxp"];

    let kick = open_on(&CATALOG, Some("Drums/Kick 1.fxp"), None, &favorites);
    assert_eq!(start(&kick), ("Drum tracks", "kick|bass drum".to_string()));

    let pad = open_on(&CATALOG, Some("Pads/Warm Pad.fxp"), None, &favorites);
    assert_eq!(start(&pad), ("Chord track", "pad".to_string()));

    let lead_as_bass = open_on(
        &CATALOG,
        Some("Leads/Lead 1.fxp"),
        Some(PatchRole::Bass),
        &favorites,
    );
    assert_eq!(start(&lead_as_bass), ("Bass track", "ALL".to_string()));
}

#[test]
fn an_initial_query_opens_filtered_by_that_regex() {
    let select = PatchSelect::open(PatchSelectRequest {
        patches: pairs(&CATALOG),
        initial_query: "warm".to_string(),
        ..Default::default()
    })
    .expect("patch list is not empty");

    assert_eq!(
        filtered(&select),
        ["Pads/Warm Pad.fxp", "Strings/Warm Strings.fxp"]
    );
    assert_eq!(text_input::textarea_value(select.query_textarea()), "warm");
}

#[test]
fn an_initial_query_keeps_the_current_patch_when_it_matches() {
    let select = PatchSelect::open(PatchSelectRequest {
        patches: pairs(&CATALOG),
        current: Some("Strings/Warm Strings.fxp".to_string()),
        initial_query: "warm".to_string(),
        ..Default::default()
    })
    .expect("patch list is not empty");

    assert_eq!(select.selected(), Some("Strings/Warm Strings.fxp"));
}

#[test]
fn an_invalid_initial_query_opens_with_the_regex_error_and_an_empty_list() {
    let select = PatchSelect::open(PatchSelectRequest {
        patches: pairs(&CATALOG),
        initial_query: "[".to_string(),
        ..Default::default()
    })
    .expect("patch list is not empty");

    assert!(select.filter_error().is_some());
    assert!(filtered(&select).is_empty());
}
