use super::*;

/// 単音だけの列から、ピッキングする列を `●`、H/P を `・` で並べる。
fn marks(pitches: &[u8]) -> String {
    let pitches: Vec<Option<u8>> = pitches.iter().copied().map(Some).collect();
    run_picks(&pitches)
        .iter()
        .map(|&pick| if pick { '●' } else { '・' })
        .collect()
}

#[test]
fn four_note_runs_pick_the_apex_and_the_valley() {
    // C C# D D# D C# C
    assert_eq!(marks(&[60, 61, 62, 63, 62, 61, 60]), "●・・●・・●");
}

#[test]
fn six_note_runs_pick_every_three_notes_and_restart_at_the_turn() {
    // C D E F G A G F E D C
    assert_eq!(
        marks(&[60, 62, 64, 65, 67, 69, 67, 65, 64, 62, 60]),
        "●・・●・●・・●・●"
    );
}

#[test]
fn five_note_run_picks_the_fourth_note_and_the_apex() {
    assert_eq!(marks(&[60, 62, 64, 65, 67]), "●・・●●");
}

#[test]
fn trill_and_three_note_runs_stay_legato() {
    assert_eq!(marks(&[60, 62, 60, 62, 60]), "・・・・・");
    assert_eq!(marks(&[60, 62, 64, 62, 60]), "・・・・・");
}

#[test]
fn repeated_pitch_breaks_the_run() {
    // 60 62 64 | 64 65 67 はどちらも 3 音。
    assert_eq!(marks(&[60, 62, 64, 64, 65, 67]), "・・・・・・");
}

#[test]
fn chord_breaks_the_run() {
    let pitches = [Some(60), Some(61), None, Some(62), Some(63)];
    assert_eq!(run_picks(&pitches), vec![false; 5]);
}

#[test]
fn string_changes_are_still_picked() {
    // 40〜42 と 45 の 2 本。上行は 3 音なので弦の最初の音だけ。
    let notes: Vec<Note> = [40, 42, 45]
        .into_iter()
        .enumerate()
        .map(|(column, pitch)| Note {
            on_seconds: column as f64 * 0.5,
            off_seconds: column as f64 * 0.5 + 0.5,
            channel: 0,
            pitch,
            velocity: 100,
            column,
        })
        .collect();
    assert_eq!(auto_pick_columns(&notes), vec![true, false, true]);
}
