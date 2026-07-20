use super::*;

impl LoopBrowser {
    pub(super) fn analysis_for_wav(&self, wav: &LoopWavId) -> Option<LoopWavAnalysis> {
        self.wav_analyses
            .iter()
            .find(|(candidate, _)| candidate.matches(wav))
            .map(|(_, analysis)| *analysis)
    }

    pub(in crate::tui) fn clip_at(
        &self,
        track: usize,
        measure: usize,
    ) -> Option<(usize, &LoopTrackClip)> {
        let cells = self.track_grid.get(track)?;
        cells
            .iter()
            .take(measure.saturating_add(1))
            .enumerate()
            .rev()
            .find_map(|(start, clip)| {
                clip.as_ref()
                    .filter(|clip| start.saturating_add(clip.span_measures) > measure)
                    .map(|clip| (start, clip))
            })
    }
}
