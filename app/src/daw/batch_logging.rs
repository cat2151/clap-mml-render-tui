//! DAW キャッシュ再レンダリングの進捗ログ管理

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use super::{playback, playback_util, CacheJob, CacheState, DawApp, PlayPosition};

#[derive(Clone)]
pub(super) struct TrackRerenderBatch {
    pub(super) pending: BTreeMap<usize, CacheJob>,
    pub(super) active_measure: Option<usize>,
    pub(super) completion_log: String,
}

pub(super) struct TrackRerenderBatchCompletionContext {
    pub(super) batches: Arc<Mutex<Vec<Option<TrackRerenderBatch>>>>,
    pub(super) log_lines: Arc<Mutex<VecDeque<String>>>,
    pub(super) cache: Arc<Mutex<Vec<Vec<super::CellCache>>>>,
    pub(super) play_position: Arc<Mutex<Option<PlayPosition>>>,
    pub(super) ab_repeat: Arc<Mutex<super::AbRepeatState>>,
    pub(super) play_measure_mmls: Arc<Mutex<Vec<String>>>,
    pub(super) cache_tx: std::sync::mpsc::Sender<CacheJob>,
}

fn format_measure_order(measures: &[usize]) -> String {
    measures
        .iter()
        .map(|measure| format!("meas{measure}"))
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn prioritized_measure_order(
    measures: impl IntoIterator<Item = usize>,
    play_position: Option<PlayPosition>,
    ab_repeat: &Arc<Mutex<super::AbRepeatState>>,
    play_measure_mmls: &[String],
) -> Vec<usize> {
    let mut ordered: Vec<usize> = measures.into_iter().collect();
    ordered.sort_unstable();
    let Some(position) = play_position else {
        return ordered;
    };
    let Some(effective_count) = playback_util::effective_measure_count(play_measure_mmls) else {
        return ordered;
    };
    let ab_repeat_range = (*ab_repeat.lock().unwrap()).normalized_range(effective_count);
    let current_measure_index = playback::current_play_measure_index(
        position.measure_index,
        effective_count,
        ab_repeat_range,
    );
    let next_measure =
        playback::following_measure_index(current_measure_index, effective_count, ab_repeat_range)
            + 1;
    ordered.sort_by_key(|&measure| {
        // play 中は「現在演奏中の次 meas」から順に 1 周する距離で優先度を決める。
        // これにより meas7 演奏中なら meas8 を先頭にし、その後 meas1, meas2... と続く。
        // ループ終端の外側にある小節は現在の再生到達に寄与しないため末尾に回す。
        let loop_distance = if measure <= effective_count {
            (measure + effective_count - next_measure) % effective_count
        } else {
            effective_count + measure
        };
        (loop_distance, measure)
    });
    ordered
}

fn take_next_batch_job(
    pending: &mut BTreeMap<usize, CacheJob>,
    track: usize,
    cache: &Arc<Mutex<Vec<Vec<super::CellCache>>>>,
    play_position: Option<PlayPosition>,
    ab_repeat: &Arc<Mutex<super::AbRepeatState>>,
    play_measure_mmls: &[String],
) -> Option<(usize, CacheJob, String)> {
    let priority_order = prioritized_measure_order(
        pending.keys().copied(),
        play_position,
        ab_repeat,
        play_measure_mmls,
    );
    let valid_order: Vec<usize> = {
        let cache = cache.lock().unwrap();
        priority_order
            .iter()
            .copied()
            .filter(|measure| {
                pending.get(measure).is_some_and(|job| {
                    let cell = &cache[track][*measure];
                    cell.state != CacheState::Empty && cell.generation == job.generation
                })
            })
            .collect()
    };

    let valid_measure_set: BTreeSet<usize> = valid_order.iter().copied().collect();
    for measure in &priority_order {
        if !valid_measure_set.contains(measure) {
            pending.remove(measure);
        }
    }

    let first_measure = valid_order.first().copied()?;
    let first_job = pending.remove(&first_measure)?;
    Some((first_measure, first_job, format_measure_order(&valid_order)))
}

impl DawApp {
    pub(super) fn start_track_rerender_batch(
        &self,
        track: usize,
        measures: &[usize],
        reason: &str,
    ) {
        if track == 0 {
            return;
        }
        let pending: BTreeMap<usize, CacheJob> = measures
            .iter()
            .copied()
            .filter_map(|measure| {
                self.prepare_cache_job(track, measure)
                    .map(|job| (measure, job))
            })
            .collect();
        if pending.is_empty() {
            self.track_rerender_batches.lock().unwrap()[track] = None;
            return;
        }
        let measure_range = super::playback_util::format_measure_list(measures)
            .map(|label| label.replace('～', "〜"))
            .unwrap_or_else(|| "none".to_string());
        let play_position = self.play_position.lock().unwrap().clone();
        let play_measure_mmls = self.play_measure_mmls.lock().unwrap().clone();
        let mut remaining = pending;
        let Some((first_measure, first_job, priority_order_label)) = take_next_batch_job(
            &mut remaining,
            track,
            &self.cache,
            play_position,
            &self.ab_repeat,
            &play_measure_mmls,
        ) else {
            self.track_rerender_batches.lock().unwrap()[track] = None;
            return;
        };
        self.append_log_line(format!(
            "cache: rerender start track{} {} ({reason})",
            track, measure_range
        ));
        self.append_log_line(format!(
            "cache: rerender reserve track{} meas{} ({})",
            track, first_measure, priority_order_label
        ));
        self.track_rerender_batches.lock().unwrap()[track] = Some(TrackRerenderBatch {
            pending: remaining,
            active_measure: Some(first_measure),
            completion_log: format!(
                "cache: rerender done track{} {} ({reason})",
                track, measure_range
            ),
        });
        self.mark_cache_rendering(track, first_measure);
        let _ = self.cache_tx.send(first_job);
    }

    pub(super) fn complete_track_rerender_batch_measure(
        ctx: &TrackRerenderBatchCompletionContext,
        track: usize,
        measure: usize,
    ) {
        if track == 0 || measure == 0 {
            return;
        }
        let play_position = ctx.play_position.lock().unwrap().clone();
        let play_measure_mmls = ctx.play_measure_mmls.lock().unwrap().clone();
        let (next_job, queued_log, completion_log) = {
            let mut batches = ctx.batches.lock().unwrap();
            let Some(batch) = batches.get_mut(track).and_then(Option::as_mut) else {
                return;
            };
            if batch.active_measure != Some(measure) {
                return;
            }
            batch.active_measure = None;
            if batch.pending.is_empty() {
                let completion_log = batch.completion_log.clone();
                batches[track] = None;
                (None, None, Some(completion_log))
            } else {
                if let Some((next_measure, next_job, priority_order_label)) = take_next_batch_job(
                    &mut batch.pending,
                    track,
                    &ctx.cache,
                    play_position,
                    &ctx.ab_repeat,
                    &play_measure_mmls,
                ) {
                    batch.active_measure = Some(next_measure);
                    (
                        Some(next_job),
                        Some(format!(
                            "cache: rerender reserve track{} meas{} ({})",
                            track, next_measure, priority_order_label
                        )),
                        None,
                    )
                } else {
                    let completion_log = batch.completion_log.clone();
                    batches[track] = None;
                    (None, None, Some(completion_log))
                }
            }
        };
        if let Some(queued_log) = queued_log {
            crate::logging::append_log_line(&ctx.log_lines, queued_log);
        }
        if let Some(next_job) = next_job {
            Self::mark_cache_rendering_in(&ctx.cache, next_job.track, next_job.measure);
            let _ = ctx.cache_tx.send(next_job);
        }
        if let Some(completion_log) = completion_log {
            crate::logging::append_log_line(&ctx.log_lines, completion_log);
        }
    }
}
