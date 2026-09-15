//! コード進行の自動ボイシング。top note の跳躍が最小になる転回とオクターブを選ぶ。
//!
//! 仕様は `cmrt-client-playground` の auto-adjust（`src/auto-adjust/auto-adjust.ts` と
//! `auto-adjust-candidates.ts`）の移植。あちらはコード表記のテキスト（`I^1'` のような
//! 転回・オクターブ指定）を候補にして候補ごとに chord2mml を回すが、ここは
//! [`crate::chord_notes`] が既に note number を返すので **note number 上で直接**
//! 転回とオクターブ移動を作る。結果は等価で、候補数ぶん chord2mml を回さずに済む。
//!
//! bass は playground の `bass is root.`（bass をコード root の1オクターブ下へ置き、
//! 別トラックへ分ける）に相当するものを自前で作る。コード進行カタログの degree 表記は
//! すべて root position（分数コードが無い）なので、chord2mml 出力の最低音を root と
//! みなしてよい。カタログに分数コードが入ったらこの前提は崩れる。

mod bass;
mod performance;
mod upper;

pub use performance::{
    bass_timed_progression, revoice_timed_progression,
    timed_auto_voiced_bass_chord_progression_performance,
    timed_auto_voiced_chord_progression_performance,
};

/// コード1つぶんの、bass と和音を分けて持つ voicing。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChordVoicing {
    /// bass パートが鳴らす1音。和音側には含めない。
    pub bass: Option<u8>,
    /// 和音の構成音。昇順。root を含む（playground の chordMml と同じ）。
    pub notes: Vec<u8>,
}

/// 候補に使う bass のオクターブ移動。
const OCTAVE_OFFSETS: [i16; 3] = [-1, 0, 1];

/// bass の快適音域（playground の相対 -24..0 = MIDI 36..60）。
const BASS_RANGE: (f64, f64) = (36.0, 60.0);

#[derive(Clone, Copy, Debug)]
struct BassCandidate {
    note: u8,
    notation_penalty: f64,
}

/// 進行全体のボイシングを Viterbi DP で選び直す。
///
/// `seed` は「この進行の1コード目が接続する相手」。cycle をまたいで進行を引き直すとき、
/// いま鳴っているコードを渡すと境界の跳躍も最小化される。無ければ `None`。
///
/// 空の和音が混ざっていたら候補を作れないので、進行全体を素通しする。
pub fn auto_voice(chords: &[Vec<u8>], seed: Option<&ChordVoicing>) -> Vec<ChordVoicing> {
    if chords.is_empty() || chords.iter().any(Vec::is_empty) {
        return chords.iter().map(|notes| passthrough(notes)).collect();
    }
    let Some(upper_path) = upper::select_path(chords, seed.map(|voicing| voicing.notes.as_slice()))
    else {
        // MIDI 範囲外へ振り切れて候補が作れない和音。voicing をあきらめて素通しする。
        return chords.iter().map(|notes| passthrough(notes)).collect();
    };
    let bass_candidate_sets = chords
        .iter()
        .zip(&upper_path)
        .map(|(notes, voiced)| build_bass_candidates(notes, voiced))
        .collect::<Vec<_>>();
    if bass_candidate_sets.iter().any(Vec::is_empty) {
        // MIDI 範囲外へ振り切れて候補が作れない和音。voicing をあきらめて素通しする。
        return chords.iter().map(|notes| passthrough(notes)).collect();
    }
    let bass_path = choose_best_bass(&bass_candidate_sets, seed.and_then(|voicing| voicing.bass));
    upper_path
        .into_iter()
        .zip(bass_path)
        .map(|(notes, bass)| ChordVoicing {
            bass: Some(bass.note),
            notes,
        })
        .collect()
}

/// Key を使って進行全体の Bass octave lane を固定した auto voicing を返す。
///
/// Chord layer は [`auto_voice`] と同じ独立した候補選択を使う。Bass layer は
/// `key_pitch_class` の tonic anchor を含む一続きの文脈として選択する。
/// Key を構造化 parse 済みの呼び出し側はこの入口を使い、Key を持たない既存の
/// 呼び出し側は互換 API の [`auto_voice`] を引き続き使用できる。
pub fn auto_voice_with_key(
    chords: &[Vec<u8>],
    key_pitch_class: u8,
    seed: Option<&ChordVoicing>,
) -> Vec<ChordVoicing> {
    if chords.is_empty() || chords.iter().any(Vec::is_empty) {
        return chords.iter().map(|notes| passthrough(notes)).collect();
    }
    let Some(upper_path) = upper::select_path(chords, seed.map(|voicing| voicing.notes.as_slice()))
    else {
        return chords.iter().map(|notes| passthrough(notes)).collect();
    };
    let structural_roots = chords
        .iter()
        .map(|notes| *notes.iter().min().expect("source chord is not empty"))
        .collect::<Vec<_>>();
    let lowest_chord_notes = upper_path
        .iter()
        .map(|notes| *notes.iter().min().expect("voiced chord is not empty"))
        .collect::<Vec<_>>();
    let Some(bass_path) = bass::select_path(
        key_pitch_class,
        &structural_roots,
        &lowest_chord_notes,
        seed.and_then(|voicing| voicing.bass),
    ) else {
        return chords.iter().map(|notes| passthrough(notes)).collect();
    };
    upper_path
        .into_iter()
        .zip(bass_path)
        .map(|(notes, bass)| ChordVoicing {
            bass: Some(bass),
            notes,
        })
        .collect()
}

/// 進行の最大跳躍。ログとテストで「効いているか」を見るために使う。
pub fn max_jumps(voicings: &[ChordVoicing]) -> (u8, u8) {
    let mut max_top = 0;
    let mut max_bass = 0;
    for pair in voicings.windows(2) {
        if let (Some(previous), Some(current)) = (top_of(&pair[0]), top_of(&pair[1])) {
            max_top = max_top.max(previous.abs_diff(current));
        }
        if let (Some(previous), Some(current)) = (pair[0].bass, pair[1].bass) {
            max_bass = max_bass.max(previous.abs_diff(current));
        }
    }
    (max_top, max_bass)
}

fn top_of(voicing: &ChordVoicing) -> Option<u8> {
    voicing.notes.iter().copied().max()
}

fn passthrough(notes: &[u8]) -> ChordVoicing {
    ChordVoicing {
        bass: None,
        notes: notes.to_vec(),
    }
}

/// 構造上の root（最低音）の1オクターブ下を基準に Bass 候補を作る。
fn build_bass_candidates(notes: &[u8], voiced: &[u8]) -> Vec<BassCandidate> {
    let mut sorted = notes.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let base_bass = i16::from(sorted[0]) - 12;

    let mut candidates = Vec::new();
    let lowest_upper = *voiced.iter().min().expect("voiced chord is not empty");
    for bass_octave in OCTAVE_OFFSETS {
        let Ok(note) = u8::try_from(base_bass + 12 * bass_octave) else {
            continue;
        };
        if note <= 127 && note < lowest_upper {
            candidates.push(BassCandidate {
                note,
                notation_penalty: f64::from(bass_octave.abs()),
            });
        }
    }
    candidates
}

fn range_penalty(value: f64, range: (f64, f64), weight: f64) -> f64 {
    if value < range.0 {
        return (range.0 - value) * weight;
    }
    if value > range.1 {
        return (value - range.1) * weight;
    }
    0.0
}

fn bass_base_score(candidate: &BassCandidate) -> f64 {
    candidate.notation_penalty + range_penalty(f64::from(candidate.note), BASS_RANGE, 4.0)
}

/// 跳躍 penalty。`free` 半音までは線形、そこを超えたぶんは二乗で効かせる。
fn jump_penalty(left: f64, right: f64, free: f64, weight: f64, extra_weight: f64) -> f64 {
    let jump = (left - right).abs();
    let extra = (jump - free).max(0.0);
    jump * weight + extra * extra * extra_weight
}

fn bass_transition_score(previous: u8, next: u8) -> f64 {
    jump_penalty(f64::from(previous), f64::from(next), 5.0, 4.0, 1.5)
}

/// 上声とは独立に、従来の Bass score で候補パスを選ぶ。
fn choose_best_bass(candidate_sets: &[Vec<BassCandidate>], seed: Option<u8>) -> Vec<BassCandidate> {
    let mut costs: Vec<Vec<f64>> = Vec::with_capacity(candidate_sets.len());
    let mut previous_indexes: Vec<Vec<usize>> = Vec::with_capacity(candidate_sets.len());

    costs.push(
        candidate_sets[0]
            .iter()
            .map(|candidate| {
                bass_base_score(candidate)
                    + seed.map_or(0.0, |seed| bass_transition_score(seed, candidate.note))
            })
            .collect(),
    );
    previous_indexes.push(vec![0; candidate_sets[0].len()]);

    for (index, candidates) in candidate_sets.iter().enumerate().skip(1) {
        let mut step_costs = Vec::with_capacity(candidates.len());
        let mut step_previous = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let mut best_cost = f64::INFINITY;
            let mut best_previous = 0;
            for (previous_index, previous) in candidate_sets[index - 1].iter().enumerate() {
                let cost = costs[index - 1][previous_index]
                    + bass_transition_score(previous.note, candidate.note)
                    + bass_base_score(candidate);
                if cost < best_cost {
                    best_cost = cost;
                    best_previous = previous_index;
                }
            }
            step_costs.push(best_cost);
            step_previous.push(best_previous);
        }
        costs.push(step_costs);
        previous_indexes.push(step_previous);
    }

    let mut selected_index = best_index(costs.last().expect("costs has one row per chord"));
    let mut selected = Vec::with_capacity(candidate_sets.len());
    for index in (0..candidate_sets.len()).rev() {
        selected.push(candidate_sets[index][selected_index]);
        selected_index = previous_indexes[index][selected_index];
    }
    selected.reverse();
    selected
}

fn best_index(costs: &[f64]) -> usize {
    costs
        .iter()
        .enumerate()
        .fold((0, f64::INFINITY), |(best, best_cost), (index, cost)| {
            if *cost < best_cost {
                (index, *cost)
            } else {
                (best, best_cost)
            }
        })
        .0
}

#[cfg(test)]
mod tests;
