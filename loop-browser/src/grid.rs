use super::*;

impl LoopPlaybackClip {
    pub fn is_one_shot(&self) -> bool {
        self.kind == cmrt_loop_domain::loop_wav_analysis::LoopWavKind::OneShot
    }

    pub fn source_bpm(&self) -> Option<f64> {
        if self.is_one_shot() {
            None
        } else {
            self.bpm
        }
    }
}

impl LoopBrowser {
    pub fn analysis_for_wav(&self, wav: &LoopWavId) -> Option<LoopWavAnalysis> {
        self.wav_analysis_indices
            .get(&wav.lookup_key())
            .and_then(|index| self.wav_analyses.get(*index))
            .map(|(_, analysis)| *analysis)
    }

    pub fn waveform_for_wav(
        &self,
        wav: &LoopWavId,
    ) -> Option<&cmrt_loop_domain::loop_waveform::LoopWaveform> {
        self.wav_analysis_indices
            .get(&wav.lookup_key())
            .and_then(|index| self.wav_waveforms.get(*index))
    }

    pub fn waveform_display_scale(&self) -> cmrt_loop_domain::loop_waveform::WaveformDisplayScale {
        self.waveform_display_scale
    }

    #[cfg(test)]
    pub fn rebuild_wav_analysis_indices(&mut self) {
        self.wav_analysis_indices.clear();
        for (index, (wav, _)) in self.wav_analyses.iter().enumerate() {
            self.wav_analysis_indices
                .entry(wav.lookup_key())
                .or_insert(index);
        }
    }

    pub fn clip_at(&self, track: usize, measure: usize) -> Option<(usize, &LoopTrackClip)> {
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

    pub fn playback_grid(&self) -> LoopPlaybackGrid {
        let measure_count = self.effective_measure_count();
        self.track_grid
            .iter()
            .map(|track| {
                (0..measure_count)
                    .map(|measure| {
                        track
                            .get(measure)
                            .and_then(Option::as_ref)
                            .and_then(|clip| {
                                let analysis = self.analysis_for_wav(&clip.wav);
                                if clip.is_previous() && analysis.is_some_and(|analysis| {
                                    analysis.kind
                                        == cmrt_loop_domain::loop_wav_analysis::LoopWavKind::OneShot
                                }) {
                                    return None;
                                }
                                let tempo = analysis.and_then(|analysis| analysis.tempo);
                                Some(LoopPlaybackClip {
                                    path: clip.wav.path(),
                                    span_measures: clip.span_measures,
                                    kind: analysis.map_or(
                                        cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
                                        |analysis| analysis.kind,
                                    ),
                                    bpm: analysis
                                        .map_or(Some(120.0), |_| tempo.map(|tempo| tempo.bpm)),
                                    category: self.category_for_wav(&clip.wav).map(str::to_string),
                                    meter_numerator: tempo.map_or(4, |tempo| tempo.meter_numerator),
                                    meter_denominator: tempo
                                        .map_or(4, |tempo| tempo.meter_denominator),
                                })
                            })
                    })
                    .collect()
            })
            .collect()
    }

    pub fn displayed_clip_at(&self, track: usize, measure: usize) -> Option<&LoopTrackClip> {
        self.clip_at(track, measure).map(|(_, clip)| clip)
    }

    pub fn sync_tree_to_current_cell(&mut self) {
        let wav = self
            .displayed_clip_at(self.track_cursor, self.measure_cursor)
            .map(|clip| clip.wav.clone());
        if let Some(wav) = wav {
            self.reveal_wav(&wav);
        }
    }

    pub fn playback_clip(&self, clip: &LoopTrackClip) -> LoopPlaybackClip {
        let key = clip.wav.lookup_key();
        let analysis = self
            .wav_analysis_indices
            .get(&key)
            .and_then(|index| self.wav_analyses.get(*index))
            .map(|(_, analysis)| *analysis);
        let tempo = analysis.and_then(|analysis| analysis.tempo);
        LoopPlaybackClip {
            path: clip.wav.path(),
            span_measures: clip.span_measures,
            kind: analysis.map_or(
                cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
                |value| value.kind,
            ),
            bpm: analysis.map_or(
                Some(cmrt_loop_browser_domain::time_stretch::TARGET_BPM),
                |_| tempo.map(|value| value.bpm),
            ),
            category: self.wav_categories.get(&key).cloned(),
            meter_numerator: tempo.map_or(4, |value| value.meter_numerator),
            meter_denominator: tempo.map_or(4, |value| value.meter_denominator),
        }
    }

    pub fn displayed_measure_count(&self) -> usize {
        self.effective_measure_count()
            .max(self.measure_cursor.saturating_add(1))
            .min(self.track_grid.first().map_or(1, Vec::len))
    }

    #[cfg(test)]
    pub fn clip_exceeds_time_ratio_limits(&self, clip: &LoopTrackClip, target_bpm: f64) -> bool {
        let source_bpm = self
            .analysis_for_wav(&clip.wav)
            .and_then(|analysis| analysis.tempo)
            .map(|tempo| tempo.bpm);
        source_bpm.is_some_and(|source_bpm| {
            cmrt_loop_browser_domain::time_stretch::exceeds_time_ratio_limits(
                source_bpm, target_bpm,
            )
        })
    }

    pub fn target_bpm(&self) -> cmrt_loop_browser_domain::time_stretch::TargetBpm {
        cmrt_loop_browser_domain::time_stretch::select_target_bpm(
            self.track_grid
                .iter()
                .flatten()
                .filter_map(Option::as_ref)
                .filter_map(|clip| {
                    self.analysis_for_wav(&clip.wav)
                        .and_then(|analysis| analysis.tempo)
                        .map(|tempo| tempo.bpm)
                }),
        )
    }

    pub fn beats_per_measure(&self) -> usize {
        let measure_count = self.effective_measure_count();
        for measure in 0..measure_count {
            for track in &self.track_grid {
                if let Some(clip) = track.get(measure).and_then(Option::as_ref) {
                    if let Some(tempo) = self
                        .analysis_for_wav(&clip.wav)
                        .and_then(|analysis| analysis.tempo)
                    {
                        return usize::from(tempo.meter_numerator).max(1);
                    }
                }
            }
        }
        4
    }

    fn effective_measure_count(&self) -> usize {
        self.track_grid
            .iter()
            .flat_map(|track| {
                track.iter().enumerate().filter_map(|(measure, clip)| {
                    clip.as_ref()
                        .filter(|clip| !clip.is_previous())
                        .map(|clip| measure.saturating_add(clip.span_measures))
                })
            })
            .max()
            .unwrap_or(1)
            .max(1)
    }
}
