use super::*;
use cmrt_loop_browser_domain::random::LoopRandomScope;
use cmrt_tui_core::bpm::BpmMode;
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

/// 抽選しただけでまだ確定していないグリッド。
/// `Shift+R` は即 commit するが、オートランダムは先読みが鳴り始めるまで commit を待つ。
pub struct StagedRandomGrid {
    pub grid: LoopTrackGrid,
    pub decks: LoopRandomDeckState,
    pub start_measure: usize,
    /// このグリッドを組み立てたときの BPM。commit で初めて画面へ反映する。
    /// 先に反映してしまうと、鳴らずに捨てられたときテンポだけが変わって残る。
    pub mode: BpmMode,
}

impl LoopBrowser {
    pub fn randomize_current_measure(&mut self) -> LoopBrowserAction {
        // 手動の一括randomではテンポを動かさない（引き直すのはオートランダムだけ）。
        let Some(staged) = self.stage_random_grid(self.bpm_mode) else {
            return LoopBrowserAction::Continue;
        };
        let start_measure = staged.start_measure;
        if !self.commit_random_grid(staged) {
            return LoopBrowserAction::Continue;
        }
        LoopBrowserAction::GridReplaced {
            start_measure,
            grid: self.playback_grid(),
            reason: LoopGridChange::BatchRandom,
        }
    }

    /// 全 track を引き直した結果を、`mode` のテンポ前提で作る。デッキのクローンに対して
    /// 抽選するので、結果を捨てても副作用は残らない。
    ///
    /// 抽選中だけ `bpm_mode` を差し替えるのは、clip の span も「BPM範囲外」の赤判定も
    /// target BPM から逆算されるため。`self` を通す全ての計算が同じテンポを見る必要がある。
    /// 差し替えは必ずここで巻き戻し、確定は [`Self::commit_random_grid`] に任せる。
    pub fn stage_random_grid(&mut self, mode: BpmMode) -> Option<StagedRandomGrid> {
        let previous_mode = std::mem::replace(&mut self.bpm_mode, mode);
        let staged = self.stage_random_grid_at_current_bpm();
        self.bpm_mode = previous_mode;
        staged
    }

    fn stage_random_grid_at_current_bpm(&mut self) -> Option<StagedRandomGrid> {
        if !self.track_grid_writable || !self.random_decks.writable {
            return None;
        }
        let targets = self.batch_random_targets();
        if targets.is_empty() || targets.iter().any(|target| target.candidates.is_empty()) {
            self.show_batch_random_notice("一括randomの候補がないtrackがあります");
            return None;
        }
        let Some((grid, decks)) = self.find_red_free_random_grid(&targets) else {
            self.show_batch_random_notice("BPM範囲外を解消できる組み合わせがありません");
            return None;
        };
        Some(StagedRandomGrid {
            grid,
            decks,
            start_measure: targets
                .iter()
                .map(|target| target.start_measure)
                .min()
                .unwrap_or(self.measure_cursor),
            mode: self.bpm_mode,
        })
    }

    /// 抽選結果を表示と TOML へ確定する。保存に失敗したら元の状態へ戻して `false` を返す。
    pub fn commit_random_grid(&mut self, staged: StagedRandomGrid) -> bool {
        let previous_grid = self.track_grid.clone();
        let previous_decks = self.random_decks.value.clone();
        // 抽選時のテンポをここで初めて採用する。grid とテンポは組で決まっているので、
        // 保存に失敗して grid を戻すときは bpm_mode も戻す。
        let previous_mode = std::mem::replace(&mut self.bpm_mode, staged.mode);

        self.random_decks.value = staged.decks;
        if let Err(error) = self.save_random_decks() {
            self.random_decks.value = previous_decks;
            self.bpm_mode = previous_mode;
            self.random_decks.error = Some(format!("random deckを保存できません: {error}"));
            return false;
        }
        self.random_decks.error = None;

        self.track_grid = staged.grid;
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous_grid;
            self.random_decks.value = previous_decks;
            self.bpm_mode = previous_mode;
            if let Err(rollback_error) = self.save_random_decks() {
                self.random_decks.error = Some(format!(
                    "random deckのrollbackを保存できません: {rollback_error}"
                ));
            }
            self.track_grid_error = Some(format!("track listを保存できません: {error}"));
            return false;
        }
        self.track_grid_error = None;
        self.sync_tree_to_current_cell();
        true
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
        let target_bpm = common_candidate_bpm(&intervals, self.bpm_mode)?;
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
        let grid = self.renormalized(&grid);
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
        // 差し替え後のグリッドで span を引き直す。loop が入れ替わると target BPM が動き、
        // one-shot が何小節ぶんかも変わるため。
        self.renormalized(&grid)
    }

    fn write_batch_clip(
        &self,
        grid: &mut LoopTrackGrid,
        track: usize,
        start: usize,
        wav: LoopWavId,
    ) {
        let span_measures = self.span_for_wav(&wav).unwrap_or(1).max(1);
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
        let target_bpm = self.target_bpm_of(grid).bpm;
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

/// 全 track が time stretch 範囲に収まる target BPM を1点選ぶ。
///
/// 手動モードはその1点だけを検査する（外れたら組み合わせごと諦める）。自動モードは
/// 寄せ先（範囲から引いた BPM）に最も近い実現可能な点を選ぶので、引いた BPM が
/// そのままでは無理でも近いテンポへ落として組み合わせを成立させられる。
fn common_candidate_bpm(
    candidates: &[Vec<(LoopWavId, BpmInterval)>],
    mode: BpmMode,
) -> Option<f64> {
    if candidates.iter().any(Vec::is_empty) {
        return None;
    }
    let target = mode.bpm();
    let mut points = vec![target];
    if mode.manual().is_some() {
        return points.into_iter().find(|point| {
            candidates
                .iter()
                .all(|track| track.iter().any(|(_, interval)| interval.contains(*point)))
        });
    }
    for (_, interval) in candidates.iter().flatten() {
        if interval.minimum.is_finite() {
            points.push(interval.minimum);
        }
        if interval.maximum.is_finite() {
            points.push(interval.maximum);
        }
    }
    points.sort_by(|left, right| {
        (left - target)
            .abs()
            .total_cmp(&(right - target).abs())
            .then_with(|| left.total_cmp(right))
    });
    points.into_iter().find(|point| {
        candidates
            .iter()
            .all(|track| track.iter().any(|(_, interval)| interval.contains(*point)))
    })
}
