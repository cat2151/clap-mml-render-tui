use super::*;

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
