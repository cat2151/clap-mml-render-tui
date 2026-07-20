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
        let measure_count = self.effective_measure_count();
        self.track_grid
            .iter()
            .map(|track| {
                (0..measure_count)
                    .map(|measure| {
                        track
                            .get(measure)
                            .and_then(Option::as_ref)
                            .or_else(|| previous_measure_repeat_clip(track, measure, measure_count))
                            .map(|clip| {
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

    pub(in crate::tui) fn previous_measure_repeat_clip(
        &self,
        track: usize,
        measure: usize,
    ) -> Option<&LoopTrackClip> {
        let cells = self.track_grid.get(track)?;
        previous_measure_repeat_clip(cells, measure, self.effective_measure_count())
    }

    pub(in crate::tui) fn displayed_measure_count(&self) -> usize {
        self.effective_measure_count()
            .max(self.measure_cursor.saturating_add(1))
            .min(self.track_grid.first().map_or(1, Vec::len))
    }

    pub(in crate::tui) fn clip_exceeds_time_ratio_limits(
        &self,
        clip: &LoopTrackClip,
        target_bpm: f64,
    ) -> bool {
        let source_bpm = self
            .analysis_for_wav(&clip.wav)
            .map_or(crate::loop_time_stretch::TARGET_BPM, |analysis| {
                analysis.bpm
            });
        crate::loop_time_stretch::exceeds_time_ratio_limits(source_bpm, target_bpm)
    }

    pub(in crate::tui) fn target_bpm(&self) -> crate::loop_time_stretch::TargetBpm {
        crate::loop_time_stretch::select_target_bpm(
            self.track_grid
                .iter()
                .flatten()
                .filter_map(Option::as_ref)
                .map(|clip| {
                    self.analysis_for_wav(&clip.wav)
                        .map_or(crate::loop_time_stretch::TARGET_BPM, |analysis| {
                            analysis.bpm
                        })
                }),
        )
    }

    fn effective_measure_count(&self) -> usize {
        self.track_grid
            .iter()
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
}

fn previous_measure_repeat_clip(
    track: &[Option<LoopTrackClip>],
    measure: usize,
    measure_count: usize,
) -> Option<&LoopTrackClip> {
    if measure >= measure_count || track.get(measure).is_some_and(Option::is_some) {
        return None;
    }
    let (start, clip) = track
        .iter()
        .take(measure)
        .enumerate()
        .rev()
        .find_map(|(start, clip)| clip.as_ref().map(|clip| (start, clip)))?;
    (clip.span_measures == 1 && start.saturating_add(clip.span_measures) <= measure).then_some(clip)
}
