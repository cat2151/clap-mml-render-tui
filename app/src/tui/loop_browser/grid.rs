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

    pub(super) fn playback_grid(&self) -> LoopPlaybackGrid {
        self.track_grid
            .iter()
            .map(|track| {
                track
                    .iter()
                    .map(|cell| {
                        cell.as_ref().map(|clip| {
                            let analysis = self.analysis_for_wav(&clip.wav);
                            LoopPlaybackClip {
                                path: clip.wav.path(),
                                span_measures: clip.span_measures,
                                bpm: analysis.map_or(120.0, |analysis| analysis.bpm),
                                category: self
                                    .metadata
                                    .category_for_wav(&clip.wav)
                                    .map(str::to_string),
                                meter_numerator: analysis
                                    .map_or(4, |analysis| analysis.meter_numerator),
                                meter_denominator: analysis
                                    .map_or(4, |analysis| analysis.meter_denominator),
                            }
                        })
                    })
                    .collect()
            })
            .collect()
    }
}
