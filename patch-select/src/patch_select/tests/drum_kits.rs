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
