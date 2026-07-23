use super::*;
use cmrt_loop_browser_domain::random::LoopRandomScope;
use std::collections::HashSet;

#[derive(Clone)]
struct BatchRandomTarget {
    track: usize,
    start_measure: usize,
    scope: LoopRandomScope,
    candidates: Vec<LoopWavId>,
    current: Option<LoopWavId>,
}

#[derive(Clone, Copy)]
struct BpmInterval {
    minimum: f64,
    maximum: f64,
}

impl BpmInterval {
    fn contains(self, bpm: f64) -> bool {
        self.minimum <= bpm && bpm <= self.maximum
    }
}

impl LoopBrowser {
    pub fn randomize_current_measure(&mut self) -> LoopBrowserAction {
        if !self.track_grid_writable || !self.random_decks.writable {
            return LoopBrowserAction::Continue;
        }
        let targets = self.batch_random_targets();
        if targets.is_empty() || targets.iter().any(|target| target.candidates.is_empty()) {
            self.show_batch_random_notice("一括randomの候補がないtrackがあります");
            return LoopBrowserAction::Continue;
        }

        let previous_grid = self.track_grid.clone();
        let previous_decks = self.random_decks.value.clone();
        let Some((next_grid, next_decks)) = self.find_red_free_random_grid(&targets) else {
            self.show_batch_random_notice("BPM範囲外を解消できる組み合わせがありません");
            return LoopBrowserAction::Continue;
        };

        self.random_decks.value = next_decks;
        if let Err(error) = self.save_random_decks() {
            self.random_decks.value = previous_decks;
            self.random_decks.error = Some(format!("random deckを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.random_decks.error = None;

        self.track_grid = next_grid;
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous_grid;
            self.random_decks.value = previous_decks;
            if let Err(rollback_error) = self.save_random_decks() {
                self.random_decks.error = Some(format!(
                    "random deckのrollbackを保存できません: {rollback_error}"
                ));
            }
            self.track_grid_error = Some(format!("track listを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.track_grid_error = None;
        self.sync_tree_to_current_cell();

        LoopBrowserAction::GridReplaced {
            start_measure: targets
                .iter()
                .map(|target| target.start_measure)
                .min()
                .unwrap_or(self.measure_cursor),
            grid: self.playback_grid(),
            reason: LoopGridChange::BatchRandom,
        }
    }

    fn batch_random_targets(&self) -> Vec<BatchRandomTarget> {
        (0..self.track_grid.len())
            .map(|track| {
                let occupied = self
                    .clip_at(track, self.measure_cursor)
                    .map(|(start, clip)| (start, clip.wav.clone()));
                let scope = occupied
                    .as_ref()
                    .and_then(|(_, wav)| self.category_for_wav(wav))
                    .map(|category| LoopRandomScope::Category {
                        category: category.to_string(),
                    })
                    .unwrap_or(LoopRandomScope::All);
                BatchRandomTarget {
                    track,
                    start_measure: occupied
                        .as_ref()
                        .map_or(self.measure_cursor, |(start, _)| *start),
                    candidates: self.random_candidates(&scope),
                    current: occupied.map(|(_, wav)| wav),
                    scope,
                }
            })
            .collect()
    }

    fn find_red_free_random_grid(
        &self,
        targets: &[BatchRandomTarget],
    ) -> Option<(LoopTrackGrid, LoopRandomDeckState)> {
        let intervals = targets
            .iter()
            .map(|target| {
                target
                    .candidates
                    .iter()
                    .filter_map(|candidate| {
                        self.interval_for_candidate(target, candidate)
                            .map(|interval| (candidate.clone(), interval))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let target_bpm = common_candidate_bpm(&intervals)?;
        let mut decks = self.random_decks.value.clone();
        let mut selections = Vec::with_capacity(targets.len());
        for (target, candidates) in targets.iter().zip(&intervals) {
            let mut selected = decks.draw(
                target.scope.clone(),
                &target.candidates,
                target.current.as_ref(),
            )?;
            for _ in 0..target.candidates.len() {
                if candidates.iter().any(|(candidate, interval)| {
                    candidate.matches(&selected) && interval.contains(target_bpm)
                }) {
                    break;
                }
                selected = decks.draw(target.scope.clone(), &target.candidates, Some(&selected))?;
            }
            if !candidates.iter().any(|(candidate, interval)| {
                candidate.matches(&selected) && interval.contains(target_bpm)
            }) {
                return None;
            }
            selections.push(selected);
        }

        let grid = self.grid_with_batch_selections(targets, &selections);
        self.bpm_red_tracks(&grid)
            .is_empty()
            .then_some((grid, decks))
    }

    fn interval_for_candidate(
        &self,
        target: &BatchRandomTarget,
        candidate: &LoopWavId,
    ) -> Option<BpmInterval> {
        let mut grid = self.track_grid.clone();
        self.write_batch_clip(
            &mut grid,
            target.track,
            target.start_measure,
            candidate.clone(),
        );
        let grid = cmrt_loop_browser_domain::track_grid::normalize_previous_markers(&grid).0;
        bpm_interval(
            grid[target.track]
                .iter()
                .filter_map(Option::as_ref)
                .filter_map(|clip| self.source_bpm_for_red(clip)),
        )
    }

    fn grid_with_batch_selections(
        &self,
        targets: &[BatchRandomTarget],
        selections: &[LoopWavId],
    ) -> LoopTrackGrid {
        let mut grid = self.track_grid.clone();
        for (target, selected) in targets.iter().zip(selections) {
            self.write_batch_clip(
                &mut grid,
                target.track,
                target.start_measure,
                selected.clone(),
            );
        }
        cmrt_loop_browser_domain::track_grid::normalize_previous_markers(&grid).0
    }

    fn write_batch_clip(
        &self,
        grid: &mut LoopTrackGrid,
        track: usize,
        start: usize,
        wav: LoopWavId,
    ) {
        let span_measures = self
            .analysis_for_wav(&wav)
            .map(|analysis| analysis.measures)
            .unwrap_or(1)
            .max(1);
        let end = start.saturating_add(span_measures);
        if grid.first().is_some_and(|row| end > row.len()) {
            for row in grid.iter_mut() {
                row.resize(end, None);
            }
        }
        for measure in 0..grid[track].len() {
            let overlaps = grid[track][measure].as_ref().is_some_and(|clip| {
                measure < end && measure.saturating_add(clip.span_measures) > start
            });
            if overlaps {
                grid[track][measure] = None;
            }
        }
        grid[track][start] = Some(LoopTrackClip::explicit(wav, span_measures));
    }

    fn bpm_red_tracks(&self, grid: &LoopTrackGrid) -> HashSet<usize> {
        let target_bpm = cmrt_loop_browser_domain::time_stretch::select_target_bpm(
            grid.iter()
                .flatten()
                .filter_map(Option::as_ref)
                .filter_map(|clip| self.source_bpm_for_red(clip)),
        )
        .bpm;
        grid.iter()
            .enumerate()
            .filter_map(|(track, cells)| {
                cells
                    .iter()
                    .filter_map(Option::as_ref)
                    .any(|clip| {
                        self.source_bpm_for_red(clip).is_some_and(|source_bpm| {
                            cmrt_loop_browser_domain::time_stretch::exceeds_time_ratio_limits(
                                source_bpm, target_bpm,
                            )
                        })
                    })
                    .then_some(track)
            })
            .collect()
    }

    fn source_bpm_for_red(&self, clip: &LoopTrackClip) -> Option<f64> {
        self.analysis_for_wav(&clip.wav).and_then(|analysis| {
            (analysis.kind != cmrt_loop_domain::loop_wav_analysis::LoopWavKind::OneShot)
                .then(|| analysis.tempo.map(|tempo| tempo.bpm))
                .flatten()
                .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
        })
    }

    fn show_batch_random_notice(&mut self, text: &str) {
        self.notice = Some(LoopBrowserNotice {
            text: text.to_string(),
            expires_at: Instant::now() + REMOVED_NOTICE_DURATION,
        });
    }
}

fn bpm_interval(bpms: impl IntoIterator<Item = f64>) -> Option<BpmInterval> {
    let mut interval = BpmInterval {
        minimum: 0.0,
        maximum: f64::INFINITY,
    };
    for bpm in bpms {
        interval.minimum = interval
            .minimum
            .max(bpm / cmrt_loop_browser_domain::time_stretch::MAX_TIME_RATIO);
        interval.maximum = interval
            .maximum
            .min(bpm / cmrt_loop_browser_domain::time_stretch::MIN_TIME_RATIO);
    }
    (interval.minimum <= interval.maximum).then_some(interval)
}

fn common_candidate_bpm(candidates: &[Vec<(LoopWavId, BpmInterval)>]) -> Option<f64> {
    if candidates.iter().any(Vec::is_empty) {
        return None;
    }
    let mut points = vec![cmrt_loop_browser_domain::time_stretch::TARGET_BPM];
    for (_, interval) in candidates.iter().flatten() {
        if interval.minimum.is_finite() {
            points.push(interval.minimum);
        }
        if interval.maximum.is_finite() {
            points.push(interval.maximum);
        }
    }
    points.sort_by(|left, right| {
        (left - cmrt_loop_browser_domain::time_stretch::TARGET_BPM)
            .abs()
            .total_cmp(&(right - cmrt_loop_browser_domain::time_stretch::TARGET_BPM).abs())
            .then_with(|| left.total_cmp(right))
    });
    points.into_iter().find(|point| {
        candidates
            .iter()
            .all(|track| track.iter().any(|(_, interval)| interval.contains(*point)))
    })
}
