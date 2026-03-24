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
) -> Option<Vec<f32>> {
    // ロック下では Arc ハンドルの収集のみ行い、ミックス処理はロック外で実施する。
    // これによりキャッシュワーカーや UI スレッドとのロック競合を最小化する。
    let track_samples: Option<Vec<Option<Arc<Vec<f32>>>>> = {
        let cache = cache.lock().unwrap();
        let mut result = Vec::with_capacity(tracks - FIRST_PLAYABLE_TRACK);
        for t in FIRST_PLAYABLE_TRACK..tracks {
            match cache[t][measure].state {
                CacheState::Empty => {
                    result.push(None); // 空トラック
                }
                CacheState::Ready => {
                    // samples が None の場合（サイズ上限超過等）もフォールバック
                    let arc = cache[t][measure].samples.clone();
                    arc.as_ref()?;
                    result.push(arc);
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

    for s in track_samples.iter().flatten() {
        any_ready = true;
        let n = s.len().min(measure_samples);
        for i in 0..n {
            mixed[i] += s[i];
        }
    }

    if !any_ready {
        // すべての playable track が Empty → 空トラックのみの小節として無音を返す
        return Some(mixed);
    }

    Some(mixed)
}

fn current_play_measure_index(current_measure_index: usize, effective_count: usize) -> usize {
    if current_measure_index < effective_count {
        current_measure_index
    } else {
        0
    }
}

fn next_play_measure_index(current_measure_index: usize, effective_count: usize) -> usize {
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
) -> Result<Vec<f32>, E>
where
    F: FnOnce() -> Result<Vec<f32>, E>,
{
    let measure_number = measure_index + 1;

    if mml.trim().is_empty() {
        crate::logging::append_log_line(
            log_lines,
            format!("meas{measure_number}: empty -> silence"),
        );
        return Ok(vec![0.0f32; measure_samples]);
    }

    if let Some(cached) = try_get_cached_samples(cache, measure_number, measure_samples, tracks) {
        crate::logging::append_log_line(log_lines, format!("meas{measure_number}: cache hit"));
        return Ok(cached);
    }

    crate::logging::append_log_line(log_lines, format!("meas{measure_number}: render"));
    let mut samples = render_fallback()?;
    if samples.len() < measure_samples {
        samples.resize(measure_samples, 0.0);
    } else {
        samples.truncate(measure_samples);
    }
    Ok(samples)
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
            'outer: loop {
                if *play_state.lock().unwrap() != DawPlayState::Playing {
                    break;
                }
                // 各小節の直前で play_measure_mmls と play_measure_samples を読み取ることで、
                // セル編集・音色変更を次小節から反映する（issue #132）。
                let mmls = play_measure_mmls.lock().unwrap().clone();
                let measure_samples = *play_measure_samples.lock().unwrap();

                // 末尾の空小節をスキップ: 有効な最後の小節までをループ対象とする。
                // これにより meas3-8 が空のときは meas1-2 だけをループする（issue #68）。
                let effective_count = match effective_measure_count(&mmls) {
                    Some(n) => n,
                    None => break 'outer,
                };

                let current_measure_index =
                    current_play_measure_index(measure_index, effective_count);
                let mml = &mmls[current_measure_index];

                let samples = match build_playback_measure_samples(
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
                    Ok(samples) => samples,
                    Err(_) => {
                        crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{}: render error", current_measure_index + 1),
                        );
                        break 'outer;
                    }
                };

                // 1 小節ずつ Sink に投入し、その都度最新の MML / テンポ変更を次小節から反映する。
                // append 直前の時刻を再生開始時刻として UI とログの粒度を 1 小節に合わせる。
                let measure_start = std::time::Instant::now();
                sink.append(rodio::buffer::SamplesBuffer::new(2, sample_rate, samples));
                *play_position.lock().unwrap() = Some(PlayPosition {
                    measure_index: current_measure_index,
                    measure_start,
                });

                // この小節の再生完了を 10 ms 粒度でポーリング待機する。
                // sink.sleep_until_end() は停止要求を検出できないためポーリングを使用する。
                loop {
                    if sink.empty() {
                        break;
                    }
                    if *play_state.lock().unwrap() != DawPlayState::Playing {
                        break 'outer;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                measure_index = next_play_measure_index(current_measure_index, effective_count);
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
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use tui_textarea::TextArea;

    use crate::config::Config;

    use super::{
        super::{CacheState, CellCache, DawApp, DawMode, DawPlayState},
        build_playback_measure_samples, current_play_measure_index, next_play_measure_index,
    };

    /// stop_play のログ出力を検証するための最小構成の DawApp を作る。
    fn build_test_app() -> DawApp {
        let tracks = 3;
        let measures = 2;
        let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
        DawApp {
            data: vec![vec![String::new(); measures + 1]; tracks],
            cursor_track: 0,
            cursor_measure: 0,
            mode: DawMode::Normal,
            textarea: TextArea::default(),
            cfg: Arc::new(Config {
                plugin_path: String::new(),
                input_midi: String::new(),
                output_midi: String::new(),
                output_wav: String::new(),
                sample_rate: 44_100.0,
                buffer_size: 512,
                patch_path: None,
                patches_dir: None,
                daw_tracks: tracks,
                daw_measures: measures,
            }),
            entry_ptr: 0,
            tracks,
            measures,
            cache: Arc::new(Mutex::new(vec![
                vec![CellCache::empty(); measures + 1];
                tracks
            ])),
            cache_tx,
            render_lock: Arc::new(Mutex::new(())),
            play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
            play_transition_lock: Arc::new(Mutex::new(())),
            play_position: Arc::new(Mutex::new(None)),
            play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
            play_measure_samples: Arc::new(Mutex::new(0)),
            log_lines: Arc::new(Mutex::new(VecDeque::new())),
            track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        }
    }

    #[test]
    fn stop_play_logs_preview_stop_for_preview_state() {
        let app = build_test_app();
        *app.play_state.lock().unwrap() = DawPlayState::Preview;

        app.stop_play();

        assert!(matches!(
            *app.play_state.lock().unwrap(),
            DawPlayState::Idle
        ));
        assert_eq!(
            app.log_lines.lock().unwrap().back().map(String::as_str),
            Some("preview: stop")
        );
    }

    #[test]
    fn stop_play_logs_play_stop_for_playing_state() {
        let app = build_test_app();
        *app.play_state.lock().unwrap() = DawPlayState::Playing;

        app.stop_play();

        assert!(matches!(
            *app.play_state.lock().unwrap(),
            DawPlayState::Idle
        ));
        assert_eq!(
            app.log_lines.lock().unwrap().back().map(String::as_str),
            Some("play: stop")
        );
    }

    #[test]
    fn current_play_measure_index_wraps_to_loop_start_when_measure_count_shrinks() {
        assert_eq!(current_play_measure_index(7, 4), 0);
        assert_eq!(current_play_measure_index(2, 4), 2);
    }

    #[test]
    fn next_play_measure_index_wraps_after_effective_end() {
        assert_eq!(next_play_measure_index(0, 4), 1);
        assert_eq!(next_play_measure_index(3, 4), 0);
    }

    #[test]
    fn build_playback_measure_samples_returns_silence_for_empty_measure() {
        let log_lines = Arc::new(Mutex::new(VecDeque::new()));
        let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
        let samples = build_playback_measure_samples(
            &cache,
            1,
            "",
            4,
            3,
            &log_lines,
            || -> Result<Vec<f32>, ()> { panic!("empty measure should not render") },
        )
        .unwrap();

        assert_eq!(samples, vec![0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            log_lines.lock().unwrap().back().map(String::as_str),
            Some("meas2: empty -> silence")
        );
    }

    #[test]
    fn build_playback_measure_samples_prefers_cache_hit() {
        let log_lines = Arc::new(Mutex::new(VecDeque::new()));
        let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
        cache.lock().unwrap()[1][1] = CellCache {
            state: CacheState::Ready,
            samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5])),
            generation: 0,
            rendered_mml_hash: None,
        };

        let samples = build_playback_measure_samples(
            &cache,
            0,
            "c",
            4,
            3,
            &log_lines,
            || -> Result<Vec<f32>, ()> { panic!("cache hit should not render") },
        )
        .unwrap();

        assert_eq!(samples, vec![0.25, -0.25, 0.5, -0.5]);
        assert_eq!(
            log_lines.lock().unwrap().back().map(String::as_str),
            Some("meas1: cache hit")
        );
    }

    #[test]
    fn build_playback_measure_samples_renders_and_normalizes_length() {
        let log_lines = Arc::new(Mutex::new(VecDeque::new()));
        let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
        cache.lock().unwrap()[1][1].state = CacheState::Pending;

        let samples = build_playback_measure_samples(
            &cache,
            0,
            "c",
            4,
            3,
            &log_lines,
            || -> Result<Vec<f32>, ()> { Ok(vec![1.0, 2.0]) },
        )
        .unwrap();

        assert_eq!(samples, vec![1.0, 2.0, 0.0, 0.0]);
        assert_eq!(
            log_lines.lock().unwrap().back().map(String::as_str),
            Some("meas1: render")
        );
    }
}
