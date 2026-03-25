//! DAW キャッシュ再レンダリングの進捗ログ管理

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use super::{playback, playback_util, CacheJob, DawApp, PlayPosition};

#[derive(Clone)]
pub(super) struct TrackRerenderBatch {
    pub(super) pending: BTreeMap<usize, CacheJob>,
    pub(super) active_measure: Option<usize>,
    pub(super) completion_log: String,
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
    let current_measure_index =
        playback::current_play_measure_index(position.measure_index, effective_count);
    let next_measure =
        playback::following_measure_index(current_measure_index, effective_count) + 1;
    ordered.sort_by_key(|&measure| {
        let loop_distance = if measure <= effective_count {
            (measure + effective_count - next_measure) % effective_count
        } else {
            effective_count + measure
        };
        (loop_distance, measure)
    });
    ordered
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
        let priority_order = prioritized_measure_order(
            pending.keys().copied(),
            self.play_position.lock().unwrap().clone(),
            &self.play_measure_mmls.lock().unwrap(),
        );
        let Some(first_measure) = priority_order.first().copied() else {
            self.track_rerender_batches.lock().unwrap()[track] = None;
            return;
        };
        let Some(first_job) = pending.get(&first_measure).cloned() else {
            self.track_rerender_batches.lock().unwrap()[track] = None;
            return;
        };
        let mut remaining = pending;
        remaining.remove(&first_measure);
        self.append_log_line(format!(
            "cache: rerender start track{} {} ({reason})",
            track, measure_range
        ));
        self.append_log_line(format!(
            "cache: rerender reserve track{} meas{} ({})",
            track,
            first_measure,
            format_measure_order(&priority_order)
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
        batches: &Arc<Mutex<Vec<Option<TrackRerenderBatch>>>>,
        log_lines: &Arc<Mutex<VecDeque<String>>>,
        cache: &Arc<Mutex<Vec<Vec<super::CellCache>>>>,
        play_position: &Arc<Mutex<Option<PlayPosition>>>,
        play_measure_mmls: &Arc<Mutex<Vec<String>>>,
        cache_tx: &std::sync::mpsc::Sender<CacheJob>,
        track: usize,
        measure: usize,
    ) {
        if track == 0 || measure == 0 {
            return;
        }
        let play_position = play_position.lock().unwrap().clone();
        let play_measure_mmls = play_measure_mmls.lock().unwrap().clone();
        let (next_job, queued_log, completion_log) = {
            let mut batches = batches.lock().unwrap();
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
                let priority_order = prioritized_measure_order(
                    batch.pending.keys().copied(),
                    play_position,
                    &play_measure_mmls,
                );
                if let Some(next_measure) = priority_order.first().copied() {
                    let next_job = batch.pending.remove(&next_measure);
                    batch.active_measure = Some(next_measure);
                    (
                        next_job,
                        Some(format!(
                            "cache: rerender reserve track{} meas{} ({})",
                            track,
                            next_measure,
                            format_measure_order(&priority_order)
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
            crate::logging::append_log_line(log_lines, queued_log);
        }
        if let Some(next_job) = next_job {
            Self::mark_cache_rendering_in(cache, next_job.track, next_job.measure);
            let _ = cache_tx.send(next_job);
        }
        if let Some(completion_log) = completion_log {
            crate::logging::append_log_line(log_lines, completion_log);
        }
    }
}
