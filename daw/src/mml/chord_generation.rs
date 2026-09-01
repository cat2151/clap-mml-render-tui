//! chord 行のセルから、演奏 track で鳴らす MML を生成する。
//!
//! # 何をどう組み立てるか
//!
//! chord2mml へ渡す 1 小節ぶんの入力は、次の順で連結する。
//!
//! ```text
//! {chord 行の init セル} {track の init セルの指定} | {chord 行の当該 measure セル} |
//! ```
//!
//! - **conductor（track 0）は連結しない。** `t120` のような MML の body を混ぜると
//!   chord2mml が必ず `Syntax error` になる（実測済み）。
//! - **半角の縦棒で囲む。** 囲まないと 1 コード = 全音符になり、1 小節に 2 コード書くと
//!   2 小節ぶんの長さに溢れる。囲むと小節内で自動等分される。
//! - 連結順が「chord 行 init → track init」なのは、chord2mml の `key:` が**後勝ち**だから。
//!   曲全体の key は chord 行 init に書き、その track だけ変えたければ track init に書く、
//!   という 1 つのルールで説明できる。
//!
//! chord 行のセルが空の小節は、convert を呼ばずに空 MML を返す
//! （chord2mml は空入力を `Err` にするため）。
//!
//! # ここで使う API
//!
//! **`cmrt_chord::parse_chord_progression` を使ってはいけない。**
//! あちらは「key は先頭に 1 つ必須」という grid sequencer 用の制約を足した API で、
//! key 省略時は暗黙に C とするこの画面の仕様と衝突する。使うのは
//! `chord2mml_core::convert` 直。

use cmrt_chord::ChordVoicing;

use crate::grid_import::{
    DawGridChordBinding, DawGridChordGeneration, DawGridLane, DawGridNoteStep,
};

const GRID_STEPS: usize = 16;
const CHORD_VOICE_LIMIT: usize = 4;

/// 演奏 track の init セルの JSON に置く、生成対象であることを表すキー。
///
/// **このキーが存在すること**だけが判定で、値は chord2mml へ渡す任意の文字列。
pub(crate) const GENERATE_FROM_CHORD_TRACK_KEY: &str = "generate from chord track";

/// chord2mml へ渡す入力文字列を組み立てる。
///
/// chord 行の当該セルが空なら `None`（変換にかけるものが無い）。
///
/// - `chord_init`: chord 行の init セル（measure 0）。`key:G` などの chord2mml 文字列。
/// - `track_directive`: 演奏 track の init セルの `"generate from chord track"` の値。
///   `close` / `drop2` / `octave down` / `key:B` など、chord2mml が認識する任意の文字列。
///   **cmrt 側では検証しない**（語彙を持たない）。空文字でもよい。
/// - `chord_cell`: chord 行の当該 measure のセル。`I-IV-V` など。
pub(crate) fn chord2mml_input(
    chord_init: &str,
    track_directive: &str,
    chord_cell: &str,
) -> Option<String> {
    cmrt_chord::chord_cell_input(chord_init, track_directive, chord_cell)
}

/// chord 行のセルから、演奏 track の 1 小節ぶんの MML を生成する。
///
/// 空セルのときと、chord2mml が `Err` を返したときは空文字列を返す
/// （その小節がその track で無音になるだけで、演奏は止まらない）。
/// `Err` のときはログに残すが、この段階では UI には出さない。
pub(crate) fn generate_mml_from_chord_cell(
    chord_init: &str,
    track_directive: &str,
    chord_cell: &str,
) -> String {
    let Some(input) = chord2mml_input(chord_init, track_directive, chord_cell) else {
        return String::new();
    };

    match chord2mml_core::convert(&input) {
        Ok(mml) => mml,
        Err(error) => {
            crate::log_line(&format!(
                "chord track: convert failed: input={input:?} error={error}"
            ));
            String::new()
        }
    }
}

/// Grid import由来の生成レシピを、現在のchord trackから1小節のMMLへ解決する。
///
/// chordがimport時のままなら、snapshotに保存したexact voicingを使う。編集された
/// 小節は同じhintを接続相手としてauto voicingし、元の音域を保ちながら追従する。
pub(crate) fn generate_grid_mml_from_chord_cell(
    generation: &DawGridChordGeneration,
    chord_init: &str,
    chord_cell: &str,
    measure: usize,
) -> String {
    if chord_cell.trim().is_empty() || measure == 0 {
        return String::new();
    }
    let Some(voicings) = resolve_voicings(generation, chord_init, chord_cell, measure) else {
        return String::new();
    };
    match &generation.binding {
        DawGridChordBinding::Chord => chord_voicings_mml(&voicings),
        DawGridChordBinding::Bass { lanes }
        | DawGridChordBinding::Arpeggio { lanes, .. }
        | DawGridChordBinding::NearestChordTone { lanes } => {
            patterned_voicings_mml(&generation.binding, lanes, &voicings)
        }
    }
}

fn resolve_voicings(
    generation: &DawGridChordGeneration,
    chord_init: &str,
    chord_cell: &str,
    measure: usize,
) -> Option<Vec<ChordVoicing>> {
    let source_index = measure.checked_sub(1)?;
    let hint = generation
        .source
        .voicings
        .get(source_index)
        .map(|voicing| ChordVoicing {
            bass: voicing.bass,
            notes: voicing.notes.clone(),
        });
    let same_source = generation
        .source
        .measures
        .get(source_index)
        .is_some_and(|source| source.trim() == chord_cell.trim())
        && chord_key(chord_init) == chord_key(&generation.source.init);
    if same_source {
        return hint.map(|voicing| vec![voicing]);
    }

    let input = format!("{} {}", chord_key(chord_init), chord_cell.trim());
    let parsed = match cmrt_chord::parse_chord_progression(&input) {
        Ok(parsed) => parsed,
        Err(error) => {
            crate::log_line(&format!(
                "grid chord track: parse failed: input={input:?} error={error}"
            ));
            return None;
        }
    };
    let voicings = cmrt_chord::auto_voice(parsed.chords(), hint.as_ref());
    (!voicings.is_empty()).then_some(voicings)
}

/// Grid recipeはkeyだけを曲全体の指定として使う。その他のchord2mml directiveは
/// legacy文字列recipeの責務で、relative lane resolverへは混ぜない。
fn chord_key(chord_init: &str) -> &str {
    chord_init
        .split_whitespace()
        .find(|part| part.to_ascii_lowercase().starts_with("key:"))
        .unwrap_or("key:C")
}

fn chord_voicings_mml(voicings: &[ChordVoicing]) -> String {
    let voice_count = voicings
        .iter()
        .map(|voicing| voicing.notes.len())
        .max()
        .unwrap_or(0);
    (0..voice_count)
        .filter_map(|voice| {
            let mut mml = String::new();
            let mut step = 0;
            let mut sounding = false;
            while step < GRID_STEPS {
                let chord = chord_index(step, voicings.len());
                let end = chord_segment_end(step, voicings.len());
                if let Some(note) = voicings[chord].notes.get(voice).copied() {
                    append_span(&mut mml, &note_name(note), end - step);
                    sounding = true;
                } else {
                    append_span(&mut mml, "r", end - step);
                }
                step = end;
            }
            sounding.then_some(mml)
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn patterned_voicings_mml(
    binding: &DawGridChordBinding,
    lanes: &[DawGridLane],
    voicings: &[ChordVoicing],
) -> String {
    lanes
        .iter()
        .enumerate()
        .filter_map(|(lane_index, lane)| patterned_lane_mml(binding, lane_index, lane, voicings))
        .collect::<Vec<_>>()
        .join(";")
}

fn patterned_lane_mml(
    binding: &DawGridChordBinding,
    lane_index: usize,
    lane: &DawGridLane,
    voicings: &[ChordVoicing],
) -> Option<String> {
    let steps = normalized_steps(&lane.steps);
    let mut mml = String::new();
    let mut step = 0;
    let mut sounding = false;
    while step < GRID_STEPS {
        if steps[step] != DawGridNoteStep::Attack {
            let end = (step + 1..=GRID_STEPS)
                .find(|index| *index == GRID_STEPS || steps[*index] == DawGridNoteStep::Attack)
                .unwrap_or(GRID_STEPS);
            append_span(&mut mml, "r", end - step);
            step = end;
            continue;
        }

        let attack_end = (step + 1..GRID_STEPS)
            .take_while(|index| steps[*index] == DawGridNoteStep::Tie)
            .last()
            .map_or(step + 1, |last| last + 1);
        let mut segment = step;
        while segment < attack_end {
            let chord = chord_index(segment, voicings.len());
            let end = attack_end.min(chord_segment_end(segment, voicings.len()));
            if let Some(note) = binding_note(binding, lane_index, lane, &voicings[chord]) {
                append_span(&mut mml, &note_name(note), end - segment);
                sounding = true;
            } else {
                append_span(&mut mml, "r", end - segment);
            }
            segment = end;
        }
        step = attack_end;
    }
    sounding.then_some(mml)
}

fn binding_note(
    binding: &DawGridChordBinding,
    lane_index: usize,
    lane: &DawGridLane,
    voicing: &ChordVoicing,
) -> Option<u8> {
    match binding {
        DawGridChordBinding::Chord => None,
        DawGridChordBinding::Bass { .. } => cmrt_chord::bass_octave_note(voicing.bass, lane_index),
        DawGridChordBinding::Arpeggio { rotation, .. } => cmrt_chord::rotated_chord_voice(
            &voicing.notes,
            lane_index,
            *rotation,
            CHORD_VOICE_LIMIT,
        ),
        DawGridChordBinding::NearestChordTone { .. } => {
            let mut classes = [false; 12];
            for note in &voicing.notes {
                classes[usize::from(*note % 12)] = true;
            }
            Some(cmrt_chord::snap_to_chord(lane.base_note, &classes))
        }
    }
}

fn normalized_steps(source: &[DawGridNoteStep]) -> [DawGridNoteStep; GRID_STEPS] {
    let mut steps = [DawGridNoteStep::Rest; GRID_STEPS];
    for (target, source) in steps.iter_mut().zip(source.iter().copied()) {
        *target = source;
    }
    let mut sounding = false;
    for step in &mut steps {
        match *step {
            DawGridNoteStep::Rest => sounding = false,
            DawGridNoteStep::Attack => sounding = true,
            DawGridNoteStep::Tie if !sounding => *step = DawGridNoteStep::Rest,
            DawGridNoteStep::Tie => {}
        }
    }
    steps
}

fn chord_index(step: usize, chord_count: usize) -> usize {
    (step * chord_count / GRID_STEPS).min(chord_count.saturating_sub(1))
}

fn chord_segment_end(step: usize, chord_count: usize) -> usize {
    let current = chord_index(step, chord_count);
    (step + 1..=GRID_STEPS)
        .find(|next| *next == GRID_STEPS || chord_index(*next, chord_count) != current)
        .unwrap_or(GRID_STEPS)
}

fn append_span(mml: &mut String, value: &str, mut steps: usize) {
    const DURATIONS: &[(usize, &str)] = &[
        (16, "1"),
        (15, "2..."),
        (14, "2.."),
        (12, "2."),
        (8, "2"),
        (7, "4.."),
        (6, "4."),
        (4, "4"),
        (3, "8."),
        (2, "8"),
        (1, "16"),
    ];
    while steps > 0 {
        let (duration_steps, suffix) = DURATIONS
            .iter()
            .copied()
            .find(|(duration_steps, _)| *duration_steps <= steps)
            .expect("one sixteenth is always available");
        mml.push_str(value);
        mml.push_str(suffix);
        steps -= duration_steps;
    }
}

fn note_name(note: u8) -> String {
    const PITCHES: [&str; 12] = [
        "c", "c+", "d", "d+", "e", "f", "f+", "g", "g+", "a", "a+", "b",
    ];
    format!("o{}{}", note / 12, PITCHES[usize::from(note % 12)])
}

/// コード進行を 1 小節ぶんずつ（= 1 コードずつ）に切り分ける。
///
/// # なぜ必要か
///
/// grid は **1 セル = 1 小節**で、chord 行もその 1 行なので、chord 行の 1 セルには
/// 原則 1 コードしか入らない。`I-IV-V-I` をまるごと 1 セルへ書くと、
/// [`chord2mml_input`] が縦棒で囲むぶん 1 小節の中で 4 等分され、**時間軸が 1/4 に
/// 圧縮される**（`c4eg f4a<c g4b<d c4eg`）。カタログの進行を chord 行へ配るときは、
/// 必ずここで切ってから 1 小節に 1 つずつ書く。
///
/// # ハイフンで割ってはいけない
///
/// ハイフンは区切りとして働かない位置にも現れる（`cmrt_chord` の
/// `a_hyphen_that_is_part_of_a_chord_name_is_not_a_separator` 参照）。区切りの判定は
/// chord2mml のパーサだけが正しく行えるので、`parse` の結果から拾う。
///
/// # 綴りは chord2mml の正規形になる
///
/// 拾うのはパーサが正規化したあとの綴りなので、`vi` は `VIm` として返る
/// （実測: `close | vi |` と `close | VIm |` はどちらも `v11/*|*/'a1<ce'/*|*/`）。
/// 鳴る音は同じで、正規形は再入力しても同じに読めるので、そのまま chord 行へ書く。
///
/// 解釈できない入力・コードが 1 つも無い入力では空 `Vec` を返す。
pub(crate) fn split_progression_into_measures(degrees: &str) -> Vec<String> {
    let degrees = degrees.trim();
    if degrees.is_empty() {
        return Vec::new();
    }
    let Ok(parsed) = chord2mml_core::parse(degrees) else {
        return Vec::new();
    };
    parsed
        .items()
        .iter()
        .filter_map(|item| match item {
            chord2mml_core::ParsedItem::Chord { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
