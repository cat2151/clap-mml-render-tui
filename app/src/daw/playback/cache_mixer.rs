use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::daw::{CacheState, CellCache, FIRST_PLAYABLE_TRACK};

use super::measure_mixer::{PlaybackMeasureAudio, PlaybackMeasureSource};

type TrackCachedSamples = Vec<(usize, f32, Option<Arc<Vec<f32>>>)>;

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

fn mix_track_into_buffer(mixed: &mut Vec<f32>, samples: &[f32], gain: f32) {
    if mixed.len() < samples.len() {
        mixed.resize(samples.len(), 0.0);
    }
    for (index, sample) in samples.iter().enumerate() {
        mixed[index] += *sample * gain;
    }
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
    track_gains: &[f32],
) -> Option<CachedMeasureSamples> {
    // ロック下では Arc ハンドルの収集のみ行い、ミックス処理はロック外で実施する。
    // これによりキャッシュワーカーや UI スレッドとのロック競合を最小化する。
    let track_samples: Option<TrackCachedSamples> = {
        let cache = cache.lock().unwrap();
        let mut result = Vec::with_capacity(tracks - FIRST_PLAYABLE_TRACK);
        for t in FIRST_PLAYABLE_TRACK..tracks {
            let gain = track_gains.get(t).copied().unwrap_or(1.0);
            if gain == 0.0 {
                continue;
            }
            match cache[t][measure].state {
                CacheState::Empty => {
                    result.push((t, gain, None)); // 空トラック
                }
                CacheState::Ready | CacheState::Pending | CacheState::Rendering => {
                    // samples が None の場合（サイズ上限超過や初回未完成等）はフォールバック
                    let cell = &cache[t][measure];
                    let arc = cell.samples.clone();
                    arc.as_ref()?;
                    if cell.rendered_measure_samples != Some(measure_samples) {
                        return None;
                    }
                    result.push((t, gain, arc));
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
        .filter_map(|(_, _, maybe_samples)| maybe_samples.as_ref().map(|samples| samples.len()))
        .fold(measure_samples, usize::max);
    let mut mixed = vec![0.0f32; mixed_len];
    let mut any_ready = false;
    let mut cached_tracks = Vec::new();

    for (track, gain, maybe_samples) in &track_samples {
        let Some(samples) = maybe_samples else {
            continue;
        };
        any_ready = true;
        cached_tracks.push(*track);
        mix_track_into_buffer(&mut mixed, samples, *gain);
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
    track_mmls: &[String],
    measure_samples: usize,
    tracks: usize,
    track_gains: &[f32],
    log_lines: &Arc<Mutex<VecDeque<String>>>,
    mut render_fallback: F,
) -> Result<PlaybackMeasureAudio, E>
where
    F: FnMut(usize, &str) -> Result<Vec<f32>, E>,
{
    let measure_number = measure_index + 1;
    let active_tracks: Vec<usize> = (FIRST_PLAYABLE_TRACK..tracks)
        .filter(|&track| {
            track_gains.get(track).copied().unwrap_or(1.0) > 0.0
                && track_mmls
                    .get(track)
                    .map(|mml| !mml.trim().is_empty())
                    .unwrap_or(false)
        })
        .collect();

    if active_tracks.is_empty() {
        crate::logging::append_log_line(
            log_lines,
            format!("meas{measure_number}: empty -> silence"),
        );
        return Ok(PlaybackMeasureAudio {
            samples: vec![0.0f32; measure_samples],
            source: PlaybackMeasureSource::Empty,
        });
    }

    if let Some(cached) =
        try_get_cached_samples(cache, measure_number, measure_samples, tracks, track_gains)
    {
        if cached.cached_tracks.len() != active_tracks.len() {
            crate::logging::append_log_line(log_lines, format!("meas{measure_number}: render"));
            let mut mixed = vec![0.0f32; measure_samples];
            for track in active_tracks {
                let gain = track_gains.get(track).copied().unwrap_or(1.0);
                let mml = track_mmls
                    .get(track)
                    .map(String::as_str)
                    .unwrap_or_default();
                let samples =
                    pad_playback_measure_samples(render_fallback(track, mml)?, measure_samples);
                mix_track_into_buffer(&mut mixed, &samples, gain);
            }
            return Ok(PlaybackMeasureAudio {
                samples: mixed,
                source: PlaybackMeasureSource::Render,
            });
        }
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
    let mut mixed = vec![0.0f32; measure_samples];
    for track in active_tracks {
        let gain = track_gains.get(track).copied().unwrap_or(1.0);
        let mml = track_mmls
            .get(track)
            .map(String::as_str)
            .unwrap_or_default();
        let samples = pad_playback_measure_samples(render_fallback(track, mml)?, measure_samples);
        mix_track_into_buffer(&mut mixed, &samples, gain);
    }
    Ok(PlaybackMeasureAudio {
        samples: mixed,
        source: PlaybackMeasureSource::Render,
    })
}
