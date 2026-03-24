//! DAW キャッシュ再レンダリングの進捗ログ管理

use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use super::DawApp;

#[derive(Clone)]
pub(super) struct TrackRerenderBatch {
    pub(super) pending: BTreeSet<usize>,
    pub(super) completion_log: String,
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
        let mut batches = self.track_rerender_batches.lock().unwrap();
        let pending: BTreeSet<usize> = measures.iter().copied().collect();
        if pending.is_empty() {
            batches[track] = None;
            return;
        }
        let measure_range = super::playback_util::format_measure_list(measures)
            .map(|label| label.replace('～', "〜"))
            .unwrap_or_else(|| "none".to_string());
        self.append_log_line(format!(
            "cache: rerender start track{} {} ({reason})",
            track, measure_range
        ));
        batches[track] = Some(TrackRerenderBatch {
            pending,
            completion_log: format!(
                "cache: rerender done track{} {} ({reason})",
                track, measure_range
            ),
        });
    }

    pub(super) fn complete_track_rerender_batch_measure(
        batches: &Arc<Mutex<Vec<Option<TrackRerenderBatch>>>>,
        log_lines: &Arc<Mutex<VecDeque<String>>>,
        track: usize,
        measure: usize,
    ) {
        if track == 0 || measure == 0 {
            return;
        }
        let completion_log = {
            let mut batches = batches.lock().unwrap();
            let Some(batch) = batches.get_mut(track).and_then(Option::as_mut) else {
                return;
            };
            if !batch.pending.remove(&measure) {
                return;
            }
            if !batch.pending.is_empty() {
                return;
            }
            let completion_log = batch.completion_log.clone();
            batches[track] = None;
            completion_log
        };
        crate::logging::append_log_line(log_lines, completion_log);
    }
}
