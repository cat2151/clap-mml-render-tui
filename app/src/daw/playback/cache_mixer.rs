use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::daw::{CacheState, CellCache, FIRST_PLAYABLE_TRACK};

use super::measure_mixer::{PlaybackMeasureAudio, PlaybackMeasureSource};

/// キャッシュ参照結果として、再生に使う合算済みサンプルとヒットした track 一覧を保持する。
#[derive(Clone)]
pub(in crate::daw) struct CachedMeasureSamples {
    pub(in crate::daw) samples: Vec<f32>,
    pub(in crate::daw) cached_tracks: Vec<usize>,
}

/// 再生用サンプルが 1 小節に満たない場合だけ無音で末尾を埋める。
///
/// 余韻を保持するため、`measure_samples` を超えるぶんは切り捨てない。
pub(in crate::daw) fn pad_playback_measure_samples(
    mut samples: Vec<f32>,
    measure_samples: usize,
) -> Vec<f32> {
    if samples.len() < measure_samples {
        samples.resize(measure_samples, 0.0);
    }
    samples
}

/// キャッシュ済みのサンプルをミックスして返す。
///
/// 指定小節（`measure`、1始まり）のすべての playable track（`FIRST_PLAYABLE_TRACK..tracks`）の
/// キャッシュを調べ、合算したサンプルを返す。
/// いずれかの playable track に再生可能なサンプルがまだ無い場合は `None` を返し、
/// 呼び出し元はフレッシュレンダリングにフォールバックすること。
///
/// `Pending` / `Rendering` でも直前世代のサンプルを保持している間はそれを優先して返す。
/// これにより再 render 中でも future chunk append を止めず、古いキャッシュで継続再生できる。
/// 全 playable track が `Empty` の場合は無音（ゼロ埋め）を返す。
/// 結果は少なくとも `measure_samples` 長になり、各トラックの余韻が残っていればその末尾も保持する。
pub(in crate::daw) fn try_get_cached_samples(
    cache: &Arc<Mutex<Vec<Vec<CellCache>>>>,
    measure: usize,
    measure_samples: usize,
    tracks: usize,
) -> Option<CachedMeasureSamples> {
    // ロック下では Arc ハンドルの収集のみ行い、ミックス処理はロック外で実施する。
    // これによりキャッシュワーカーや UI スレッドとのロック競合を最小化する。
    let track_samples: Option<Vec<(usize, Option<Arc<Vec<f32>>>)>> = {
        let cache = cache.lock().unwrap();
        let mut result = Vec::with_capacity(tracks - FIRST_PLAYABLE_TRACK);
        for t in FIRST_PLAYABLE_TRACK..tracks {
            match cache[t][measure].state {
                CacheState::Empty => {
                    result.push((t, None)); // 空トラック
                }
                CacheState::Ready | CacheState::Pending | CacheState::Rendering => {
                    // samples が None の場合（サイズ上限超過や初回未完成等）はフォールバック
                    let cell = &cache[t][measure];
                    let arc = cell.samples.clone();
                    arc.as_ref()?;
                    if cell.rendered_measure_samples != Some(measure_samples) {
                        return None;
                    }
                    result.push((t, arc));
                }
                CacheState::Error => {
                    // エラー中のセルはステールサンプルに頼らずフォールバックする。
                    return None;
                }
            }
        }
        Some(result)
    };

    let track_samples = track_samples?;

    // ロック外でミックス処理を行う
    let mixed_len = track_samples
        .iter()
        .filter_map(|(_, maybe_samples)| maybe_samples.as_ref().map(|samples| samples.len()))
        .fold(measure_samples, usize::max);
    let mut mixed = vec![0.0f32; mixed_len];
    let mut any_ready = false;
    let mut cached_tracks = Vec::new();

    for (track, maybe_samples) in &track_samples {
        let Some(samples) = maybe_samples else {
            continue;
        };
        any_ready = true;
        cached_tracks.push(*track);
        for i in 0..samples.len() {
            mixed[i] += samples[i];
        }
    }

    if !any_ready {
        // すべての playable track が Empty → 空トラックのみの小節として無音を返す
        return Some(CachedMeasureSamples {
            samples: mixed,
            cached_tracks,
        });
    }

    Some(CachedMeasureSamples {
        samples: mixed,
        cached_tracks,
    })
}

pub(in crate::daw::playback) fn build_playback_measure_samples<F, E>(
    cache: &Arc<Mutex<Vec<Vec<CellCache>>>>,
    measure_index: usize,
    mml: &str,
    measure_samples: usize,
    tracks: usize,
    log_lines: &Arc<Mutex<VecDeque<String>>>,
    render_fallback: F,
) -> Result<PlaybackMeasureAudio, E>
where
    F: FnOnce() -> Result<Vec<f32>, E>,
{
    let measure_number = measure_index + 1;

    if mml.trim().is_empty() {
        crate::logging::append_log_line(
            log_lines,
            format!("meas{measure_number}: empty -> silence"),
        );
        return Ok(PlaybackMeasureAudio {
            samples: vec![0.0f32; measure_samples],
            source: PlaybackMeasureSource::Empty,
        });
    }

    if let Some(cached) = try_get_cached_samples(cache, measure_number, measure_samples, tracks) {
        let cache_entries = if cached.cached_tracks.is_empty() {
            "empty-tracks".to_string()
        } else {
            cached
                .cached_tracks
                .iter()
                .map(|track| format!("track{track}/meas{measure_number}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        crate::logging::append_log_line(
            log_lines,
            format!("meas{measure_number}: cache hit {cache_entries}"),
        );
        return Ok(PlaybackMeasureAudio {
            samples: cached.samples,
            source: PlaybackMeasureSource::Cache {
                tracks: cached.cached_tracks,
            },
        });
    }

    crate::logging::append_log_line(log_lines, format!("meas{measure_number}: render"));
    let samples = pad_playback_measure_samples(render_fallback()?, measure_samples);
    Ok(PlaybackMeasureAudio {
        samples,
        source: PlaybackMeasureSource::Render,
    })
}
