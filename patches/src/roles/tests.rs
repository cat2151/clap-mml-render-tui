use super::*;

fn input<'a>(display: &'a str, category: Option<&'a str>) -> PatchRoleInput<'a> {
    PatchRoleInput {
        display,
        normalized_display: display,
        selector_category: category,
    }
}

#[test]
fn roles_use_the_latest_exclusive_cascade() {
    let index = PatchRoleIndex::build(
        [
            input("arp bass pad", None),
            input("bass pad", None),
            input("pad lead", None),
            input("flute", None),
            input("plain", None),
        ],
        &[],
    );

    assert_eq!(index.role_of("arp bass pad"), Some(PatchRole::Triggered));
    assert_eq!(index.role_of("bass pad"), Some(PatchRole::Bass));
    assert_eq!(index.role_of("pad lead"), Some(PatchRole::Chord));
    assert_eq!(index.role_of("flute"), Some(PatchRole::Lead));
    assert_eq!(index.role_of("plain"), Some(PatchRole::Etc));
}

#[test]
fn selector_category_and_user_rules_participate_in_classification() {
    let user = vec![("lead".to_string(), r"\bviolin".to_string())];
    let index = PatchRoleIndex::build(
        [
            input("BA Admiral.vvp", Some("Bass")),
            input("violin solo", None),
        ],
        &user,
    );

    assert_eq!(index.role_of("BA Admiral.vvp"), Some(PatchRole::Bass));
    assert_eq!(index.role_of("violin solo"), Some(PatchRole::Lead));
}

#[test]
fn bass_word_boundary_rejects_sbs_but_accepts_super_bs() {
    let index = PatchRoleIndex::build([input("sbs", None), input("super-bs", None)], &[]);

    assert_eq!(index.role_of("sbs"), Some(PatchRole::Etc));
    assert_eq!(index.role_of("super-bs"), Some(PatchRole::Bass));
}

#[test]
fn drum_parts_are_explicit_and_percussion_is_not_the_remainder() {
    let index = PatchRoleIndex::build(
        [
            input("kick", None),
            input("snare", None),
            input("closed hat", None),
            input("perc shaker", None),
            input("drum tom", None),
        ],
        &[],
    );

    assert_eq!(index.drum_candidates(DrumPatchRole::Kick), ["kick"]);
    assert_eq!(index.drum_candidates(DrumPatchRole::Snare), ["snare"]);
    assert_eq!(index.drum_candidates(DrumPatchRole::HiHat), ["closed hat"]);
    assert_eq!(
        index.drum_candidates(DrumPatchRole::Percussion),
        ["perc shaker"]
    );
}

#[test]
fn every_builtin_condition_has_an_explicit_leading_word_boundary() {
    for preset in builtin_role_presets() {
        assert!(preset.pattern.starts_with(r"\b"), "{}", preset.pattern);
        assert!(is_valid_condition(preset.pattern), "{}", preset.pattern);
    }
}
