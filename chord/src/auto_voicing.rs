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

use std::collections::HashSet;

/// コード1つぶんの、bass と和音を分けて持つ voicing。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChordVoicing {
    /// bass パートが鳴らす1音。和音側には含めない。
    pub bass: Option<u8>,
    /// 和音の構成音。昇順。root を含む（playground の chordMml と同じ）。
    pub notes: Vec<u8>,
}

/// 候補に使う転回数の上限。構成音数-1 で頭打ちにする。
const MAX_INVERSION: usize = 3;
/// 候補に使うオクターブ移動。和音側と bass 側で独立に選ぶ。
const OCTAVE_OFFSETS: [i16; 3] = [-1, 0, 1];

/// 隣り合う半音（ぶつかり）1つあたりの penalty。他の項より桁違いに重い。
const SEMITONE_INTERVAL_PENALTY: f64 = 24.0;
/// bass の快適音域（playground の相対 -24..0 = MIDI 36..60）。
const BASS_RANGE: (f64, f64) = (36.0, 60.0);
/// top note の快適音域。playground（60..84）より広めに取る。
const TOP_RANGE: (f64, f64) = (55.0, 88.0);

#[derive(Clone, Copy, Debug)]
struct Metrics {
    bass: f64,
    top: f64,
    center: f64,
    semitone_intervals: usize,
}

#[derive(Clone, Debug)]
struct Candidate {
    voicing: ChordVoicing,
    metrics: Metrics,
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
    let candidate_sets = chords
        .iter()
        .map(|notes| build_candidates(notes))
        .collect::<Vec<_>>();
    if candidate_sets.iter().any(Vec::is_empty) {
        // MIDI 範囲外へ振り切れて候補が作れない和音。voicing をあきらめて素通しする。
        return chords.iter().map(|notes| passthrough(notes)).collect();
    }
    choose_best(&candidate_sets, seed.and_then(metrics_of))
        .into_iter()
        .map(|candidate| candidate.voicing)
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

fn metrics_of(voicing: &ChordVoicing) -> Option<Metrics> {
    let bass = voicing.bass?;
    metrics(f64::from(bass), &voicing.notes)
}

fn metrics(bass: f64, notes: &[u8]) -> Option<Metrics> {
    if notes.is_empty() {
        return None;
    }
    let sum = notes.iter().map(|note| f64::from(*note)).sum::<f64>();
    Some(Metrics {
        bass,
        top: f64::from(*notes.iter().max().expect("notes is not empty")),
        center: sum / notes.len() as f64,
        semitone_intervals: count_semitone_intervals(notes),
    })
}

/// 昇順に並んだ構成音の、隣接差がちょうど半音1つのペアの数。
fn count_semitone_intervals(notes: &[u8]) -> usize {
    notes
        .windows(2)
        .filter(|pair| pair[1].saturating_sub(pair[0]) == 1)
        .count()
}

/// 1コードぶんの候補を、転回 × 和音オクターブ × bass オクターブで作る。
///
/// 昇順・重複除去した構成音を基準にする。root（最低音）の1オクターブ下が bass の基準値。
fn build_candidates(notes: &[u8]) -> Vec<Candidate> {
    let mut sorted = notes.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let base_bass = i16::from(sorted[0]) - 12;

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for inversion in 0..=MAX_INVERSION.min(sorted.len() - 1) {
        let inverted = invert(&sorted, inversion);
        for chord_octave in OCTAVE_OFFSETS {
            let Some(voiced) = transpose(&inverted, chord_octave) else {
                continue;
            };
            for bass_octave in OCTAVE_OFFSETS {
                let Ok(bass) = u8::try_from(base_bass + 12 * bass_octave) else {
                    continue;
                };
                if bass > 127 || !seen.insert((bass, voiced.clone())) {
                    continue;
                }
                let Some(metrics) = metrics(f64::from(bass), &voiced) else {
                    continue;
                };
                candidates.push(Candidate {
                    voicing: ChordVoicing {
                        bass: Some(bass),
                        notes: voiced.clone(),
                    },
                    metrics,
                    notation_penalty: notation_penalty(inversion, chord_octave, bass_octave),
                });
            }
        }
    }
    candidates
}

/// 下から `inversion` 個の音を1オクターブ上へ上げ、昇順へ並べ直す。
///
/// MIDI 範囲の判定はオクターブ移動まで済ませてからでよいので、ここでは `i16` のまま返す。
fn invert(sorted: &[u8], inversion: usize) -> Vec<i16> {
    let mut inverted = sorted
        .iter()
        .enumerate()
        .map(|(index, note)| i16::from(*note) + if index < inversion { 12 } else { 0 })
        .collect::<Vec<_>>();
    inverted.sort_unstable();
    inverted
}

/// 全構成音を `octave` オクターブ動かす。1音でも MIDI 範囲外なら候補にしない。
fn transpose(notes: &[i16], octave: i16) -> Option<Vec<u8>> {
    notes
        .iter()
        .map(|note| {
            u8::try_from(note + 12 * octave)
                .ok()
                .filter(|note| *note <= 127)
        })
        .collect()
}

/// playground の `notationPenalty` 相当。素直な root position から離れるほど重い。
fn notation_penalty(inversion: usize, chord_octave: i16, bass_octave: i16) -> f64 {
    f64::from(chord_octave.abs())
        + f64::from(bass_octave.abs())
        + inversion as f64 * 0.4
        + if chord_octave == bass_octave {
            0.0
        } else {
            0.15
        }
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

fn base_score(candidate: &Candidate) -> f64 {
    candidate.notation_penalty
        + candidate.metrics.semitone_intervals as f64 * SEMITONE_INTERVAL_PENALTY
        + range_penalty(candidate.metrics.bass, BASS_RANGE, 4.0)
        + range_penalty(candidate.metrics.top, TOP_RANGE, 3.0)
}

/// 跳躍 penalty。`free` 半音までは線形、そこを超えたぶんは二乗で効かせる。
fn jump_penalty(left: f64, right: f64, free: f64, weight: f64, extra_weight: f64) -> f64 {
    let jump = (left - right).abs();
    let extra = (jump - free).max(0.0);
    jump * weight + extra * extra * extra_weight
}

fn transition_score(previous: &Metrics, next: &Metrics) -> f64 {
    jump_penalty(previous.bass, next.bass, 5.0, 4.0, 1.5)
        + jump_penalty(previous.top, next.top, 4.0, 3.0, 1.0)
        + jump_penalty(previous.center, next.center, 4.0, 1.2, 0.3)
}

/// playground の `chooseBestCandidates` と同型の Viterbi DP。
fn choose_best(candidate_sets: &[Vec<Candidate>], seed: Option<Metrics>) -> Vec<Candidate> {
    let mut costs: Vec<Vec<f64>> = Vec::with_capacity(candidate_sets.len());
    let mut previous_indexes: Vec<Vec<usize>> = Vec::with_capacity(candidate_sets.len());

    costs.push(
        candidate_sets[0]
            .iter()
            .map(|candidate| {
                base_score(candidate)
                    + seed.map_or(0.0, |seed| transition_score(&seed, &candidate.metrics))
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
                    + transition_score(&previous.metrics, &candidate.metrics)
                    + base_score(candidate);
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
        selected.push(candidate_sets[index][selected_index].clone());
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
