use super::*;

use cmrt_tui_core::patch_plugins::CatalogPlugin;

/// 行→用途が診断表示の8用途だけを返すことの番人。
#[test]
fn every_row_role_appears_in_the_role_table() {
    let all = [
        GridPatchPurpose::Note,
        GridPatchPurpose::Chord,
        GridPatchPurpose::Bass,
        GridPatchPurpose::Arpeggio,
        GridPatchPurpose::Kick,
        GridPatchPurpose::Snare,
        GridPatchPurpose::HiHat,
        GridPatchPurpose::Percussion,
    ];
    for chord_on in [true, false] {
        for row in 0..FULL_DRUM_TRACK_COUNT {
            let drum = row
                .checked_sub(FIRST_DRUM_ROW)
                .and_then(|index| DrumRole::ALL.get(index))
                .copied();
            let role = row_patch_purpose(row, chord_on, drum);
            assert!(
                all.contains(&role),
                "chord_on={chord_on} row={row} の用途 {role:?} が診断表に無い"
            );
        }
    }
}

/// chord mode を切ると先頭3行はNOTEに戻るが、drum 行の用途は変わらない。
#[test]
fn drum_rows_keep_their_role_when_chord_mode_is_off() {
    assert_eq!(
        row_patch_purpose(CHORD_ROW, true, None),
        GridPatchPurpose::Chord
    );
    assert_eq!(
        row_patch_purpose(CHORD_ROW, false, None),
        GridPatchPurpose::Note
    );
    assert_eq!(
        row_patch_purpose(BASS_ROW, true, None),
        GridPatchPurpose::Bass
    );
    assert_eq!(
        row_patch_purpose(BASS_ROW, false, None),
        GridPatchPurpose::Note
    );
    assert_eq!(
        row_patch_purpose(ARPEGGIO_ROW, true, None),
        GridPatchPurpose::Arpeggio
    );
    assert_eq!(
        row_patch_purpose(ARPEGGIO_ROW, false, None),
        GridPatchPurpose::Note
    );
    for chord_on in [true, false] {
        assert_eq!(
            row_patch_purpose(FIRST_DRUM_ROW, chord_on, Some(DrumRole::Kick)),
            GridPatchPurpose::Kick
        );
    }
}

#[test]
fn the_sample_column_is_blank_when_there_is_no_candidate() {
    assert_eq!(sample_label(&[]), "");
    assert_eq!(sample_label(&["a"]), "例: a");
    assert_eq!(sample_label(&["a", "b"]), "例: a | b");
    assert_eq!(sample_label(&["a", "b", "c"]), "例: a | b ...");
}

#[test]
fn unset_values_are_shown_as_unset() {
    assert_eq!(optional(None), "(未設定)");
    assert_eq!(optional(Some("Dexed")), "Dexed");
    assert_eq!(optional_str(""), "(未設定)");
    assert_eq!(optional_str("x.clap"), "x.clap");
}

/// 内訳はカタログの並び順どおりに、1 件も落とさず 1 件も重複させずに数える。
///
/// 「Vaporizer2 の候補が 0 件」を見つけるための欄なので、**0 件のプラグインも
/// 省略せずに出す**（省略すると「行が無い」と「数えていない」の区別が付かない）。
#[test]
fn the_breakdown_counts_every_candidate_once_per_plugin() {
    use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID, VAPORIZER2_PLUGIN_ID};

    let plugin = |name: &str, plugin_id: &str| CatalogPlugin {
        name: name.to_string(),
        plugin_path: String::new(),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let patch_plugins = PatchPlugins::from_catalog(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Vaporizer2", VAPORIZER2_PLUGIN_ID),
    ]);

    let counts = per_plugin_counts(
        &[
            "Pads/Pad 1.fxp",
            "AR Accent Arp.vvp",
            "PD Emily.vvp",
            "Dexed_01.syx/00 Bell",
        ],
        &patch_plugins,
    );

    assert_eq!(counts, "Surge XT 1 / Dexed 1 / Vaporizer2 2");
    // 候補が 0 件でも、プラグインの行は消えない。
    assert_eq!(
        per_plugin_counts(&[], &patch_plugins),
        "Surge XT 0 / Dexed 0 / Vaporizer2 0"
    );
}

/// 設定不足でカタログから外れたプラグインは、名前と直し方つきで欄に出る。
///
/// **0 件のときも欄は空にしない。** 「外れたものは無い」と「そもそも数えていない」を
/// 区別できないと、この診断で切り分けたい当のことが分からなくなる。
#[test]
fn the_report_lists_skipped_plugins_even_when_there_are_none() {
    use cmrt_runtime::{CatalogSkipReason, SkippedCatalogPlugin};

    let none = skipped_section_lines(&[]);
    assert_eq!(none.len(), 1);
    assert!(none[0].contains("なし"), "{}", none[0]);

    let lines = skipped_section_lines(&[SkippedCatalogPlugin {
        name: "Vaporizer2".to_string(),
        reason: CatalogSkipReason::NoPatchDirs,
    }]);

    assert_eq!(lines.len(), 1);
    // 文言は cmrt_runtime 側が単一ソース。ここでは「素通ししている」ことだけ見る。
    assert_eq!(
        lines[0],
        SkippedCatalogPlugin {
            name: "Vaporizer2".to_string(),
            reason: CatalogSkipReason::NoPatchDirs,
        }
        .notice_line()
    );
}
