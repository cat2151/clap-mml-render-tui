use super::*;

use cmrt_tui_core::patch_plugins::CatalogPlugin;

/// 表に出す 8 役割が、行が取りうる用途を1つも取りこぼしていないことの番人。
///
/// 行→用途の対応（[`row_patch_role`]）が増えたのに [`ALL_ROLES`] を足し忘れると、
/// 「候補が0件の行がある」ことを報告できないまま素通りする。
#[test]
fn every_row_role_appears_in_the_role_table() {
    for chord_on in [true, false] {
        for row in 0..FULL_DRUM_TRACK_COUNT {
            let drum = row
                .checked_sub(FIRST_DRUM_ROW)
                .and_then(|index| DrumRole::ALL.get(index))
                .copied();
            let role = row_patch_role(row, chord_on, drum);
            assert!(
                ALL_ROLES.contains(&role),
                "chord_on={chord_on} row={row} の用途 {role:?} が ALL_ROLES に無い"
            );
        }
    }
}

/// chord mode を切ると先頭3行は Free に戻るが、drum 行の用途は変わらない。
#[test]
fn drum_rows_keep_their_role_when_chord_mode_is_off() {
    assert_eq!(row_patch_role(CHORD_ROW, true, None), PatchRole::Chord);
    assert_eq!(row_patch_role(CHORD_ROW, false, None), PatchRole::Free);
    assert_eq!(row_patch_role(BASS_ROW, true, None), PatchRole::Bass);
    assert_eq!(row_patch_role(BASS_ROW, false, None), PatchRole::Free);
    assert_eq!(
        row_patch_role(ARPEGGIO_ROW, true, None),
        PatchRole::Arpeggio
    );
    assert_eq!(row_patch_role(ARPEGGIO_ROW, false, None), PatchRole::Free);
    for chord_on in [true, false] {
        assert_eq!(
            row_patch_role(FIRST_DRUM_ROW, chord_on, Some(DrumRole::Kick)),
            PatchRole::Kick
        );
    }
}

#[test]
fn an_empty_filter_reads_as_unfiltered() {
    assert_eq!(list_label(&[]), "(空 = 絞らない)");
    assert_eq!(
        list_label(&["Keys".to_string(), "Pads".to_string()]),
        "Keys, Pads"
    );
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
    use cmrt_runtime::{PatchRoles, DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID, VAPORIZER2_PLUGIN_ID};

    let plugin = |name: &str, plugin_id: &str| CatalogPlugin {
        name: name.to_string(),
        plugin_path: String::new(),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        patch_roles: PatchRoles::default(),
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
