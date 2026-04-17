use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render_for_cache_with_probe, CoreConfig, NativeRenderProbeContext};

use super::super::playback::pad_playback_measure_samples;
use super::super::{
    DawPlayState, PlayPosition, MAX_CACHED_SAMPLES, OVERLAY_PREVIEW_CACHE_MAX_ENTRIES,
};

pub(in crate::daw) fn begin_preview_output<F>(
    play_transition_lock: &Arc<Mutex<()>>,
    play_state: &Arc<Mutex<DawPlayState>>,
    play_position: &Arc<Mutex<Option<PlayPosition>>>,
    preview_session: &AtomicU64,
    session: u64,
    measure_index: usize,
    enqueue_audio: F,
) -> bool
where
    F: FnOnce(),
{
    let _transition_guard = play_transition_lock.lock().unwrap();
    if *play_state.lock().unwrap() != DawPlayState::Preview
        || preview_session.load(Ordering::Acquire) != session
    {
        return false;
    }
    *play_position.lock().unwrap() = Some(PlayPosition {
        measure_index,
        measure_start: std::time::Instant::now(),
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
pub(super) fn render_mixed_preview_tracks<F>(
    entry_ref: &PluginEntry,
    daw_cfg: &crate::config::Config,
    measure_samples: usize,
    active_tracks: &[usize],
    track_mmls: &[String],
    track_gains: &[f32],
    mut build_probe_context: F,
) -> Option<Vec<f32>>
where
    F: FnMut(usize, &str) -> NativeRenderProbeContext,
{
    let mut mixed = vec![0.0f32; measure_samples];
    for track in active_tracks {
        let gain = track_gains.get(*track).copied().unwrap_or(1.0);
        let mml = track_mmls
            .get(*track)
            .map(String::as_str)
            .unwrap_or_default();
        let probe_context = build_probe_context(*track, mml);
        let result = {
            let core_cfg = CoreConfig::from(daw_cfg);
            mml_render_for_cache_with_probe(mml, &core_cfg, entry_ref, Some(&probe_context))
        };
        let samples = result
            .ok()
            .map(|samples| pad_playback_measure_samples(samples, measure_samples))?;
        if mixed.len() < samples.len() {
            mixed.resize(samples.len(), 0.0);
        }
        for (index, sample) in samples.iter().enumerate() {
            mixed[index] += *sample * gain;
        }
    }
    Some(mixed)
}
