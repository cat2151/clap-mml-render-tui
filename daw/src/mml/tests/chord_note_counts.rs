//! chord 行から生成した MML が、**実際に SMF のノートになる**ことの統合確認。
//!
//! `mml::tests::chord_track` が「どんな MML 文字列になるか」を固定しているのに対し、
//! こちらは**その MML を演奏経路と同じ `cmrt_core::mml_to_smf_bytes` に通して
//! NoteOn を数える**。MML パーサは未知の文字を黙って捨ててエラーにしないので、
//! 「変換が Ok だった」だけでは音が出る保証にならない（`I` は MML として解釈すると
//! ノート 0 個になる）。数えるところまでやって初めて「鳴る」と言える。
//!
//! 期待値はすべて、ローカルの `mmlabc-to-smf.exe --no-play -o x.mid "<MML>"` で
//! 実際に SMF を作り、その中身を読んで得た値をそのまま書いている（推測で書かない）。
//!
//! ```text
//! $ mmlabc-to-smf.exe --no-play -o i.mid "t120v11/*|*/'c1eg'/*|*/"
//!   → NoteOn 3 個: (tick 0, 60) (tick 0, 64) (tick 0, 67)
//! ```
//!
//! 分解能は 480 tick / 4 分音符。全音符 = 1920 tick なので、
//! 1 小節に 2 コードなら 2 つ目は tick 960、3 コードなら 640 / 1280 に来る。

use midly::{MidiMessage, Smf, TrackEventKind};

use super::chord_track::{data_with_chord_row, generate_init, GENERATED_TRACK, OTHER_TRACK};
use crate::mml::{build_cell_mml_from_data, build_measure_mml_from_data};
use crate::{MEASURES, TRACKS};

/// 最終 MML を**演奏と同じ経路**（`cmrt_core::mml_to_smf_bytes`）で SMF にし、
/// NoteOn を `(絶対 tick, note number)` の昇順リストで返す。
///
/// `;` で分岐した MML は複数の MIDI track になるので、track をまたいで集めて並べ直す。
pub(crate) fn note_ons(mml: &str) -> Vec<(u32, u8)> {
    let bytes = cmrt_core::mml_to_smf_bytes(mml).expect("MML から SMF へ変換できること");
    let smf = Smf::parse(&bytes).expect("SMF として読めること");

    let mut note_ons = Vec::new();
    for track in &smf.tracks {
        let mut tick = 0_u32;
        for event in track {
            tick = tick.saturating_add(event.delta.as_int());
            if let TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } = event.kind
            {
                // velocity 0 の NoteOn は NoteOff の別表記。
                if vel.as_int() > 0 {
                    note_ons.push((tick, key.as_int()));
                }
            }
        }
    }
    note_ons.sort_unstable();
    note_ons
}

/// 生成対象 track のセル (track, measure) を、演奏用 MML にしてから SMF にする。
fn generated_note_ons(data: &[Vec<String>], measure: usize) -> Vec<(u32, u8)> {
    note_ons(&build_cell_mml_from_data(
        data,
        MEASURES,
        GENERATED_TRACK,
        measure,
    ))
}

#[test]
fn one_degree_sounds_as_three_simultaneous_notes() {
    let mut data = data_with_chord_row("", &[(1, "I")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    // 手書きは空。それでも chord 行から 3 音が鳴る。
    assert_eq!(
        generated_note_ons(&data, 1),
        vec![(0, 60), (0, 64), (0, 67)]
    );
}

#[test]
fn two_degrees_sound_as_six_notes_split_inside_the_measure() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    assert_eq!(
        generated_note_ons(&data, 1),
        vec![(0, 60), (0, 64), (0, 67), (960, 65), (960, 69), (960, 72),]
    );
}

#[test]
fn three_degrees_sound_as_nine_notes() {
    let mut data = data_with_chord_row("", &[(1, "I-IV-V")]);
    data[GENERATED_TRACK][0] = generate_init("");

    assert_eq!(
        generated_note_ons(&data, 1),
        vec![
            (0, 60),
            (0, 64),
            (0, 67),
            (640, 65),
            (640, 69),
            (640, 72),
            (1280, 67),
            (1280, 71),
            (1280, 74),
        ]
    );
}

#[test]
fn the_key_on_the_chord_row_transposes_the_sounding_notes() {
    let mut data = data_with_chord_row("key:G", &[(1, "I")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    // C の 60/64/67 が G の 67/71/74 になる。
    assert_eq!(
        generated_note_ons(&data, 1),
        vec![(0, 67), (0, 71), (0, 74)]
    );
}

#[test]
fn a_voicing_directive_moves_the_notes_without_losing_any() {
    let mut data = data_with_chord_row("", &[(1, "I")]);
    data[GENERATED_TRACK][0] = generate_init("octave down");

    // 音数は変わらず 1 オクターブ下がるだけ。
    assert_eq!(
        generated_note_ons(&data, 1),
        vec![(0, 48), (0, 52), (0, 55)]
    );
}

#[test]
fn an_empty_chord_row_cell_sounds_nothing() {
    let mut data = data_with_chord_row("", &[(1, "I")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    // measure 2 の chord 行は空。無音になるだけで、変換は壊れない。
    assert!(generated_note_ons(&data, 2).is_empty());
}

#[test]
fn a_broken_chord_row_cell_sounds_nothing_instead_of_a_wrong_note() {
    let mut data = data_with_chord_row("", &[(1, "???")]);
    data[GENERATED_TRACK][0] = generate_init("close");

    assert!(generated_note_ons(&data, 1).is_empty());
}

#[test]
fn a_handwritten_cell_sounds_its_own_notes_instead_of_the_chord() {
    let mut data = data_with_chord_row("", &[(1, "I")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    data[GENERATED_TRACK][1] = "cde".to_string();

    // 和音（同時 3 音）ではなく、手書きの単音 3 つが順に鳴る。
    assert_eq!(
        generated_note_ons(&data, 1),
        vec![(0, 60), (240, 62), (480, 64)]
    );
}

#[test]
fn the_measure_mml_sounds_the_generated_and_the_handwritten_track_together() {
    let mut data = data_with_chord_row("", &[(1, "I-IV")]);
    data[GENERATED_TRACK][0] = generate_init("close");
    data[OTHER_TRACK][1] = "cde".to_string();

    let mml = build_measure_mml_from_data(&data, MEASURES, TRACKS, 1, &[false; TRACKS]);

    // 生成 track の 6 音 + 手書き track の 3 音。
    let sounding = note_ons(&mml);
    assert_eq!(sounding.len(), 9);
    assert_eq!(
        sounding,
        vec![
            (0, 60),
            (0, 60),
            (0, 64),
            (0, 67),
            (240, 62),
            (480, 64),
            (960, 65),
            (960, 69),
            (960, 72),
        ]
    );
}

#[test]
fn the_generate_key_in_the_json_prefix_does_not_swallow_any_note() {
    let mut data = data_with_chord_row("", &[(1, "I")]);
    data[GENERATED_TRACK][0] =
        r#"{"Surge XT patch": "piano", "generate from chord track": "close"}"#.to_string();

    // 最終 MML の先頭 JSON に生成キーが残っていても、ノートは 3 つとも残る。
    assert_eq!(
        generated_note_ons(&data, 1),
        vec![(0, 60), (0, 64), (0, 67)]
    );
}
