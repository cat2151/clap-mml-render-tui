use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cmrt_core::NativeRenderProbeContext;

use super::super::playback::pad_playback_measure_samples;
use super::super::render_queue::{RenderPriority, RenderQueue};
use super::super::{
    DawPlayState, PlayPosition, MAX_CACHED_SAMPLES, OVERLAY_PREVIEW_CACHE_MAX_ENTRIES,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PreviewRenderProgressPhase {
    Started,
    Done { elapsed_ms: u128 },
    Error { elapsed_ms: u128 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PreviewRenderProgress {
    pub(super) track: usize,
    pub(super) completed: usize,
    pub(super) total: usize,
    pub(super) phase: PreviewRenderProgressPhase,
}

pub(super) struct MixedPreviewRenderRequest<'a> {
    pub(super) priority: RenderPriority,
    pub(super) measure_samples: usize,
    pub(super) active_tracks: &'a [usize],
    pub(super) track_mmls: &'a [String],
    pub(super) track_gains: &'a [f32],
}

pub(crate) struct PreviewOutputState<'a> {
    pub(crate) play_transition_lock: &'a Arc<Mutex<()>>,
    pub(crate) play_state: &'a Arc<Mutex<DawPlayState>>,
    pub(crate) play_position: &'a Arc<Mutex<Option<PlayPosition>>>,
    pub(crate) preview_session: &'a AtomicU64,
}

pub(crate) struct PreviewOutputRequest {
    pub(crate) session: u64,
    pub(crate) measure_index: usize,
    pub(crate) measure_duration: std::time::Duration,
}

pub(crate) fn begin_preview_output<F>(
    state: PreviewOutputState<'_>,
    request: PreviewOutputRequest,
    enqueue_audio: F,
) -> bool
where
    F: FnOnce(),
{
    let _transition_guard = state.play_transition_lock.lock().unwrap();
    if *state.play_state.lock().unwrap() != DawPlayState::Preview
        || state.preview_session.load(Ordering::Acquire) != request.session
    {
        return false;
    }
    *state.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: request.measure_index,
        measure_start: std::time::Instant::now(),
        measure_duration: request.measure_duration,
    });
    enqueue_audio();
    true
}

/// Preview snapshot cache 用のキーを作る。
///
/// `measure_index`、各 track の MML スナップショット、各 track gain をまとめてハッシュし、
/// 同じ preview 条件のときだけ同一キーになるようにする。
/// gain は `f32` の数値比較ではなく `to_bits()` を使ってビット列ごと区別する。
pub(super) fn overlay_preview_cache_key(
    measure_index: usize,
    track_mmls: &[String],
    track_gains: &[f32],
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    measure_index.hash(&mut hasher);
    track_mmls.hash(&mut hasher);
    track_gains
        .iter()
        .for_each(|gain| gain.to_bits().hash(&mut hasher));
    hasher.finish()
}

/// Preview snapshot cache へサンプルを挿入する。
///
/// エントリ上限を超えて新規キーを入れるときは、古い preview 条件を一括破棄してから
/// 新しい結果を入れる単純な eviction 戦略にしている。
pub(super) fn insert_overlay_preview_cache(
    cache: &mut HashMap<u64, Arc<Vec<f32>>>,
    key: u64,
    samples: Arc<Vec<f32>>,
) {
    if samples.len() > MAX_CACHED_SAMPLES {
        return;
    }
    if cache.len() >= OVERLAY_PREVIEW_CACHE_MAX_ENTRIES && !cache.contains_key(&key) {
        cache.clear();
    }
    cache.insert(key, samples);
}

/// 指定された preview 用 track MML 群をオフラインレンダリングし、track ごとの gain を掛けて
/// 1 本のステレオバッファへ合成して返す。
/// 各 track のレンダリング結果は `measure_samples` 未満なら末尾を埋めて長さを揃える。
pub(super) fn render_mixed_preview_tracks<F, P>(
    render_queue: &RenderQueue,
    request: MixedPreviewRenderRequest<'_>,
    mut build_probe_context: F,
    mut report_progress: P,
) -> Option<Vec<f32>>
where
    F: FnMut(usize, &str) -> NativeRenderProbeContext,
    P: FnMut(PreviewRenderProgress),
{
    let mut mixed = vec![0.0f32; request.measure_samples];
    let total = request.active_tracks.len();
    for (index, track) in request.active_tracks.iter().enumerate() {
        let gain = request.track_gains.get(*track).copied().unwrap_or(1.0);
        let mml = request
            .track_mmls
            .get(*track)
            .map(String::as_str)
            .unwrap_or_default();
        report_progress(PreviewRenderProgress {
            track: *track,
            completed: index,
            total,
            phase: PreviewRenderProgressPhase::Started,
        });
        let started = std::time::Instant::now();
        let probe_context = build_probe_context(*track, mml);
        let result = render_queue.render_blocking(request.priority, mml, probe_context);
        let elapsed_ms = started.elapsed().as_millis();
        let samples = match result {
            Ok(samples) => {
                report_progress(PreviewRenderProgress {
                    track: *track,
                    completed: index + 1,
                    total,
                    phase: PreviewRenderProgressPhase::Done { elapsed_ms },
                });
                pad_playback_measure_samples(samples, request.measure_samples)
            }
            Err(_) => {
                report_progress(PreviewRenderProgress {
                    track: *track,
                    completed: index + 1,
                    total,
                    phase: PreviewRenderProgressPhase::Error { elapsed_ms },
                });
                return None;
            }
        };
        if mixed.len() < samples.len() {
            mixed.resize(samples.len(), 0.0);
        }
        for (index, sample) in samples.iter().enumerate() {
            mixed[index] += *sample * gain;
        }
    }
    Some(mixed)
}
