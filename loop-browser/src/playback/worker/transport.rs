use super::LoopPlaybackGrid;

#[derive(Default)]
pub struct TransportState {
    paused: bool,
    current_measure: Option<usize>,
    restart_measure: Option<usize>,
}

impl TransportState {
    pub fn pause(&mut self) {
        self.paused = true;
        self.current_measure = None;
        self.restart_measure = None;
    }

    pub fn resume_at(&mut self, measure: usize) {
        self.paused = false;
        self.current_measure = None;
        self.restart_measure = Some(measure);
    }

    pub fn restart_at(&mut self, measure: usize) {
        self.current_measure = None;
        self.restart_measure = Some(measure);
    }

    pub fn started(&mut self, measure: usize) {
        self.current_measure = Some(measure);
    }

    pub fn clear_current(&mut self) {
        self.current_measure = None;
    }

    pub fn next_measure_to_start(&mut self, grid: &LoopPlaybackGrid) -> Option<usize> {
        if self.paused {
            return None;
        }
        self.restart_measure
            .take()
            .and_then(|measure| measure_at_or_after(grid, measure))
            .or_else(|| next_measure(grid, self.current_measure))
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

pub fn next_measure(grid: &LoopPlaybackGrid, current: Option<usize>) -> Option<usize> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    if measures == 0 {
        return None;
    }
    let start = current.map_or(0, |measure| (measure + 1) % measures);
    for offset in 0..measures {
        let measure = (start + offset) % measures;
        if measure_is_occupied(grid, measure) {
            return Some(measure);
        }
    }
    None
}

pub fn measure_at_or_after(grid: &LoopPlaybackGrid, start: usize) -> Option<usize> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    if measures == 0 {
        return None;
    }
    let start = start % measures;
    for offset in 0..measures {
        let measure = (start + offset) % measures;
        if measure_is_occupied(grid, measure) {
            return Some(measure);
        }
    }
    None
}

fn measure_is_occupied(grid: &LoopPlaybackGrid, measure: usize) -> bool {
    grid.iter().any(|track| {
        track
            .iter()
            .take(measure.saturating_add(1))
            .enumerate()
            .any(|(start, clip)| {
                clip.as_ref()
                    .is_some_and(|clip| start.saturating_add(clip.span_measures) > measure)
            })
    })
}
