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
fn selector_category_wins_over_non_triggered_display_matches() {
    let hate = "patches_3rdparty/Altenberg/Basses/Hate.fxp";
    let bass_drum = "patches_3rdparty/Giana Brotherz/Drums/Bass Drum.fxp";
    let organ_lead = "patches_factory/Leads/Organ Donor.fxp";
    let steel_drum = "patches_3rdparty/Slowboat/Keys/Quick Steel Drum.fxp";
    let bass_sequence = "patches_3rdparty/Bluelight/Basses/Bass Seq 110 BPM.fxp";
    let index = PatchRoleIndex::build(
        [
            input(hate, Some("Basses")),
            input(bass_drum, Some("Drums")),
            input(organ_lead, Some("Leads")),
            input(steel_drum, Some("Keys")),
            input(bass_sequence, Some("Basses")),
        ],
        &[],
    );

    assert_eq!(index.role_of(hate), Some(PatchRole::Bass));
    assert_eq!(index.role_of(bass_drum), Some(PatchRole::Drum));
    assert_eq!(index.role_of(organ_lead), Some(PatchRole::Lead));
    assert_eq!(index.role_of(steel_drum), Some(PatchRole::Chord));
    assert_eq!(index.role_of(bass_sequence), Some(PatchRole::Triggered));
    assert_eq!(index.candidates(PatchRole::Bass), [hate]);
    assert!(!index
        .drum_candidates(DrumPatchRole::HiHat)
        .iter()
        .any(|candidate| candidate == hate));
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
    // `drum tom` は Drum だが部位語が無いので、どの行の候補にもならない。
    assert_eq!(index.role_of("drum tom"), Some(PatchRole::Drum));
}

/// `Percussion/` フォルダ配下の kick・snare・hat が、PERC行の候補にも化けないこと。
#[test]
fn specific_drum_parts_win_over_the_percussion_folder() {
    let kick = "patches_factory/Percussion/Kick 909ish.fxp";
    let snare = "patches_3rdparty/Kinsey Dulcet/Percussion/Deep Cut Snare.fxp";
    let hat = "patches_3rdparty/Psiome Send Sound/Percussion/Hat Electro.fxp";
    let perc = "patches_3rdparty/Slowboat/Percussion/Djembeish 1.fxp";
    let index = PatchRoleIndex::build(
        [
            input(kick, None),
            input(snare, None),
            input(hat, None),
            input(perc, None),
        ],
        &[],
    );

    assert_eq!(index.drum_candidates(DrumPatchRole::Kick), [kick]);
    assert_eq!(index.drum_candidates(DrumPatchRole::Snare), [snare]);
    assert_eq!(index.drum_candidates(DrumPatchRole::HiHat), [hat]);
    assert_eq!(index.drum_candidates(DrumPatchRole::Percussion), [perc]);
}

/// bassdrum はバスドラムなので Kick 行。`Percussion/` 配下にあっても変わらない。
#[test]
fn bass_drum_is_a_kick_in_both_spellings() {
    let spaced = "patches_3rdparty/John Valentine/Percussion/Orchestral Bass Drum.fxp";
    let joined = "sfz/Virtual-Playing-Orchestra3/Percussion/bassdrum.sfz";
    let plain = "patches_3rdparty/Giana Brotherz/Drums/Bass Drum.fxp";
    let index = PatchRoleIndex::build(
        [input(spaced, None), input(joined, None), input(plain, None)],
        &[],
    );

    assert_eq!(index.role_of(plain), Some(PatchRole::Drum));
    assert_eq!(
        index.drum_candidates(DrumPatchRole::Kick),
        [spaced, joined, plain]
    );
    assert!(index.drum_candidates(DrumPatchRole::Percussion).is_empty());
}

/// 表示名から部位を引き直せること。同じ用途の音色だけを抽選し直す側が使う。
#[test]
fn drum_role_of_answers_the_part_only_for_drums_with_a_part_word() {
    let index = PatchRoleIndex::build(
        [
            input("Drums/Kick Clean.fxp", None),
            input("Drums/Snare Tight.fxp", None),
            input("Drums/Closed Hat.fxp", None),
            input("Drums/Perc Shaker.fxp", None),
            input("Drums/Drum Tom.fxp", None),
            input("Pads/Warm Pad.fxp", None),
        ],
        &[],
    );

    assert_eq!(
        index.drum_role_of("Drums/Kick Clean.fxp"),
        Some(DrumPatchRole::Kick)
    );
    assert_eq!(
        index.drum_role_of("Drums/Snare Tight.fxp"),
        Some(DrumPatchRole::Snare)
    );
    assert_eq!(
        index.drum_role_of("Drums/Closed Hat.fxp"),
        Some(DrumPatchRole::HiHat)
    );
    assert_eq!(
        index.drum_role_of("Drums/Perc Shaker.fxp"),
        Some(DrumPatchRole::Percussion)
    );
    // Drum だが部位語が無い音色と、Drum 以外は、どちらも部位を持たない。
    assert_eq!(
        index.role_of("Drums/Drum Tom.fxp"),
        Some(PatchRole::Drum),
        "部位語が無くても role は Drum のまま"
    );
    assert_eq!(index.drum_role_of("Drums/Drum Tom.fxp"), None);
    assert_eq!(index.drum_role_of("Pads/Warm Pad.fxp"), None);
    assert_eq!(index.drum_role_of("Missing/Not In Catalog.fxp"), None);
}

#[test]
fn every_builtin_condition_has_an_explicit_leading_word_boundary() {
    for preset in builtin_role_presets() {
        assert!(preset.pattern.starts_with(r"\b"), "{}", preset.pattern);
        assert!(is_valid_condition(preset.pattern), "{}", preset.pattern);
    }
}
