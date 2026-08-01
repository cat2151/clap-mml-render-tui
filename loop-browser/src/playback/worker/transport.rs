use super::LoopPlaybackGrid;

#[derive(Default)]
pub struct TransportState {
    paused: bool,
    current_measure: Option<usize>,
    restart_measure: Option<usize>,
    /// 現在のグリッドを何周し終えたか。先読みしたグリッドへ差し替えるタイミングの判定に使う。
    cycle: u64,
}

impl TransportState {
    pub fn pause(&mut self) {
        self.paused = true;
        self.current_measure = None;
        self.restart_measure = None;
        self.cycle = 0;
    }

    pub fn resume_at(&mut self, measure: usize) {
        self.paused = false;
        self.current_measure = None;
        self.restart_measure = Some(measure);
        self.cycle = 0;
    }

    pub fn restart_at(&mut self, measure: usize) {
        self.current_measure = None;
        self.restart_measure = Some(measure);
        self.cycle = 0;
    }

    pub fn started(&mut self, measure: usize) {
        self.current_measure = Some(measure);
    }

    pub fn clear_current(&mut self) {
        self.current_measure = None;
    }

    pub fn cycle(&self) -> u64 {
        self.cycle
    }

    /// グリッドを差し替えた直後に呼び、周回数を新しいグリッドの 0 周目から数え直す。
    pub fn reset_cycle(&mut self) {
        self.cycle = 0;
    }

    /// 次に鳴らす measure が先頭へ折り返すか（＝いま周の境目か）。
    /// 差し替えは音の途切れを避けるためこの境目でだけ行う。
    pub fn wraps_next(&self, grid: &LoopPlaybackGrid) -> bool {
        if self.paused || self.restart_measure.is_some() {
            return false;
        }
        let Some(current) = self.current_measure else {
            return false;
        };
        next_measure(grid, Some(current)).is_some_and(|measure| measure <= current)
    }

    pub fn next_measure_to_start(&mut self, grid: &LoopPlaybackGrid) -> Option<usize> {
        if self.paused {
            return None;
        }
        if let Some(measure) = self
            .restart_measure
            .take()
            .and_then(|measure| measure_at_or_after(grid, measure))
        {
            return Some(measure);
        }
        let next = next_measure(grid, self.current_measure);
        if let (Some(current), Some(next)) = (self.current_measure, next) {
            if next <= current {
                self.cycle = self.cycle.saturating_add(1);
            }
        }
        next
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

#[cfg(test)]
mod tests;
