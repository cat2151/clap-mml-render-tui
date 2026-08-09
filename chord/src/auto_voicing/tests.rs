use super::*;

use crate::chord_notes;

/// 素の root position（chord2mml の出力そのまま）の top note 最大跳躍。
fn raw_max_top_jump(chords: &[Vec<u8>]) -> u8 {
    let tops = chords
        .iter()
        .map(|notes| *notes.iter().max().expect("chord is not empty"))
        .collect::<Vec<_>>();
    tops.windows(2)
        .map(|pair| pair[0].abs_diff(pair[1]))
        .max()
        .unwrap_or(0)
}

fn pitch_classes(notes: &[u8]) -> Vec<u8> {
    let mut classes = notes.iter().map(|note| note % 12).collect::<Vec<_>>();
    classes.sort_unstable();
    classes.dedup();
    classes
}

#[test]
fn top_note_jumps_shrink_against_the_raw_root_position() {
    let chords = chord_notes("I-IV-V-I", "C").unwrap();
    // 素のまま鳴らすと top は 67 → 72 → 74 → 67 で最大7半音跳ぶ。
    assert_eq!(raw_max_top_jump(&chords), 7);

    let voiced = auto_voice(&chords, None);
    let (top_jump, _) = max_jumps(&voiced);
    assert!(
        top_jump <= 4,
        "top note の跳躍が縮んでいない: {top_jump} ({voiced:?})"
    );
}

#[test]
fn every_catalog_progression_keeps_its_pitch_classes() {
    for degrees in [
        "I-IV-V-I",
        "IIm7-V7-IM7",
        "Im-bVII-bVI-bVII",
        "IM7-VIm7-IIm7-V7",
    ] {
        let chords = chord_notes(degrees, "C").unwrap();
        let voiced = auto_voice(&chords, None);
        assert_eq!(voiced.len(), chords.len(), "{degrees}");
        for (index, (original, result)) in chords.iter().zip(&voiced).enumerate() {
            assert_eq!(
                pitch_classes(original),
                pitch_classes(&result.notes),
                "{degrees} の {index} 番目で構成音のピッチクラスが変わった"
            );
            let mut sorted = result.notes.clone();
            sorted.sort_unstable();
            assert_eq!(
                sorted, result.notes,
                "{degrees} の {index} 番目が昇順でない"
            );
        }
    }
}

#[test]
fn the_bass_is_a_separate_note_inside_its_own_range() {
    let chords = chord_notes("I-IV-V-I", "C").unwrap();
    for voicing in auto_voice(&chords, None) {
        let bass = voicing.bass.expect("bass が作られていない");
        assert!(
            (24..=64).contains(&bass),
            "bass が音域から外れている: {bass}"
        );
        assert!(
            bass < *voicing.notes.iter().min().unwrap(),
            "bass が和音の最低音より上にある: {bass} < {:?}",
            voicing.notes
        );
    }
}

#[test]
fn semitone_clusters_are_avoided_in_the_chosen_voicing() {
    // 転回すると 71-72 のぶつかりが出る IM7。penalty が効けば選ばれない。
    let voiced = auto_voice(&[vec![60, 64, 67, 71]], None);
    assert_eq!(count_semitone_intervals(&voiced[0].notes), 0, "{voiced:?}");
}

#[test]
fn a_seed_pulls_the_first_chord_toward_the_sounding_top_note() {
    let chords = chord_notes("IV-V", "C").unwrap();
    let low_seed = ChordVoicing {
        bass: Some(48),
        notes: vec![60, 64, 67],
    };
    let high_seed = ChordVoicing {
        bass: Some(48),
        notes: vec![72, 76, 79],
    };

    let low_top = top_of(&auto_voice(&chords, Some(&low_seed))[0]).unwrap();
    let high_top = top_of(&auto_voice(&chords, Some(&high_seed))[0]).unwrap();
    assert!(
        low_top < high_top,
        "seed の top note に引き寄せられていない: low={low_top} high={high_top}"
    );
}

#[test]
fn empty_input_is_passed_through_without_a_bass() {
    assert!(auto_voice(&[], None).is_empty());
    let passed = auto_voice(&[vec![60, 64, 67], Vec::new()], None);
    assert_eq!(passed.len(), 2);
    assert!(passed.iter().all(|voicing| voicing.bass.is_none()));
    assert_eq!(passed[0].notes, vec![60, 64, 67]);
}

#[test]
fn max_jumps_reports_the_widest_step_of_both_parts() {
    let voicings = vec![
        ChordVoicing {
            bass: Some(48),
            notes: vec![60, 64, 67],
        },
        ChordVoicing {
            bass: Some(53),
            notes: vec![60, 65, 69],
        },
        ChordVoicing {
            bass: Some(43),
            notes: vec![62, 67, 71],
        },
    ];
    assert_eq!(max_jumps(&voicings), (2, 10));
}
