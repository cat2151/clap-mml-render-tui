use super::*;

/// 名前に `poly` を含む patch だけを poly と判定するスタブ。
struct PolyByName;

impl VoicingLookup for PolyByName {
    fn is_poly(&self, patch: &str) -> bool {
        patch.to_lowercase().contains("poly")
    }
}

fn pairs(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| (name.to_string(), name.to_lowercase()))
        .collect()
}

fn categories(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn chord_requires_both_the_category_and_poly() {
    let cats = categories(&["Pads"]);
    let filter = RoleFilter::new(PatchRole::Chord, &cats);

    assert!(matches_role(
        "patches_factory/Pads/Poly Pad.fxp",
        "patches_factory/pads/poly pad.fxp",
        &filter,
        &PolyByName
    ));
    // カテゴリは合うが mono（未判定）。
    assert!(!matches_role(
        "patches_factory/Pads/Mono Pad.fxp",
        "patches_factory/pads/mono pad.fxp",
        &filter,
        &PolyByName
    ));
    // poly だがカテゴリ違い。
    assert!(!matches_role(
        "patches_factory/Leads/Poly Lead.fxp",
        "patches_factory/leads/poly lead.fxp",
        &filter,
        &PolyByName
    ));
}

#[test]
fn bass_and_arpeggio_only_look_at_the_category() {
    let cats = categories(&["Basses"]);
    let filter = RoleFilter::new(PatchRole::Bass, &cats);

    // mono でも bass は成立する。
    assert!(matches_role(
        "patches_factory/Basses/Mono Bass.fxp",
        "patches_factory/basses/mono bass.fxp",
        &filter,
        &PolyByName
    ));

    let cats = categories(&["Plucks"]);
    let filter = RoleFilter::new(PatchRole::Arpeggio, &cats);

    assert!(matches_role(
        "patches_factory/Plucks/Mono Pluck.fxp",
        "patches_factory/plucks/mono pluck.fxp",
        &filter,
        &PolyByName
    ));
}

/// Free は chord 候補の否定。和音向きの音色は chord 行へ回す。
#[test]
fn free_is_the_complement_of_chord() {
    let cats = categories(&["Pads"]);
    let chord = RoleFilter::new(PatchRole::Chord, &cats);
    let free = RoleFilter::new(PatchRole::Free, &cats);

    for (display, lower) in pairs(&[
        "patches_factory/Pads/Poly Pad.fxp",
        "patches_factory/Pads/Mono Pad.fxp",
        "patches_factory/Leads/Poly Lead.fxp",
    ]) {
        assert_ne!(
            matches_role(&display, &lower, &chord, &PolyByName),
            matches_role(&display, &lower, &free, &PolyByName),
            "{display} で Chord と Free が排他になっていない"
        );
    }
}

#[test]
fn empty_categories_do_not_narrow_the_candidates() {
    let filter = RoleFilter::new(PatchRole::Arpeggio, &[]);

    assert!(matches_role(
        "patches_factory/Drums/Kick.fxp",
        "patches_factory/drums/kick.fxp",
        &filter,
        &PolyByName
    ));
}

#[test]
fn pick_for_role_only_draws_from_the_candidates() {
    let all = pairs(&[
        "patches_factory/Pads/Poly Pad.fxp",
        "patches_factory/Pads/Mono Pad.fxp",
        "patches_factory/Leads/Poly Lead.fxp",
    ]);
    let cats = categories(&["Pads"]);
    let filter = RoleFilter::new(PatchRole::Chord, &cats);

    for _ in 0..32 {
        assert_eq!(
            pick_for_role(&all, &filter, &PolyByName).as_deref(),
            Some("patches_factory/Pads/Poly Pad.fxp")
        );
    }
}

#[test]
fn pick_for_role_returns_none_when_nothing_matches() {
    let all = pairs(&["patches_factory/Pads/Mono Pad.fxp"]);
    let cats = categories(&["Pads"]);
    let filter = RoleFilter::new(PatchRole::Chord, &cats);

    assert_eq!(pick_for_role(&all, &filter, &PolyByName), None);
}

/// 実機の Surge にある名前をそのまま並べた drum の候補。
fn drum_pairs() -> Vec<(String, String)> {
    pairs(&[
        "patches_factory/Percussion/Kick 909ish.fxp",
        "patches_3rdparty/Vendor/Drums/Bass Drum.fxp",
        "patches_factory/Percussion/Snare Tight.fxp",
        "patches_3rdparty/Vendor/Drums/Closed Hi-Hat.fxp",
        "patches_3rdparty/Vendor/Drums/Cowbell.fxp",
        "patches_factory/Pads/Poly Pad.fxp",
    ])
}

fn matching(pairs: &[(String, String)], filter: &RoleFilter<'_>) -> Vec<String> {
    pairs
        .iter()
        .filter(|(display, lower)| matches_role(display, lower, filter, &PolyByName))
        .map(|(display, _)| display.clone())
        .collect()
}

/// カテゴリは `Percussion` / `Drums` の粒度しか無いので、役割はキーワードで分ける。
#[test]
fn drum_roles_split_one_category_by_name_keywords() {
    let all = drum_pairs();
    let cats = categories(&["Percussion", "Drums"]);
    let kick = categories(&["kick", "bass drum"]);
    let snare = categories(&["snare"]);
    let hat = categories(&["hat"]);

    assert_eq!(
        matching(
            &all,
            &RoleFilter::with_keywords(PatchRole::Kick, &cats, &kick)
        ),
        [
            "patches_factory/Percussion/Kick 909ish.fxp",
            "patches_3rdparty/Vendor/Drums/Bass Drum.fxp",
        ]
    );
    assert_eq!(
        matching(
            &all,
            &RoleFilter::with_keywords(PatchRole::Snare, &cats, &snare)
        ),
        ["patches_factory/Percussion/Snare Tight.fxp"]
    );
    assert_eq!(
        matching(
            &all,
            &RoleFilter::with_keywords(PatchRole::HiHat, &cats, &hat)
        ),
        ["patches_3rdparty/Vendor/Drums/Closed Hi-Hat.fxp"]
    );
}

/// percussion は「他の3役に取られなかった残り全部」。判定が反転する。
#[test]
fn percussion_takes_what_the_other_drum_roles_left() {
    let all = drum_pairs();
    let cats = categories(&["Percussion", "Drums"]);
    let others = categories(&["kick", "bass drum", "snare", "hat"]);

    assert_eq!(
        matching(
            &all,
            &RoleFilter::with_keywords(PatchRole::Percussion, &cats, &others)
        ),
        ["patches_3rdparty/Vendor/Drums/Cowbell.fxp"]
    );
}

/// drum は単音なので poly を要求しない。要求すると候補がほぼ全滅する。
#[test]
fn drum_roles_do_not_require_poly() {
    let cats = categories(&["Percussion"]);
    let kick = categories(&["kick"]);
    let filter = RoleFilter::with_keywords(PatchRole::Kick, &cats, &kick);

    assert!(matches_role(
        "patches_factory/Percussion/Kick Tech 1.fxp",
        "patches_factory/percussion/kick tech 1.fxp",
        &filter,
        &PolyByName
    ));
}

/// キーワードが空なら、kick / snare / hi-hat はカテゴリだけで絞る。
#[test]
fn drum_roles_without_keywords_fall_back_to_the_category() {
    let all = drum_pairs();
    let cats = categories(&["Percussion"]);

    assert_eq!(
        matching(
            &all,
            &RoleFilter::with_keywords(PatchRole::Kick, &cats, &[])
        ),
        [
            "patches_factory/Percussion/Kick 909ish.fxp",
            "patches_factory/Percussion/Snare Tight.fxp",
        ]
    );
}
