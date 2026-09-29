use super::track_patch_display;
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

/// 音色と「chord 行から生成する」指定の両方が入った init セル。
fn generated_patch_cell(display: &str, directive: &str) -> String {
    format!(r#"{{"Surge XT patch": "{display}", "generate from chord track": "{directive}"}}"#)
}

const BASS: &str = "patches_factory/Basses/Wobble Bass.fxp";

#[test]
fn a_patch_track_reports_its_role_and_patch_name_separately() {
    let snapshot = snapshot_with(&[BASS]);
    let display = track_patch_display(&patch_cell(BASS), Some(&snapshot));

    assert_eq!(display.role_label(), "bass");
    assert_eq!(display.patch_label(), "Wobble Bass");
    assert_eq!(
        display.init_cell_label().as_deref(),
        Some("bass:Wobble Bass")
    );
}

#[test]
fn a_loading_catalog_still_reports_the_patch_name() {
    // catalog が Loading / Err のときは role だけが引けない。音色名は init セルから読める。
    let display = track_patch_display(&patch_cell(BASS), None);

    assert_eq!(display.role_label(), "---");
    assert_eq!(display.patch_label(), "Wobble Bass");
}

#[test]
fn an_unclassified_patch_reports_the_patch_name_without_a_role() {
    let snapshot = snapshot_with(&[BASS]);
    let display = track_patch_display(
        &patch_cell("not_in_the_catalog/Mystery Thing.fxp"),
        Some(&snapshot),
    );

    assert_eq!(display.role_label(), "---");
    assert_eq!(display.patch_label(), "Mystery Thing");
}

#[test]
fn a_shortened_patch_name_is_resolved_before_looking_up_the_role() {
    let snapshot = snapshot_with(&[BASS]);
    let display = track_patch_display(&patch_cell("Basses/Wobble Bass.fxp"), Some(&snapshot));

    assert_eq!(display.role_label(), "bass");
}

#[test]
fn a_plain_mml_cell_has_neither_a_role_nor_a_patch_name() {
    let snapshot = snapshot_with(&[BASS]);
    let display = track_patch_display("cdefgab", Some(&snapshot));

    assert_eq!(display.role_label(), "---");
    assert_eq!(display.patch_label(), "---");
    assert_eq!(display.init_cell_label(), None);
}

#[test]
fn an_empty_patch_name_counts_as_no_patch() {
    let display = track_patch_display(&patch_cell("   "), None);

    assert_eq!(display.patch_label(), "---");
    assert_eq!(display.init_cell_label(), None);
}

#[test]
fn a_generated_track_marks_its_role_with_the_generated_mark() {
    let snapshot = snapshot_with(&[BASS]);
    let display = track_patch_display(&generated_patch_cell(BASS, "close"), Some(&snapshot));

    assert_eq!(display.role_label(), "*bass");
    assert_eq!(display.patch_label(), "Wobble Bass");
    assert_eq!(display.generated_directive(), Some("close"));
}

#[test]
fn a_generated_track_without_a_patch_shows_its_directive_as_the_role() {
    let display = track_patch_display(r#"{"generate from chord track": "octave down"}"#, None);

    assert_eq!(display.role_label(), "*octave down");
    assert_eq!(display.patch_label(), "---");
}

#[test]
fn a_generated_track_with_an_empty_directive_still_says_it_is_generated() {
    let display = track_patch_display(r#"{"generate from chord track": ""}"#, None);

    assert_eq!(display.role_label(), "*chord");
    assert_eq!(display.generated_directive(), Some(""));
}
