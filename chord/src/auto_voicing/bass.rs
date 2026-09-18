//! Key と structural root に基づく Bass の octave lane 選択。

const OCTAVE: i16 = 12;
const WINDOW_SPAN: i16 = 16;
const MAX_WINDOW_START: i16 = 127 - WINDOW_SPAN;
const TONIC_C_ANCHOR: i16 = 48;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    jump_excess: u64,
    octave_moves: u64,
    total_movement: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScoredPath {
    score: Score,
    notes: Vec<u8>,
}

/// Structural root ごとの Bass note を選ぶ。
///
/// `structural_roots` は source chord の最低音、`lowest_chord_notes` は独立に選択済みの
/// chord layer の最低音。tonic は canonical register に固定し、non-tonic は canonical
/// Bass とその隣接 octave のみを候補にする。
pub(super) fn select_path(
    key_pitch_class: u8,
    structural_roots: &[u8],
    lowest_chord_notes: &[u8],
    seed: Option<u8>,
) -> Option<Vec<u8>> {
    if structural_roots.is_empty() {
        return Some(Vec::new());
    }
    if structural_roots.len() != lowest_chord_notes.len() {
        return None;
    }

    let key_pitch_class = key_pitch_class % 12;
    let tonic_anchor = TONIC_C_ANCHOR + i16::from(key_pitch_class);
    let first_window_start = (tonic_anchor - WINDOW_SPAN).max(0);
    let last_window_start = tonic_anchor.min(MAX_WINDOW_START);

    let mut best = None;
    for window_start in first_window_start..=last_window_start {
        let window_end = window_start + WINDOW_SPAN;
        let candidate_sets = structural_roots
            .iter()
            .zip(lowest_chord_notes)
            .map(|(&root, &lowest_chord)| {
                candidates(
                    key_pitch_class,
                    tonic_anchor,
                    root,
                    lowest_chord,
                    window_start,
                    window_end,
                )
            })
            .collect::<Vec<_>>();
        if candidate_sets.iter().any(Vec::is_empty) {
            continue;
        }

        let selected = choose_best(&candidate_sets, seed);
        if best
            .as_ref()
            .is_none_or(|current| path_is_better(&selected, current))
        {
            best = Some(selected);
        }
    }
    best.map(|path| path.notes)
}

/// tonic-containing window 内にある canonical / ±12 candidates を作る。
///
/// chord layer の最低音との unison は許す（`docs/adr/0021`）。`<` にすると canonical が
/// 消えた 1 chord のために jump_excess が前後の chord まで octave down させる。
fn candidates(
    key_pitch_class: u8,
    tonic_anchor: i16,
    structural_root: u8,
    lowest_chord_note: u8,
    window_start: i16,
    window_end: i16,
) -> Vec<(u8, bool)> {
    let canonical = i16::from(structural_root) - OCTAVE;
    let is_tonic = structural_root % 12 == key_pitch_class;
    let candidate_notes = if is_tonic {
        vec![(tonic_anchor, false)]
    } else {
        [-1, 0, 1]
            .into_iter()
            .map(|octave| (canonical + octave * OCTAVE, octave != 0))
            .collect()
    };

    candidate_notes
        .into_iter()
        .filter_map(|(note, moved)| {
            if !(window_start..=window_end).contains(&note) {
                return None;
            }
            let note = u8::try_from(note).ok().filter(|note| *note <= 127)?;
            (note <= lowest_chord_note).then_some((note, moved))
        })
        .collect()
}

/// 1 window の候補を DP で比較する。Score の field 順が承認済み優先順位で、
/// notes の辞書順を lower-note-first の deterministic tie-break にする。
fn choose_best(candidate_sets: &[Vec<(u8, bool)>], seed: Option<u8>) -> ScoredPath {
    let mut previous = candidate_sets[0]
        .iter()
        .map(|&(note, moved)| ScoredPath {
            score: Score {
                jump_excess: seed.map_or(0, |previous| jump_excess(previous, note)),
                octave_moves: u64::from(moved),
                total_movement: seed.map_or(0, |previous| u64::from(previous.abs_diff(note))),
            },
            notes: vec![note],
        })
        .collect::<Vec<_>>();

    for candidates in candidate_sets.iter().skip(1) {
        let mut next = Vec::with_capacity(candidates.len());
        for &(note, moved) in candidates {
            let selected = previous
                .iter()
                .map(|path| extend_path(path, note, moved))
                .min_by(compare_paths)
                .expect("every candidate step has a previous path");
            next.push(selected);
        }
        previous = next;
    }

    previous
        .into_iter()
        .min_by(compare_paths)
        .expect("candidate sets are non-empty")
}

fn extend_path(path: &ScoredPath, note: u8, moved: bool) -> ScoredPath {
    let previous = *path.notes.last().expect("path has at least one note");
    let mut notes = path.notes.clone();
    notes.push(note);
    ScoredPath {
        score: Score {
            jump_excess: path.score.jump_excess + jump_excess(previous, note),
            octave_moves: path.score.octave_moves + u64::from(moved),
            total_movement: path.score.total_movement + u64::from(previous.abs_diff(note)),
        },
        notes,
    }
}

fn jump_excess(previous: u8, next: u8) -> u64 {
    let excess = u64::from(previous.abs_diff(next).saturating_sub(7));
    excess * excess
}

fn compare_paths(left: &ScoredPath, right: &ScoredPath) -> std::cmp::Ordering {
    left.score
        .cmp(&right.score)
        .then_with(|| left.notes.cmp(&right.notes))
}

fn path_is_better(candidate: &ScoredPath, current: &ScoredPath) -> bool {
    compare_paths(candidate, current).is_lt()
}

#[cfg(test)]
mod tests;
