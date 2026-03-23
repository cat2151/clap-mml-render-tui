//! DawApp の演奏メソッド

use std::sync::{Arc, Mutex};

use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render_for_cache, CoreConfig};

use super::{CacheState, CellCache, DawApp, DawPlayState, PlayPosition, FIRST_PLAYABLE_TRACK};

/// 末尾の空小節を除いた有効な小節数を計算する。
///
/// すべての小節が空の場合は `None` を返す。
/// これにより meas3-8 が空のときは meas1-2 だけをループする（issue #68）。
pub(super) fn effective_measure_count(mmls: &[String]) -> Option<usize> {
    mmls.iter()
        .rposition(|m| !m.trim().is_empty())
        .map(|idx| idx + 1)
}

fn measure_indices_matching(mmls: &[String], is_match: impl Fn(&str) -> bool) -> Vec<usize> {
    mmls.iter()
        .enumerate()
        .filter_map(|(idx, mml)| is_match(mml.trim()).then_some(idx + 1))
        .collect()
}

pub(super) fn non_empty_measure_indices(mmls: &[String]) -> Vec<usize> {
    measure_indices_matching(mmls, |mml| !mml.is_empty())
}

pub(super) fn empty_measure_indices(mmls: &[String]) -> Vec<usize> {
    measure_indices_matching(mmls, str::is_empty)
}

pub(super) fn format_measure_list(indices: &[usize]) -> Option<String> {
    if indices.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut start = indices[0];
    let mut prev = indices[0];

    for &index in &indices[1..] {
        if index == prev + 1 {
            prev = index;
            continue;
        }

        if start == prev {
            parts.push(format!("meas {start}"));
        } else {
            parts.push(format!("meas {start}～{prev}"));
        }
        start = index;
        prev = index;
    }

    if start == prev {
        parts.push(format!("meas {start}"));
    } else {
        parts.push(format!("meas {start}～{prev}"));
    }

    Some(parts.join(", "))
}

pub(super) fn loop_measure_summary_label(mmls: &[String]) -> Option<String> {
    let effective_count = effective_measure_count(mmls)?;
    let loop_measures: Vec<usize> = (1..=effective_count).collect();
    let loop_label = format_measure_list(&loop_measures)?;
    let empty_label = format_measure_list(&empty_measure_indices(mmls))
        .unwrap_or_else(|| "none".to_string());
    Some(format!(
        "loop meas : {loop_label}, empty meas : {empty_label}"
    ))
}

pub(super) fn play_start_log_lines(mmls: &[String]) -> Vec<String> {
    let Some(effective_count) = effective_measure_count(mmls) else {
        return Vec::new();
    };

    let active_measures = non_empty_measure_indices(mmls);
    let empty_measures = empty_measure_indices(mmls);
    let mut lines: Vec<String> = mmls
        .iter()
        .enumerate()
        .map(|(idx, mml)| {
            if mml.trim().is_empty() {
                format!("meas{} : empty", idx + 1)
            } else {
                format!("meas{} : 内容があります", idx + 1)
            }
        })
        .collect();

    lines.push(format!(
        "有効meas : {}",
        format_measure_list(&active_measures).unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "empty meas : {}",
        format_measure_list(&empty_measures).unwrap_or_else(|| "none".to_string())
    ));
    lines.push("loop start meas : meas1".to_string());
    lines.push(format!("loop end meas : meas{effective_count}"));
    lines
}

/// キャッシュ済みのサンプルをミックスして返す。
///
/// 指定小節（`measure`、1始まり）のすべての playable track（`FIRST_PLAYABLE_TRACK..tracks`）の
/// キャッシュを調べ、合算したサンプルを返す。
/// いずれかの playable track が `Ready` でない（Pending / Error）場合は `None` を返し、
/// 呼び出し元はフレッシュレンダリングにフォールバックすること。
/// 全 playable track が `Empty` の場合は無音（ゼロ埋め）を返す。
/// 結果は `measure_samples` 長に正確に揃えて返す（超過分は切り捨て、不足分はゼロ埋め済み）。
fn try_get_cached_samples(
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

            'outer: loop {
                if *play_state.lock().unwrap() != DawPlayState::Playing {
                    break;
                }
                // ループの先頭で毎回 play_measure_mmls と play_measure_samples を読み取ることで、
                // セル編集・音色変更を次ループから即座に反映する（hot reload）
                let mmls = play_measure_mmls.lock().unwrap().clone();
                let measure_samples = *play_measure_samples.lock().unwrap();

                // 末尾の空小節をスキップ: 有効な最後の小節までをループ対象とする。
                // これにより meas3-8 が空のときは meas1-2 だけをループする（issue #68）。
                let effective_count = match effective_measure_count(&mmls) {
                    Some(n) => n,
                    None => break 'outer,
                };

                // 全有効小節のサンプルを事前収集する。
                // シームレス再生のため、全サンプルをまとめて Sink にキューイングしてから
                // 時間ベースで位置を更新する（issue #68）。
                let mut all_samples: Vec<(usize, Vec<f32>)> = Vec::with_capacity(effective_count);
                for (measure_index, mml) in mmls.iter().enumerate().take(effective_count) {
                    if *play_state.lock().unwrap() != DawPlayState::Playing {
                        break 'outer;
                    }

                    let measure_number = measure_index + 1;
                    let samples = if mml.trim().is_empty() {
                        crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{measure_number}: empty -> silence"),
                        );
                        // 中間の空小節は無音で維持（前後の小節とのタイミングを保持）
                        vec![0.0f32; measure_samples]
                    } else if let Some(cached) =
                        try_get_cached_samples(&cache, measure_index + 1, measure_samples, tracks)
                    {
                        crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{measure_number}: cache hit"),
                        );
                        // キャッシュヒット: 事前レンダリング済みサンプルをそのまま使用
                        cached
                    } else {
                        crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{measure_number}: render"),
                        );
                        // キャッシュミス: レンダリングにフォールバック
                        // render_lock を取得してからレンダリングすることで、
                        // `mml_str_to_smf_bytes` が書き出す共有デバッグファイルへ
                        // キャッシュワーカーと同時書き込みしないようにする
                        let result = {
                            let _guard = render_lock.lock().unwrap();
                            // mml_render_for_cache を使用することで patch_history.txt への追記を行わない
                            let core_cfg = CoreConfig::from(&daw_cfg);
                            mml_render_for_cache(mml, &core_cfg, entry_ref)
                        };
                        match result {
                            Ok(mut s) => {
                                // 設定された拍子・テンポに基づく 1 小節の長さに正確に pad / truncate する
                                if s.len() < measure_samples {
                                    s.resize(measure_samples, 0.0);
                                } else {
                                    s.truncate(measure_samples);
                                }
                                s
                            }
                            Err(_) => {
                                crate::logging::append_log_line(
                                    &log_lines,
                                    format!("meas{measure_number}: render error"),
                                );
                                break 'outer;
                            }
                        }
                    };
                    all_samples.push((measure_index, samples));
                }

                if all_samples.is_empty() {
                    break 'outer;
                }

                // インデックスとバッファを分離する。
                // バッファは clone せず所有権ごと Sink に移動することでメモリコピーを回避する。
                let (measure_indices, sample_bufs): (Vec<usize>, Vec<Vec<f32>>) =
                    all_samples.into_iter().unzip();

                // measure_samples はステレオインターリーブ（L/R 各 1 サンプル = 2 要素）のため
                // 実時間 = measure_samples / (sample_rate * 2) となる。
                let measure_duration_secs = measure_samples as f64 / (sample_rate as f64 * 2.0);

                // 全サンプルを Sink にまとめてキューイングしてシームレス再生を実現する（issue #68）。
                // loop_start は最初の append 直前に記録することで、実際のオーディオ開始と
                // できる限り近いタイムスタンプを得て位置推定の精度を高める。
                let loop_start = std::time::Instant::now();
                for buf in sample_bufs {
                    sink.append(rodio::buffer::SamplesBuffer::new(2, sample_rate, buf));
                }

                // 時間ベースで各小節の再生開始位置を更新する。
                // 10 ms 粒度でポーリングすることで停止要求に素早く応答できる（issue #68）。
                for (i, measure_index) in measure_indices.iter().enumerate() {
                    let measure_start_target = loop_start
                        + std::time::Duration::from_secs_f64(i as f64 * measure_duration_secs);
                    // この小節の期待開始時刻まで待機（停止チェック付き）
                    loop {
                        if std::time::Instant::now() >= measure_start_target {
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
                        measure_index: *measure_index,
                        measure_start: measure_start_target,
                    });
                }

                // 最後の小節の再生完了を 10 ms 粒度でポーリング待機する。
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

    /// 指定された小節を一度だけ再生するプレビュー（ループなし）
    pub(super) fn start_preview(&self, measure_index: usize) {
        let mmls = self.build_measure_mmls();
        let mml = mmls.get(measure_index).cloned().unwrap_or_default();
        if mml.trim().is_empty() {
            return;
        }

        let measure_samples = self.measure_duration_samples();
        let play_state = Arc::clone(&self.play_state);
        let play_position = Arc::clone(&self.play_position);
        let render_lock = Arc::clone(&self.render_lock);
        let cache = Arc::clone(&self.cache);
        let cfg = Arc::clone(&self.cfg);
        let log_lines = Arc::clone(&self.log_lines);
        let entry_ptr = self.entry_ptr;
        let tracks = self.tracks;

        *play_state.lock().unwrap() = DawPlayState::Preview;
        crate::logging::append_log_line(&log_lines, format!("preview: meas{}", measure_index + 1));

        std::thread::spawn(move || {
            // SAFETY: entry は main() のスタックに生存している
            let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
            let daw_cfg = (*cfg).clone();
            let sample_rate = daw_cfg.sample_rate as u32;

            let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() else {
                // Audio init failed: only reset to Idle if we are still the active Preview session.
                crate::logging::append_log_line(&log_lines, "preview: audio init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Preview {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };
            let Ok(sink) = rodio::Sink::try_new(&stream_handle) else {
                crate::logging::append_log_line(&log_lines, "preview: sink init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Preview {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };

            // キャッシュヒット時は即時再生、ミス時はレンダリングにフォールバック
            let samples_opt = if let Some(cached) =
                try_get_cached_samples(&cache, measure_index + 1, measure_samples, tracks)
            {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: cache hit", measure_index + 1),
                );
                Some(cached)
            } else {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: render", measure_index + 1),
                );
                let result = {
                    let _guard = render_lock.lock().unwrap();
                    let core_cfg = CoreConfig::from(&daw_cfg);
                    mml_render_for_cache(&mml, &core_cfg, entry_ref)
                };
                result.ok().map(|mut s| {
                    if s.len() < measure_samples {
                        s.resize(measure_samples, 0.0);
                    } else {
                        s.truncate(measure_samples);
                    }
                    s
                })
            };

            if let Some(samples) = samples_opt {
                // サンプル取得成功後、まだ Preview セッションが有効なときだけ再生開始時刻を更新する。
                // stop や新しい演奏開始後に上書きしないようガードする。
                // レンダリング失敗時は play_position を更新せず、UI に再生中と表示させない。
                if *play_state.lock().unwrap() == DawPlayState::Preview {
                    *play_position.lock().unwrap() = Some(PlayPosition {
                        measure_index,
                        measure_start: std::time::Instant::now(),
                    });
                }
                // Preview 中に stop された場合は再生しない
                if *play_state.lock().unwrap() == DawPlayState::Preview {
                    let source = rodio::buffer::SamplesBuffer::new(2, sample_rate, samples);
                    sink.append(source);
                    sink.sleep_until_end();
                }
            } else {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: render error", measure_index + 1),
                );
            }

            // Only reset to Idle if we are still the active Preview session.
            // An unconditional write would clobber a newer session started after stop.
            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Preview {
                *state = DawPlayState::Idle;
                drop(state);
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "preview: finished");
            }
        });
    }

    pub(super) fn stop_play(&self) {
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
        super::{CellCache, DawApp, DawMode, DawPlayState, MEASURES},
        effective_measure_count, format_measure_list, play_start_log_lines,
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
            play_position: Arc::new(Mutex::new(None)),
            play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
            play_measure_samples: Arc::new(Mutex::new(0)),
            log_lines: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    // ─── effective_measure_count ──────────────────────────────────

    #[test]
    fn effective_measure_count_all_empty_returns_none() {
        let mmls = vec!["".to_string(); MEASURES];
        assert_eq!(effective_measure_count(&mmls), None);
    }

    #[test]
    fn effective_measure_count_skips_trailing_empty_measures() {
        // meas1=cccccccc, meas2=ffffffff, meas3-8 空 → 有効小節数=2（issue #68）
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "cccccccc".to_string();
        mmls[1] = "ffffffff".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(2));
    }

    #[test]
    fn effective_measure_count_includes_internal_empty_measures() {
        // meas1 非空、meas2 空（中間）、meas3 非空、meas4-8 空 → 有効小節数=3
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "cde".to_string();
        mmls[2] = "fga".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(3));
    }

    #[test]
    fn effective_measure_count_single_non_empty_measure() {
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "c".to_string();
        assert_eq!(effective_measure_count(&mmls), Some(1));
    }

    #[test]
    fn effective_measure_count_all_measures_non_empty() {
        let mmls: Vec<String> = (0..MEASURES).map(|i| format!("c{}", i)).collect();
        assert_eq!(effective_measure_count(&mmls), Some(MEASURES));
    }

    #[test]
    fn effective_measure_count_whitespace_only_treated_as_empty() {
        let mut mmls = vec!["".to_string(); MEASURES];
        mmls[0] = "cde".to_string();
        mmls[1] = "   ".to_string(); // whitespace-only → treated as empty (trailing)
        assert_eq!(effective_measure_count(&mmls), Some(1));
    }

    #[test]
    fn format_measure_list_merges_consecutive_ranges() {
        assert_eq!(
            format_measure_list(&[1, 2, 3, 5, 7, 8]),
            Some("meas 1～3, meas 5, meas 7～8".to_string())
        );
    }

    #[test]
    fn play_start_log_lines_describe_active_and_empty_measures() {
        let mut mmls = vec![String::new(); MEASURES];
        mmls[0] = "c".to_string();

        assert_eq!(
            play_start_log_lines(&mmls),
            vec![
                "meas1 : 内容があります".to_string(),
                "meas2 : empty".to_string(),
                "meas3 : empty".to_string(),
                "meas4 : empty".to_string(),
                "meas5 : empty".to_string(),
                "meas6 : empty".to_string(),
                "meas7 : empty".to_string(),
                "meas8 : empty".to_string(),
                "有効meas : meas 1".to_string(),
                "empty meas : meas 2～8".to_string(),
                "loop start meas : meas1".to_string(),
                "loop end meas : meas1".to_string(),
            ]
        );
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
}
