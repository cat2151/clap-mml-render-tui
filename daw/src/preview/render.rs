use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cmrt_core::NativeRenderProbeContext;
use cmrt_tui_core::mixer::auto_trim::{auto_trim_volumes_db, measure_track_level, TrackLevel};

use super::super::render_queue::{RenderPriority, RenderQueue};
use super::super::{
    DawPlayState, PlayPosition, MAX_CACHED_SAMPLES, OVERLAY_PREVIEW_CACHE_MAX_ENTRIES,
};
use super::cached_samples::pad_playback_measure_samples;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PreviewRenderProgressPhase {
    Started,
    Done { elapsed_ms: u128 },
    Error { elapsed_ms: u128 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PreviewRenderProgress {
    pub(crate) track: usize,
    pub(crate) completed: usize,
    pub(crate) total: usize,
    pub(crate) phase: PreviewRenderProgressPhase,
}

pub(crate) struct MixedPreviewRenderRequest<'a> {
    pub(crate) priority: RenderPriority,
    pub(crate) measure_samples: usize,
    pub(crate) active_tracks: &'a [usize],
    pub(crate) track_mmls: &'a [String],
    pub(crate) track_gains: &'a [f32],
    /// track ごとの音量差を測って mixer 初期値ぶんの補正を掛けるか。
    /// 素の音量（gain 1.0）で render する Grid history preview だけが `true`。
    /// ユーザーが調整済みの gain を持つ DAW の preview では二重補正になるため `false`。
    pub(crate) auto_trim: bool,
}

impl MixedPreviewRenderRequest<'_> {
    fn track_count(&self) -> usize {
        self.track_mmls.len().max(self.track_gains.len())
    }
}

/// 合成済みの preview 音声と、そのとき決まった mixer 初期値。
pub(crate) struct MixedPreviewRender {
    pub(crate) samples: Vec<f32>,
    /// [`MixedPreviewRenderRequest::auto_trim`] が `true` のときだけ入る、track ごとの dB。
    /// `samples` にはこの補正が既に掛かっている。
    pub(crate) auto_trim_volumes_db: Option<Vec<i32>>,
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
pub(crate) fn overlay_preview_cache_key(
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

/// Preview snapshot cache へエントリを挿入する。
///
/// エントリ上限を超えて新規キーを入れるときは、古い preview 条件を一括破棄してから
/// 新しい結果を入れる単純な eviction 戦略にしている。
///
/// 値の型は呼び出し側ごとに違う（音声だけ／音声と mixer 初期値の組）ため、
/// サイズ上限の判定に使うサンプル数だけを別で受け取る。
pub(crate) fn insert_overlay_preview_cache<T>(
    cache: &mut HashMap<u64, T>,
    key: u64,
    sample_count: usize,
    entry: T,
) {
    if sample_count > MAX_CACHED_SAMPLES {
        return;
    }
    if cache.len() >= OVERLAY_PREVIEW_CACHE_MAX_ENTRIES && !cache.contains_key(&key) {
        cache.clear();
    }
    cache.insert(key, entry);
}

/// 指定された preview 用 track MML 群をオフラインレンダリングし、track ごとの gain を掛けて
/// 1 本のステレオバッファへ合成して返す。
/// 各 track のレンダリング結果は `measure_samples` 未満なら末尾を埋めて長さを揃える。
pub(crate) fn render_mixed_preview_tracks<F, P>(
    render_queue: &RenderQueue,
    request: MixedPreviewRenderRequest<'_>,
    mut build_probe_context: F,
    mut report_progress: P,
) -> Option<MixedPreviewRender>
where
    F: FnMut(usize, &str) -> NativeRenderProbeContext,
    P: FnMut(PreviewRenderProgress),
{
    let total = request.active_tracks.len();
    let (response_tx, response_rx) = std::sync::mpsc::channel();
    let mut pending = HashMap::new();
    for track in request.active_tracks {
        let gain = request.track_gains.get(*track).copied().unwrap_or(1.0);
        let mml = request
            .track_mmls
            .get(*track)
            .map(String::as_str)
            .unwrap_or_default();
        report_progress(PreviewRenderProgress {
            track: *track,
            completed: 0,
            total,
            phase: PreviewRenderProgressPhase::Started,
        });
        let started = std::time::Instant::now();
        let probe_context = build_probe_context(*track, mml);
        let request_id = render_queue.reserve_request_id();
        if render_queue
            .submit_with_id(
                request_id,
                request.priority,
                mml.to_string(),
                probe_context,
                response_tx.clone(),
            )
            .is_err()
        {
            report_progress(PreviewRenderProgress {
                track: *track,
                completed: 1,
                total,
                phase: PreviewRenderProgressPhase::Error {
                    elapsed_ms: started.elapsed().as_millis(),
                },
            });
            return None;
        }
        pending.insert(request_id, (*track, gain, started));
    }
    drop(response_tx);

    let mut completed = 0;
    let mut rendered = HashMap::new();
    let mut failed = false;
    while !pending.is_empty() {
        let result = response_rx.recv().ok()?;
        let (track, gain, started) = pending.remove(&result.request_id)?;
        completed += 1;
        let elapsed_ms = started.elapsed().as_millis();
        let samples = match result.result {
            Ok(samples) => {
                report_progress(PreviewRenderProgress {
                    track,
                    completed,
                    total,
                    phase: PreviewRenderProgressPhase::Done { elapsed_ms },
                });
                pad_playback_measure_samples(samples, request.measure_samples)
            }
            Err(_) => {
                report_progress(PreviewRenderProgress {
                    track,
                    completed,
                    total,
                    phase: PreviewRenderProgressPhase::Error { elapsed_ms },
                });
                failed = true;
                continue;
            }
        };
        rendered.insert(track, (gain, samples));
    }
    if failed {
        return None;
    }

    let auto_trim_volumes_db = request.auto_trim.then(|| {
        let levels: Vec<TrackLevel> = request
            .active_tracks
            .iter()
            .filter_map(|track| {
                let (_, samples) = rendered.get(track)?;
                measure_track_level(*track, samples)
            })
            .collect();
        auto_trim_volumes_db(&levels, request.track_count())
    });

    let mut mixed = vec![0.0f32; request.measure_samples];
    for track in request.active_tracks {
        let (gain, samples) = rendered.remove(track)?;
        // 測った補正は呼び出し側の gain へ掛け合わせる。これで preview から聞こえる
        // バランスが、そのまま import 後の mixer 初期値のバランスになる。
        let gain = match &auto_trim_volumes_db {
            Some(volumes_db) => {
                gain * cmrt_tui_core::mixer::volume_db_to_gain(
                    volumes_db.get(*track).copied().unwrap_or(0),
                )
            }
            None => gain,
        };
        if mixed.len() < samples.len() {
            mixed.resize(samples.len(), 0.0);
        }
        for (index, sample) in samples.iter().enumerate() {
            mixed[index] += *sample * gain;
        }
    }
    Some(MixedPreviewRender {
        samples: mixed,
        auto_trim_volumes_db,
    })
}
