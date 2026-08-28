use super::{init_cell_text, patch_stem};
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;

/// role が引ける音色を持つ snapshot。`PatchRoleIndex` は表示パスから分類する。
fn snapshot_with(displays: &[&str]) -> PatchCatalogSnapshot {
    PatchCatalogSnapshot::from_pairs(
        displays
            .iter()
            .map(|display| {
                (
                    (*display).to_string(),
                    cmrt_patches::normalize_patch_lookup_key(display),
                )
            })
            .collect(),
    )
}

fn patch_cell(display: &str) -> String {
    format!(r#"{{"Surge XT patch": "{display}"}}"#)
}

#[test]
fn patch_stem_drops_only_known_patch_file_extensions() {
    assert_eq!(
        patch_stem("patches_3rdparty/Dan Maurer/Winds/Reed To Pipe Morph.fxp"),
        "Reed To Pipe Morph"
    );
    assert_eq!(patch_stem("OR Organ Rotary 2.vvp"), "OR Organ Rotary 2");
    assert_eq!(
        patch_stem("sfz/Virtual-Playing-Orchestra3/Brass/trombone-SOLO.sfz"),
        "trombone-SOLO"
    );
    // Dexed のカートリッジ内音色は拡張子を持たない。ドットを含む名前を欠けさせないこと。
    assert_eq!(
        patch_stem("SynprezFM/SynprezFM_22.syx/05 SampleSqr2"),
        "05 SampleSqr2"
    );
    assert_eq!(
        patch_stem("SynprezFM/SynprezFM_01.syx/05 T.BL-EXPA"),
        "05 T.BL-EXPA"
    );
}

#[test]
fn tempo_track_shows_the_beat_and_the_tempo() {
    assert_eq!(
        init_cell_text(0, r#"{"beat": "4/4"}t120"#, None).as_deref(),
        Some("4/4 t120")
    );
}

#[test]
fn tempo_track_without_a_tempo_shows_the_beat_alone() {
    assert_eq!(
        init_cell_text(0, r#"{"beat": "3/4"}"#, None).as_deref(),
        Some("3/4")
    );
}

#[test]
fn tempo_track_falls_back_to_the_raw_mml_when_the_beat_is_missing() {
    assert_eq!(init_cell_text(0, "t120", None), None);
}

#[test]
fn patch_track_shows_the_role_and_the_patch_name() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";
    let lead = "patches_factory/Leads/Screaming Lead.fxp";
    let snapshot = snapshot_with(&[bass, lead]);

    assert_eq!(
        init_cell_text(1, &patch_cell(bass), Some(&snapshot)).as_deref(),
        Some("bass:Wobble Bass")
    );
    assert_eq!(
        init_cell_text(2, &patch_cell(lead), Some(&snapshot)).as_deref(),
        Some("lead:Screaming Lead")
    );
}

#[test]
fn patch_track_shows_the_patch_name_alone_while_the_catalog_is_loading() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";

    assert_eq!(
        init_cell_text(1, &patch_cell(bass), None).as_deref(),
        Some("Wobble Bass")
    );
}

#[test]
fn patch_track_shows_the_patch_name_alone_when_the_role_is_unknown() {
    let snapshot = snapshot_with(&["patches_factory/Basses/Wobble Bass.fxp"]);

    assert_eq!(
        init_cell_text(
            1,
            &patch_cell("not_in_the_catalog/Mystery Thing.fxp"),
            Some(&snapshot)
        )
        .as_deref(),
        Some("Mystery Thing")
    );
}

#[test]
fn patch_track_resolves_a_shortened_patch_name_before_looking_up_the_role() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";
    let snapshot = snapshot_with(&[bass]);

    assert_eq!(
        init_cell_text(1, &patch_cell("Basses/Wobble Bass.fxp"), Some(&snapshot)).as_deref(),
        Some("bass:Wobble Bass")
    );
}

#[test]
fn patch_track_falls_back_to_the_raw_mml_for_a_plain_mml_cell() {
    let snapshot = snapshot_with(&["patches_factory/Basses/Wobble Bass.fxp"]);

    assert_eq!(init_cell_text(1, "cdefgab", Some(&snapshot)), None);
}
