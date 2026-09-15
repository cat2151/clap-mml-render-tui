use std::collections::HashSet;

/// 候補に使う転回数の上限。構成音数-1 で頭打ちにする。
const MAX_INVERSION: usize = 3;
/// 候補に使う和音のオクターブ移動。
const OCTAVE_OFFSETS: [i16; 3] = [-1, 0, 1];

/// 隣り合う半音（ぶつかり）1つあたりの penalty。他の項より桁違いに重い。
const SEMITONE_INTERVAL_PENALTY: f64 = 24.0;
/// top note の快適音域。playground（60..84）より広めに取る。
const TOP_RANGE: (f64, f64) = (55.0, 88.0);

#[derive(Clone, Copy, Debug)]
struct Metrics {
    top: f64,
    center: f64,
    semitone_intervals: usize,
}

#[derive(Clone, Debug)]
struct Candidate {
    notes: Vec<u8>,
    metrics: Metrics,
    notation_penalty: f64,
}

/// Bass の候補や score を参照せず、和音の転回とオクターブだけを進行全体で選ぶ。
pub(super) fn select_path(chords: &[Vec<u8>], seed: Option<&[u8]>) -> Option<Vec<Vec<u8>>> {
    let candidate_sets = chords
        .iter()
        .map(|notes| build_candidates(notes))
        .collect::<Vec<_>>();
    if candidate_sets.iter().any(Vec::is_empty) {
        return None;
    }
    Some(
        choose_best(&candidate_sets, seed.and_then(metrics))
            .into_iter()
            .map(|candidate| candidate.notes)
            .collect(),
    )
}

fn metrics(notes: &[u8]) -> Option<Metrics> {
    if notes.is_empty() {
        return None;
    }
    let sum = notes.iter().map(|note| f64::from(*note)).sum::<f64>();
    Some(Metrics {
        top: f64::from(*notes.iter().max().expect("notes is not empty")),
        center: sum / notes.len() as f64,
        semitone_intervals: count_semitone_intervals(notes),
    })
}

/// 昇順に並んだ構成音の、隣接差がちょうど半音1つのペアの数。
pub(super) fn count_semitone_intervals(notes: &[u8]) -> usize {
    notes
        .windows(2)
        .filter(|pair| pair[1].saturating_sub(pair[0]) == 1)
        .count()
}

/// 1コードぶんの候補を、転回 × 和音オクターブで作る。
fn build_candidates(notes: &[u8]) -> Vec<Candidate> {
    let mut sorted = notes.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for inversion in 0..=MAX_INVERSION.min(sorted.len() - 1) {
        let inverted = invert(&sorted, inversion);
        for chord_octave in OCTAVE_OFFSETS {
            let Some(voiced) = transpose(&inverted, chord_octave) else {
                continue;
            };
            if !seen.insert(voiced.clone()) {
                continue;
            }
            let Some(metrics) = metrics(&voiced) else {
                continue;
            };
            candidates.push(Candidate {
                notes: voiced,
                metrics,
                notation_penalty: notation_penalty(inversion, chord_octave),
            });
        }
    }
    candidates
}

/// 下から `inversion` 個の音を1オクターブ上へ上げ、昇順へ並べ直す。
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

fn notation_penalty(inversion: usize, chord_octave: i16) -> f64 {
    f64::from(chord_octave.abs()) + inversion as f64 * 0.4
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
        + range_penalty(candidate.metrics.top, TOP_RANGE, 3.0)
}

/// 跳躍 penalty。`free` 半音までは線形、そこを超えたぶんは二乗で効かせる。
fn jump_penalty(left: f64, right: f64, free: f64, weight: f64, extra_weight: f64) -> f64 {
    let jump = (left - right).abs();
    let extra = (jump - free).max(0.0);
    jump * weight + extra * extra * extra_weight
}

fn transition_score(previous: &Metrics, next: &Metrics) -> f64 {
    jump_penalty(previous.top, next.top, 4.0, 3.0, 1.0)
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
