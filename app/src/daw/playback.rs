//! DawApp の演奏メソッド

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render_for_cache, CoreConfig};

use super::playback_util::play_start_log_lines;
pub(super) use super::playback_util::{effective_measure_count, loop_measure_summary_label};
use super::{CacheState, CellCache, DawApp, DawPlayState, PlayPosition, FIRST_PLAYABLE_TRACK};

#[derive(Clone)]
pub(super) struct CachedMeasureSamples {
    pub(super) samples: Vec<f32>,
    pub(super) cached_tracks: Vec<usize>,
}

#[derive(Clone)]
enum PlaybackMeasureSource {
    Empty,
    Cache { tracks: Vec<usize> },
    Render,
}

impl PlaybackMeasureSource {
    fn build_log_line(&self, measure_number: usize) -> String {
        match self {
            Self::Empty => format!("play: start meas{measure_number} empty -> silence"),
            Self::Cache { tracks } => {
                if tracks.is_empty() {
                    format!("play: start meas{measure_number} cache empty-tracks")
                } else {
                    let cache_entries = tracks
                        .iter()
                        .map(|track| format!("track{track}/meas{measure_number}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("play: start meas{measure_number} cache {cache_entries}")
                }
            }
            Self::Render => format!("play: start meas{measure_number} render fallback"),
        }
    }
}

struct PlaybackMeasureAudio {
    samples: Vec<f32>,
    source: PlaybackMeasureSource,
}

/// キャッシュ済みのサンプルをミックスして返す。
///
/// 指定小節（`measure`、1始まり）のすべての playable track（`FIRST_PLAYABLE_TRACK..tracks`）の
/// キャッシュを調べ、合算したサンプルを返す。
/// いずれかの playable track が `Ready` でない（Pending / Error）場合は `None` を返し、
/// 呼び出し元はフレッシュレンダリングにフォールバックすること。
/// 全 playable track が `Empty` の場合は無音（ゼロ埋め）を返す。
/// 結果は `measure_samples` 長に正確に揃えて返す（超過分は切り捨て、不足分はゼロ埋め済み）。
pub(super) fn try_get_cached_samples(
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
                CacheState::Ready => {
                    // samples が None の場合（サイズ上限超過等）もフォールバック
                    let arc = cache[t][measure].samples.clone();
                    arc.as_ref()?;
                    result.push((t, arc));
                }
                _ => {
                    // Pending または Error → キャッシュ未完成、フォールバックが必要
                    return None;
                }
            }
        }
        Some(result)
    };

    let track_samples = track_samples?;

    // ロック外でミックス処理を行う
    // 最初からゼロ埋め済みのバッファを使うことで measure_samples を超える書き込みを防ぐ
    let mut mixed = vec![0.0f32; measure_samples];
    let mut any_ready = false;
    let mut cached_tracks = Vec::new();

    for (track, maybe_samples) in &track_samples {
        let Some(samples) = maybe_samples else {
            continue;
        };
        any_ready = true;
        cached_tracks.push(*track);
        let n = samples.len().min(measure_samples);
        for i in 0..n {
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

pub(super) fn current_play_measure_index(
    current_measure_index: usize,
    effective_count: usize,
) -> usize {
    if current_measure_index < effective_count {
        current_measure_index
    } else {
        0
    }
}

pub(super) fn following_measure_index(
    current_measure_index: usize,
    effective_count: usize,
) -> usize {
    (current_measure_index + 1) % effective_count
}

fn build_playback_measure_samples<F, E>(
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
    let mut samples = render_fallback()?;
    if samples.len() < measure_samples {
        samples.resize(measure_samples, 0.0);
    } else {
        samples.truncate(measure_samples);
    }
    Ok(PlaybackMeasureAudio {
        samples,
        source: PlaybackMeasureSource::Render,
    })
}

#[derive(Clone)]
struct QueuedMeasure {
    measure_index: usize,
    measure_start: std::time::Instant,
    measure_duration: std::time::Duration,
}

fn measure_duration(sample_count: usize, sample_rate: u32) -> std::time::Duration {
    // sample_count はステレオのインターリーブ済みサンプル総数（L/R の合計要素数）。
    // そのため実時間は frames (= sample_count / 2) / sample_rate となり、
    // sample_count / (sample_rate * 2) と等価になる。
    std::time::Duration::from_secs_f64(sample_count as f64 / (sample_rate as f64 * 2.0))
}

impl DawApp {
    // ─── 演奏 ─────────────────────────────────────────────────

    pub(super) fn start_play(&self) {
        let measure_mmls = self.build_measure_mmls();
        if measure_mmls.iter().all(|m| m.trim().is_empty()) {
            return;
        }

        // play_measure_mmls と play_measure_samples を最新の値で更新してからスレッドに共有する
        *self.play_measure_mmls.lock().unwrap() = measure_mmls;
        *self.play_measure_samples.lock().unwrap() = self.measure_duration_samples();

        let play_state = Arc::clone(&self.play_state);
        let play_position = Arc::clone(&self.play_position);
        let play_measure_mmls = Arc::clone(&self.play_measure_mmls);
        let play_measure_samples = Arc::clone(&self.play_measure_samples);
        let render_lock = Arc::clone(&self.render_lock);
        let cache = Arc::clone(&self.cache);
        let cfg = Arc::clone(&self.cfg);
        let log_lines = Arc::clone(&self.log_lines);
        let entry_ptr = self.entry_ptr;
        let tracks = self.tracks;

        *play_state.lock().unwrap() = DawPlayState::Playing;
        crate::logging::append_log_line(&log_lines, "play: start");
        for line in play_start_log_lines(&self.play_measure_mmls.lock().unwrap()) {
            crate::logging::append_log_line(&log_lines, line);
        }

        std::thread::spawn(move || {
            // SAFETY: entry は main() のスタックに生存している
            let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
            let daw_cfg = (*cfg).clone();
            let sample_rate = daw_cfg.sample_rate as u32;

            // OutputStream と Sink をスレッドに 1 つだけ作成し、小節をまたいで再利用する。
            // これにより小節ごとのオーディオ初期化オーバーヘッドとグリッチを防ぐ。
            let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() else {
                // Audio init failed: only reset to Idle if we are still the active Playing session.
                crate::logging::append_log_line(&log_lines, "play: audio init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Playing {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };
            let Ok(sink) = rodio::Sink::try_new(&stream_handle) else {
                crate::logging::append_log_line(&log_lines, "play: sink init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Playing {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };

            let mut measure_index = 0usize;
            let mut current_measure = None::<QueuedMeasure>;

            'outer: loop {
                if *play_state.lock().unwrap() != DawPlayState::Playing {
                    break;
                }

                // 各小節の直前に最新の MML / 小節長を読み取って現在小節を決定する。
                // 初回はここで小節を解決し、2 小節目以降は後続小節の lookahead 準備に使う。
                let mmls = play_measure_mmls.lock().unwrap().clone();
                let measure_samples = *play_measure_samples.lock().unwrap();
                let effective_count = match effective_measure_count(&mmls) {
                    Some(n) => n,
                    None => break 'outer,
                };

                if current_measure.is_none() {
                    let current_measure_index =
                        current_play_measure_index(measure_index, effective_count);
                    let mml = &mmls[current_measure_index];
                    let playback_audio = match build_playback_measure_samples(
                        &cache,
                        current_measure_index,
                        mml,
                        measure_samples,
                        tracks,
                        &log_lines,
                        || {
                            let _guard = render_lock.lock().unwrap();
                            let core_cfg = CoreConfig::from(&daw_cfg);
                            mml_render_for_cache(mml, &core_cfg, entry_ref)
                        },
                    ) {
                        Ok(playback_audio) => playback_audio,
                        Err(_) => {
                            crate::logging::append_log_line(
                                &log_lines,
                                format!("meas{}: render error", current_measure_index + 1),
                            );
                            break 'outer;
                        }
                    };
                    let measure_start = std::time::Instant::now();
                    let measure_duration = measure_duration(measure_samples, sample_rate);
                    let PlaybackMeasureAudio { samples, source } = playback_audio;
                    sink.append(rodio::buffer::SamplesBuffer::new(2, sample_rate, samples));
                    *play_position.lock().unwrap() = Some(PlayPosition {
                        measure_index: current_measure_index,
                        measure_start,
                    });
                    crate::logging::append_log_line(
                        &log_lines,
                        source.build_log_line(current_measure_index + 1),
                    );
                    current_measure = Some(QueuedMeasure {
                        measure_index: current_measure_index,
                        measure_start,
                        measure_duration,
                    });
                }

                let current = current_measure.expect(
                    "BUG: current_measure must be initialized before lookahead; this indicates a logic error in the playback loop initialization",
                );

                // 現在小節の再生中に次小節を 1 つ先読みし、Sink が空になる前に追加する。
                // これにより cache miss 時のフォールバックレンダリングでも無音ギャップを最小化する。
                let lookahead_measure_index =
                    following_measure_index(current.measure_index, effective_count);
                let next_mml = &mmls[lookahead_measure_index];
                let next_playback_audio = match build_playback_measure_samples(
                    &cache,
                    lookahead_measure_index,
                    next_mml,
                    measure_samples,
                    tracks,
                    &log_lines,
                    || {
                        let _guard = render_lock.lock().unwrap();
                        let core_cfg = CoreConfig::from(&daw_cfg);
                        mml_render_for_cache(next_mml, &core_cfg, entry_ref)
                    },
                ) {
                    Ok(playback_audio) => playback_audio,
                    Err(_) => {
                        crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{}: render error", lookahead_measure_index + 1),
                        );
                        break 'outer;
                    }
                };

                let next_measure_start = current.measure_start + current.measure_duration;
                let next_measure_duration = measure_duration(measure_samples, sample_rate);
                let PlaybackMeasureAudio {
                    samples: next_samples,
                    source: next_source,
                } = next_playback_audio;
                sink.append(rodio::buffer::SamplesBuffer::new(
                    2,
                    sample_rate,
                    next_samples,
                ));

                // 小節境界は期待再生開始時刻を基準にポーリングする。
                // rodio::Sink には「現在キュー先頭の小節だけの終了」を待つ API がないため、
                // lookahead を維持しつつ停止要求にも追従できるよう時間ベースの待機を使う。
                loop {
                    if std::time::Instant::now() >= next_measure_start {
                        break;
                    }
                    if *play_state.lock().unwrap() != DawPlayState::Playing {
                        break 'outer;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                if *play_state.lock().unwrap() != DawPlayState::Playing {
                    break 'outer;
                }

                *play_position.lock().unwrap() = Some(PlayPosition {
                    measure_index: lookahead_measure_index,
                    measure_start: next_measure_start,
                });
                crate::logging::append_log_line(
                    &log_lines,
                    next_source.build_log_line(lookahead_measure_index + 1),
                );
                current_measure = Some(QueuedMeasure {
                    measure_index: lookahead_measure_index,
                    measure_start: next_measure_start,
                    measure_duration: next_measure_duration,
                });
                measure_index = lookahead_measure_index;
            }

            // Only reset to Idle if we are still the active Playing session.
            // An unconditional write would clobber a newer session started after stop.
            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Playing {
                *state = DawPlayState::Idle;
                drop(state);
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "play: finished");
            }
        });
    }

    pub(super) fn stop_play(&self) {
        let _transition_guard = self.play_transition_lock.lock().unwrap();
        let prev_state = {
            let mut play_state = self.play_state.lock().unwrap();
            let prev_state = *play_state;
            *play_state = DawPlayState::Idle;
            prev_state
        };
        match prev_state {
            DawPlayState::Idle => {}
            DawPlayState::Preview => self.append_log_line("preview: stop"),
            DawPlayState::Playing => self.append_log_line("play: stop"),
        }
        *self.play_position.lock().unwrap() = None;
    }
}

#[cfg(test)]
#[path = "../tests/daw/playback.rs"]
mod tests;
