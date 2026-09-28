use super::*;

mod plugin_terms;

fn input<'a>(display: &'a str, category: Option<&'a str>) -> PatchRoleInput<'a> {
    PatchRoleInput {
        display,
        normalized_display: display,
        selector_category: category,
        plugin: None,
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
    let user = vec![("lead".to_string(), r"\btheremin".to_string())];
    let index = PatchRoleIndex::build(
        [
            input("BA Admiral.vvp", Some("Bass")),
            input("theremin solo", None),
        ],
        &user,
    );

    assert_eq!(index.role_of("BA Admiral.vvp"), Some(PatchRole::Bass));
    assert_eq!(index.role_of("theremin solo"), Some(PatchRole::Lead));
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
            input("drum loop", None),
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
    // `drum loop` は Drum だが部位語が無いので、どの行の候補にもならない。
    assert_eq!(index.role_of("drum loop"), Some(PatchRole::Drum));
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
            input("Drums/Drum Loop.fxp", None),
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
        index.role_of("Drums/Drum Loop.fxp"),
        Some(PatchRole::Drum),
        "部位語が無くても role は Drum のまま"
    );
    assert_eq!(index.drum_role_of("Drums/Drum Loop.fxp"), None);
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

#[test]
fn preset_label_is_the_first_builtin_preset_within_the_decided_role() {
    let user = vec![("lead".to_string(), r"\btheremin".to_string())];
    let index = PatchRoleIndex::build(
        [
            input("DX Snare 1", None),
            input("Percussion/Kick Clean.fxp", None),
            input("String Pad", Some("Pads")),
            input("theremin solo", None),
            input("synth thing", None),
            input("plain", None),
        ],
        &user,
    );

    assert_eq!(index.preset_label_of("DX Snare 1"), Some("snare"));
    assert_eq!(
        index.preset_label_of("Percussion/Kick Clean.fxp"),
        Some("kick|bass drum")
    );
    // labelはRoleを決めた階層（ここではcategory）の中で探す。
    assert_eq!(index.preset_label_of("String Pad"), Some("pad"));
    assert_eq!(index.role_of("theremin solo"), Some(PatchRole::Lead));
    assert_eq!(index.preset_label_of("theremin solo"), None);
    assert_eq!(index.role_of("synth thing"), Some(PatchRole::Etc));
    assert_eq!(index.preset_label_of("synth thing"), Some("synth"));
    assert_eq!(index.preset_label_of("plain"), None);
}

/// 近い階層が勝つので、`Brass/`や`Percussion/`フォルダの下でもpatch名の楽器になる。
#[test]
fn a_nearer_level_wins_over_the_folders_above_it() {
    let sax = "dexed_cart_1.0/!instruments/brass/sax/sax01.syx/03 highsax";
    let bari = "dexed_cart_1.0/brass3.syx/12 bari sax";
    let bell = "dexed_cart_1.0/percussion/bells/tubular bell";
    let horn = "dexed_cart_1.0/brass/horns.syx/01 french horn";
    let index = PatchRoleIndex::build(
        [
            input(sax, None),
            input(bari, None),
            input(bell, None),
            input(horn, None),
        ],
        &[],
    );

    // `highsax`は語頭境界で外れ、1つ上の`sax01.syx`で決まる。
    assert_eq!(index.preset_label_of(sax), Some("sax"));
    assert_eq!(index.preset_label_of(bari), Some("sax"));
    assert_eq!(index.role_of(bell), Some(PatchRole::Lead));
    assert_eq!(index.preset_label_of(bell), Some("bell"));
    assert_eq!(index.preset_label_of(horn), Some("brass"));
}

#[test]
fn sax_is_its_own_label_apart_from_woodwind() {
    let index = PatchRoleIndex::build([input("alto sax", None), input("flute", None)], &[]);

    assert_eq!(index.preset_label_of("alto sax"), Some("sax"));
    assert_eq!(index.preset_label_of("flute"), Some("woodwind"));
}

/// 作者名`Jeff Saxe`はsaxの根拠にしない。その下の音色は自分の名前で決まる。
#[test]
fn the_jeff_saxe_folder_is_not_evidence_of_a_sax() {
    let crash = "dexed_cart_1.0/jeff saxe/816-h.syx/05 crash cy";
    let tom = "dexed_cart_1.0/jeff saxe/816-h.syx/06 drum tom";
    let index = PatchRoleIndex::build([input(crash, None), input(tom, None)], &[]);

    assert_eq!(index.role_of(crash), Some(PatchRole::Drum));
    assert_eq!(index.preset_label_of(crash), Some("perc"));
    assert_eq!(index.role_of(tom), Some(PatchRole::Drum));
}

#[test]
fn joined_handclaps_are_percussion() {
    let joined = "dexed_cart_1.0/!unsorted/tim garrett/tx7-34b.syx/00 handclap1";
    let spaced = "dexed_cart_1.0/!instruments/misc/dx7_a6.syx/10 hand claps";
    let index = PatchRoleIndex::build([input(joined, None), input(spaced, None)], &[]);

    assert_eq!(index.preset_label_of(joined), Some("perc"));
    assert_eq!(
        index.drum_candidates(DrumPatchRole::Percussion),
        [joined, spaced]
    );
}

/// Surgeはcategoryも`Percussion`なので、部位は階層を遡らず具体的な部位語が勝つ。
#[test]
fn a_percussion_category_does_not_take_kicks_from_the_kick_row() {
    let kick = "patches_factory/Percussion/Kick 909ish.fxp";
    let index = PatchRoleIndex::build([input(kick, Some("Percussion"))], &[]);

    assert_eq!(index.drum_candidates(DrumPatchRole::Kick), [kick]);
}

#[test]
fn abbreviated_bass_drums_are_kicks_but_bass_drive_is_a_bass() {
    let index = PatchRoleIndex::build(
        [
            input("percussion/drums/drums01.syx/14 bass dr. 2", None),
            input("mega rom/percussion 1.syx/19 bassdr,2ms", None),
            input("bass drive", None),
        ],
        &[],
    );

    assert_eq!(
        index.drum_candidates(DrumPatchRole::Kick),
        [
            "percussion/drums/drums01.syx/14 bass dr. 2",
            "mega rom/percussion 1.syx/19 bassdr,2ms"
        ]
    );
    assert_eq!(index.role_of("bass drive"), Some(PatchRole::Bass));
}

/// Dexedの10文字に詰めた略記と連結語も、楽器のlabelへ入る。
#[test]
fn abbreviated_instrument_names_get_their_labels() {
    let cases = [
        ("dx7 rhodes", "keyboard|keys|piano"),
        ("02 proc.e.pno", "keyboard|keys|piano"),
        ("13 wurli 8", "keyboard|keys|piano"),
        ("harpsich.0", "keyboard|keys|piano"),
        ("hammond 2", "organ"),
        ("strgs low", "strings"),
        ("syn brs 1a", "brass"),
        ("syn. vox 3", "choir|vocal"),
        ("jazz-guit1", "guitar|gtr"),
        ("00 eleguitar1", "guitar|gtr"),
        ("08 nylongtr.a", "guitar|gtr"),
        ("12 jazzguitar", "guitar|gtr"),
        ("r synbass1", "bass|bs"),
        ("koto 2", "pluck"),
        ("harp    2", "pluck"),
        ("k.sitar  1", "pluck"),
        ("panflute2", "woodwind"),
        ("marimba", "mallet"),
        ("vibra x  3", "mallet"),
        ("music box", "mallet"),
        ("tom toms", "perc"),
        ("congas", "perc"),
        ("crash.cymb", "perc"),
    ];
    let index = PatchRoleIndex::build(cases.map(|(name, _)| input(name, None)), &[]);

    for (name, label) in cases {
        assert_eq!(index.preset_label_of(name), Some(label), "{name}");
    }
}

/// `harp`・`synbass`・`vibra`は、別の楽器の略記が同じ綴りで始まる。
#[test]
fn prefixes_shared_with_other_instruments_stay_apart() {
    let index = PatchRoleIndex::build(
        [
            input("harpic 3", None),
            input("harpiano", None),
            input("synbassoon", None),
            input("vibrato", None),
        ],
        &[],
    );

    assert_eq!(index.preset_label_of("harpic 3"), None);
    assert_eq!(index.preset_label_of("harpiano"), None);
    assert_ne!(index.role_of("synbassoon"), Some(PatchRole::Bass));
    assert_eq!(index.preset_label_of("vibrato"), None);
}

/// 部位語の無い打楽器はPERC行へ入る。
#[test]
fn toms_and_congas_are_percussion_row_candidates() {
    let index = PatchRoleIndex::build([input("tom toms", None), input("congas", None)], &[]);

    assert_eq!(
        index.drum_candidates(DrumPatchRole::Percussion),
        ["tom toms", "congas"]
    );
}
