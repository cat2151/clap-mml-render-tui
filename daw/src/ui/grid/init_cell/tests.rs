use super::{init_cell_text, init_indicator_text, patch_stem};
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
        init_cell_text(2, &patch_cell(bass), Some(&snapshot)).as_deref(),
        Some("bass:Wobble Bass")
    );
    assert_eq!(
        init_cell_text(3, &patch_cell(lead), Some(&snapshot)).as_deref(),
        Some("lead:Screaming Lead")
    );
}

#[test]
fn patch_track_shows_the_patch_name_alone_while_the_catalog_is_loading() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";

    assert_eq!(
        init_cell_text(2, &patch_cell(bass), None).as_deref(),
        Some("Wobble Bass")
    );
}

#[test]
fn patch_track_shows_the_patch_name_alone_when_the_role_is_unknown() {
    let snapshot = snapshot_with(&["patches_factory/Basses/Wobble Bass.fxp"]);

    assert_eq!(
        init_cell_text(
            2,
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
        init_cell_text(2, &patch_cell("Basses/Wobble Bass.fxp"), Some(&snapshot)).as_deref(),
        Some("bass:Wobble Bass")
    );
}

#[test]
fn patch_track_falls_back_to_the_raw_mml_for_a_plain_mml_cell() {
    let snapshot = snapshot_with(&["patches_factory/Basses/Wobble Bass.fxp"]);

    assert_eq!(init_cell_text(2, "cdefgab", Some(&snapshot)), None);
}

// ─── chord 行から生成される track の init 列 ───────────────────

const GENERATED_TRACK: usize = crate::FIRST_PLAYABLE_TRACK;

/// 音色と「chord 行から生成する」指定の両方が入った init セル。
fn generated_patch_cell(display: &str, directive: &str) -> String {
    format!(r#"{{"Surge XT patch": "{display}", "generate from chord track": "{directive}"}}"#)
}

#[test]
fn a_generated_track_marks_its_init_cell_before_the_patch_name() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";
    let snapshot = snapshot_with(&[bass]);

    assert_eq!(
        init_cell_text(
            GENERATED_TRACK,
            &generated_patch_cell(bass, "close"),
            Some(&snapshot)
        )
        .as_deref(),
        Some("*bass:Wobble Bass")
    );
}

#[test]
fn a_track_without_the_generate_key_keeps_its_unmarked_init_cell() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";
    let snapshot = snapshot_with(&[bass]);

    assert_eq!(
        init_cell_text(GENERATED_TRACK, &patch_cell(bass), Some(&snapshot)).as_deref(),
        Some("bass:Wobble Bass")
    );
}

#[test]
fn a_generated_track_without_a_patch_shows_the_directive_instead_of_the_raw_json() {
    assert_eq!(
        init_cell_text(
            GENERATED_TRACK,
            r#"{"generate from chord track": "octave down"}"#,
            None
        )
        .as_deref(),
        Some("*octave down")
    );
}

#[test]
fn a_generated_track_with_an_empty_directive_still_says_it_is_generated() {
    assert_eq!(
        init_cell_text(
            GENERATED_TRACK,
            r#"{"generate from chord track": ""}"#,
            None
        )
        .as_deref(),
        Some("*chord")
    );
}

#[test]
fn the_indicator_row_shows_the_directive_that_goes_to_chord2mml() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";

    assert_eq!(
        init_indicator_text(GENERATED_TRACK, &generated_patch_cell(bass, "1st inv")).as_deref(),
        Some("1st inv")
    );
}

#[test]
fn the_indicator_row_says_chord_when_the_directive_is_empty() {
    assert_eq!(
        init_indicator_text(GENERATED_TRACK, r#"{"generate from chord track": ""}"#).as_deref(),
        Some("chord")
    );
}

#[test]
fn the_indicator_row_stays_empty_for_a_track_that_is_not_generated() {
    let bass = "patches_factory/Basses/Wobble Bass.fxp";

    assert_eq!(
        init_indicator_text(GENERATED_TRACK, &patch_cell(bass)),
        None
    );
    assert_eq!(init_indicator_text(GENERATED_TRACK, "cdefgab"), None);
    assert_eq!(
        init_indicator_text(crate::tracks::TEMPO_TRACK, r#"{"beat": "4/4"}t120"#),
        None
    );
}

/// chord 行の init セルは chord2mml への指定文字列（`key:G` など）。
/// 組み直さず、書いたとおりに出す（`None` = 呼び出し側が生の文字列を詰める）。
#[test]
fn the_chord_row_init_cell_is_shown_exactly_as_written() {
    assert_eq!(init_cell_text(crate::CHORD_TRACK, "key:G", None), None);
    assert_eq!(
        init_indicator_text(crate::CHORD_TRACK, "key:G octave down"),
        None
    );
}
