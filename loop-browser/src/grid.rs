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
        self.playback_grid_of(&self.track_grid)
    }

    /// 任意の track grid（まだ `self.track_grid` へ反映していない先読み用のものを含む）を
    /// 再生スレッドへ渡す形へ変換する。
    pub fn playback_grid_of(&self, track_grid: &LoopTrackGrid) -> LoopPlaybackGrid {
        let measure_count = measure_count_of(track_grid);
        track_grid
            .iter()
            .map(|track| {
                (0..measure_count)
                    .map(|measure| {
                        track.get(measure).and_then(Option::as_ref).map(|clip| {
                            let analysis = self.analysis_for_wav(&clip.wav);
                            let tempo = analysis.and_then(|analysis| analysis.tempo);
                            LoopPlaybackClip {
                                path: clip.wav.path(),
                                span_measures: clip.span_measures,
                                kind: analysis.map_or(
                                    cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
                                    |analysis| analysis.kind,
                                ),
                                bpm: analysis.map_or(Some(120.0), |_| tempo.map(|tempo| tempo.bpm)),
                                category: self.category_for_wav(&clip.wav).map(str::to_string),
                                meter_numerator: tempo.map_or(4, |tempo| tempo.meter_numerator),
                                meter_denominator: tempo.map_or(4, |tempo| tempo.meter_denominator),
                            }
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
        self.target_bpm_of(&self.track_grid)
    }

    pub fn target_bpm_of(
        &self,
        track_grid: &LoopTrackGrid,
    ) -> cmrt_loop_browser_domain::time_stretch::TargetBpm {
        cmrt_loop_browser_domain::time_stretch::select_target_bpm_with_override(
            track_grid
                .iter()
                .flatten()
                .filter_map(Option::as_ref)
                .filter_map(|clip| {
                    self.analysis_for_wav(&clip.wav)
                        .and_then(|analysis| analysis.tempo)
                        .map(|tempo| tempo.bpm)
                }),
            self.bpm_mode.manual(),
        )
    }

    /// 1 小節の秒数。one-shot の span を決めるのに要る。
    ///
    /// 参照するのは tempo を持つ clip（＝loop）だけで、one-shot は target BPM にも拍子にも
    /// 関与しない。だから「one-shot の span を測るために one-shot の span が要る」という
    /// 循環にはならない。
    pub fn measure_seconds_of(&self, track_grid: &LoopTrackGrid) -> f64 {
        let (numerator, denominator) = self.grid_meter_of(track_grid);
        let seconds = 60.0 / self.target_bpm_of(track_grid).bpm * f64::from(numerator) * 4.0
            / f64::from(denominator);
        if seconds.is_finite() && seconds > 0.0 {
            seconds
        } else {
            2.0
        }
    }

    fn grid_meter_of(&self, track_grid: &LoopTrackGrid) -> (u16, u16) {
        let measures = track_grid.iter().map(Vec::len).max().unwrap_or(0);
        for measure in 0..measures {
            for track in track_grid {
                let tempo = track
                    .get(measure)
                    .and_then(Option::as_ref)
                    .and_then(|clip| self.analysis_for_wav(&clip.wav))
                    .and_then(|analysis| analysis.tempo);
                if let Some(tempo) = tempo {
                    return (tempo.meter_numerator.max(1), tempo.meter_denominator.max(1));
                }
            }
        }
        (4, 4)
    }

    /// grid へ置くときの span（小節数）。解析が無い WAV は `None`（呼び出し側の既定に任せる）。
    pub fn span_for_wav(&self, wav: &LoopWavId) -> Option<usize> {
        self.span_for_wav_at(wav, self.measure_seconds())
    }

    pub fn measure_seconds(&self) -> f64 {
        self.measure_seconds_of(&self.track_grid)
    }

    /// グリッドを1巡するのにかかる秒数。
    /// 1小節の秒数 × grid の小節数。one-shot で伸びた span も `measure_count_of` が拾う。
    ///
    /// `displayed_measure_count()` は measure cursor に引きずられて伸びるので使わない。
    /// 実際に鳴るのは `effective_measure_count()` のぶんだけ。
    pub fn cycle_seconds(&self) -> f64 {
        self.measure_seconds() * self.effective_measure_count() as f64
    }

    /// loop は解析の小節数、one-shot は「鳴り終わるまでの小節数を 2 の冪へ切り上げ」。
    /// one-shot を 1/2/4/8… の区切りでだけ鳴らし直し、長い空白も途中の重なりも作らないため。
    pub fn span_for_wav_at(&self, wav: &LoopWavId, measure_seconds: f64) -> Option<usize> {
        let analysis = self.analysis_for_wav(wav)?;
        if analysis.kind != cmrt_loop_domain::loop_wav_analysis::LoopWavKind::OneShot {
            return Some(analysis.measures.max(1));
        }
        Some(one_shot_span_measures(
            analysis.duration_seconds,
            measure_seconds,
        ))
    }

    /// 編集操作のあとの整形。**one-shot の span だけ**を今のテンポで引き直し、
    /// previous マーカーを敷き直す。one-shot の span は 1 小節の秒数に依存するので、
    /// grid が変わるたびに引き直す必要がある。
    /// loop の span は grid に入っている値を尊重する（解析値との突き合わせは reload の仕事）。
    pub fn renormalized(&self, track_grid: &LoopTrackGrid) -> LoopTrackGrid {
        let measure_seconds = self.measure_seconds_of(track_grid);
        self.reflowed(track_grid, |browser, wav| {
            browser.one_shot_span_for_wav_at(wav, measure_seconds)
        })
    }

    /// reload / migration 時の整形。loop は解析の小節数、one-shot はテンポから、と
    /// 全 clip の span を引き直す。
    pub fn reflowed_from_analysis(&self, track_grid: &LoopTrackGrid) -> LoopTrackGrid {
        let measure_seconds = self.measure_seconds_of(track_grid);
        self.reflowed(track_grid, |browser, wav| {
            browser.span_for_wav_at(wav, measure_seconds)
        })
    }

    fn reflowed(
        &self,
        track_grid: &LoopTrackGrid,
        span_for: impl Fn(&Self, &LoopWavId) -> Option<usize>,
    ) -> LoopTrackGrid {
        let (reflowed, _) =
            cmrt_loop_browser_domain::track_grid::reflow_with_spans(track_grid, |wav| {
                span_for(self, wav)
            });
        cmrt_loop_browser_domain::track_grid::normalize_previous_markers(&reflowed).0
    }

    fn one_shot_span_for_wav_at(&self, wav: &LoopWavId, measure_seconds: f64) -> Option<usize> {
        let analysis = self.analysis_for_wav(wav)?;
        (analysis.kind == cmrt_loop_domain::loop_wav_analysis::LoopWavKind::OneShot)
            .then(|| one_shot_span_measures(analysis.duration_seconds, measure_seconds))
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
        measure_count_of(&self.track_grid)
    }
}

/// 壊れた解析値でグリッドの軸が暴走しないための歯止め。機能としての上限ではない。
const MAX_ONE_SHOT_SPAN_MEASURES: usize = 1_024;

fn one_shot_span_measures(duration_seconds: f64, measure_seconds: f64) -> usize {
    if !duration_seconds.is_finite()
        || duration_seconds <= 0.0
        || !measure_seconds.is_finite()
        || measure_seconds <= 0.0
    {
        return 1;
    }
    let measures = (duration_seconds / measure_seconds).ceil();
    if !measures.is_finite() || measures <= 1.0 {
        return 1;
    }
    (measures as usize)
        .min(MAX_ONE_SHOT_SPAN_MEASURES)
        .next_power_of_two()
}

fn measure_count_of(track_grid: &LoopTrackGrid) -> usize {
    track_grid
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
