use super::{LoopTrackClip, LoopTrackGrid, LoopWavId};

pub fn reflow_with_spans(
    grid: &LoopTrackGrid,
    mut span_for: impl FnMut(&LoopWavId) -> Option<usize>,
) -> (LoopTrackGrid, bool) {
    let original_width = grid.first().map_or(1, Vec::len).max(1);
    let mut tracks = Vec::with_capacity(grid.len().max(1));
    let mut width = original_width;
    let mut changed = false;
    for track in grid {
        let mut updated = vec![None; original_width];
        let mut occupied_until = 0;
        for (old_measure, clip) in track.iter().enumerate().filter_map(|(measure, clip)| {
            clip.as_ref()
                .filter(|clip| !clip.is_previous())
                .map(|clip| (measure, clip))
        }) {
            let span = span_for(&clip.wav).unwrap_or(clip.span_measures).max(1);
            let measure = old_measure.max(occupied_until);
            let end = measure.saturating_add(span);
            if end > updated.len() {
                updated.resize(end, None);
            }
            updated[measure] = Some(LoopTrackClip::explicit(clip.wav.clone(), span));
            occupied_until = end;
            width = width.max(end);
            changed |= measure != old_measure || span != clip.span_measures;
        }
        tracks.push(updated);
    }
    if tracks.is_empty() {
        tracks.push(vec![None; width]);
        changed = true;
    }
    for track in &mut tracks {
        track.resize(width, None);
    }
    changed |= grid.iter().any(|track| track.len() != width);
    (tracks, changed)
}

pub fn normalize_previous_markers(grid: &LoopTrackGrid) -> (LoopTrackGrid, bool) {
    let mut normalized = without_previous_markers(grid);
    let axis = longest_explicit_end(&normalized);
    let width = normalized
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(axis)
        .max(axis);
    for track in &mut normalized {
        track.resize(width, None);
        fill_track(track, axis);
    }
    let changed = normalized != *grid;
    (normalized, changed)
}

fn without_previous_markers(grid: &LoopTrackGrid) -> LoopTrackGrid {
    let mut explicit = grid.clone();
    for track in &mut explicit {
        for cell in track {
            if cell.as_ref().is_some_and(LoopTrackClip::is_previous) {
                *cell = None;
            }
        }
    }
    explicit
}

fn longest_explicit_end(grid: &LoopTrackGrid) -> usize {
    grid.iter()
        .flat_map(|track| {
            track.iter().enumerate().filter_map(|(measure, clip)| {
                clip.as_ref()
                    .map(|clip| measure.saturating_add(clip.span_measures))
            })
        })
        .max()
        .unwrap_or(1)
        .max(1)
}

fn fill_track(track: &mut [Option<LoopTrackClip>], axis: usize) {
    let anchors = track
        .iter()
        .take(axis)
        .enumerate()
        .filter_map(|(measure, clip)| clip.as_ref().map(|clip| (measure, clip.clone())))
        .collect::<Vec<_>>();
    if anchors.is_empty() {
        return;
    }
    for (index, (source_measure, source)) in anchors.iter().enumerate() {
        let start = source_measure.saturating_add(source.span_measures);
        let end = anchors.get(index + 1).map_or(axis, |(measure, _)| *measure);
        fill_segment(track, start, end, *source_measure, source);
    }
    let (first_measure, _) = &anchors[0];
    if *first_measure > 0 {
        let (source_measure, source) = anchors.last().expect("anchors is not empty");
        fill_segment(track, 0, *first_measure, *source_measure, source);
    }
}

fn fill_segment(
    track: &mut [Option<LoopTrackClip>],
    mut measure: usize,
    end: usize,
    source_measure: usize,
    source: &LoopTrackClip,
) {
    while measure < end {
        let span_measures = source.span_measures.min(end - measure).max(1);
        track[measure] = Some(LoopTrackClip {
            wav: source.wav.clone(),
            span_measures,
            previous_source_measure: Some(source_measure),
        });
        measure += span_measures;
    }
}
