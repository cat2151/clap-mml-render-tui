//! 小節ごとの state load を実測する（判断 3 の決着）。
//!
//! 「1 instance = 1 WAV で、小節が変わるたびに 1.6MB の state load を掛け直す」形が
//! 小節長（約 2.4 秒）に間に合うのかを、**演奏 track 数を変えながら**数字で出すための
//! テスト。通常は skip され、実サーバーがあるときだけ走る。
//!
//! **先読みを入れてから、測る先が `prepare_ms` から `next_ms` へ移った。** state load は
//! 小節境界ではなく「1 つ前の小節を鳴らしている最中」に出るようになったので、
//! 小節長に対する占有率を決めるのは `next_ms` のほう。`prepare_ms` は
//! 「小節境界で演奏スレッドが止まっていた時間」で、先読みが効いていれば
//! 1 小節目以外は 0 になる。
//!
//! ```text
//! CMRT_REALTIME_PLAY_SERVER_PORT=8712 CMRT_LIVE_INSTANCE_COUNT=8 \
//!   ../clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe > server.log 2>&1 &
//! CMRT_LIVE_CACHE_TEST_PORT=8712 CMRT_LIVE_CACHE_TEST_WAV=<絶対パス> \
//! CMRT_LIVE_CACHE_TEST_TRACKS=8 \
//!   cargo test -p cmrt-daw --lib state_load -- --test-threads=1 --nocapture
//! ```
//!
//! ## なぜ WAV をコピーするのか
//!
//! Stage 4 の計測は「毎小節**同じ**パスを送り直す」条件だったので、サーバーがパスで
//! キャッシュしている可能性を排除できていなかった。ここでは `(行, 小節)` ごとに
//! **別ファイル**を temp へ用意して、実際の演奏と同じ「小節ごとに別パス」を作る。
//! 中身は同じなので鳴り方は変わらず、パスだけが毎回新しくなる。

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use crate::{
    playback::{live_gain::live_track_gains, real_server::real_server_from_env},
    tracks::FIRST_PLAYABLE_TRACK,
};

/// 演奏 track 数。1 / 4 / 8 と変えて走らせるために環境変数で受ける。
const TRACKS_ENV: &str = "CMRT_LIVE_CACHE_TEST_TRACKS";
/// 実測に使う小節数。`(行, 小節)` ぶんの WAV を temp へコピーするので、増やすと重い。
const MEASURES: usize = 4;
/// 実際の DAW と同じ小節長（約 2.4 秒）。占有率をそのまま読めるようにするため。
const MEASURE_SECONDS: f64 = 2.4;
const SAMPLE_RATE: u32 = 48_000;
/// 定常状態で許す占有率。ここを超えたら「小節の中で state load が終わらない」と見なす。
/// 実測（2026-09-02）は release ビルドのサーバーで 8 track / 10.6%、debug で 28.3%。
const STEADY_BUDGET_RATIO: f64 = 0.5;
/// 1 オーディオブロック（buffer 512 / 48kHz ≒ 10.7ms）。track 間のズレはこの中に収める。
/// 同じブロックで適用されたイベントは同時に鳴るので、ここを切っていればズレは聞こえない。
const ONE_AUDIO_BLOCK_MS: f64 = 10.0;

/// `(行, 小節)` ごとに別パスのキャッシュ WAV を作る temp ディレクトリ。
struct DistinctCacheWavs {
    dir: PathBuf,
}

impl DistinctCacheWavs {
    fn new(source: &Path, tracks: usize) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "cmrt-live-cache-state-load-{}-{tracks}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp ディレクトリを作れること");
        let wavs = Self { dir };
        for row in FIRST_PLAYABLE_TRACK..FIRST_PLAYABLE_TRACK + tracks {
            for measure_index in 0..MEASURES {
                let path = wavs.path(measure_index, row);
                std::fs::copy(source, &path).expect("キャッシュ WAV をコピーできること");
            }
        }
        wavs
    }

    fn path(&self, measure_index: usize, row: usize) -> PathBuf {
        self.dir
            .join(format!("track{row}_meas{}.wav", measure_index + 1))
    }
}

impl Drop for DistinctCacheWavs {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

/// 小節ログから `<key>=` の数値を拾う（`prepare_ms` / `note_on_ms` / `next_ms`）。
fn measure_millis(log_lines: &[String], key: &str) -> Vec<f64> {
    let needle = format!(" {key}=");
    log_lines
        .iter()
        .filter_map(|line| line.split(&needle).nth(1))
        .map(|rest| {
            rest.split_whitespace()
                .next()
                .expect("値が続くこと")
                .parse::<f64>()
                .unwrap_or_else(|_| panic!("{key} は数値"))
        })
        .collect()
}

/// 2 つの内訳を足して「小節の頭で演奏スレッドが止まっていた時間」にする。
fn totals(prepare: &[f64], note_on: &[f64]) -> Vec<f64> {
    prepare
        .iter()
        .zip(note_on)
        .map(|(prepare, note_on)| prepare + note_on)
        .collect()
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}

fn max(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::MIN, f64::max)
}

/// 演奏中の underrun を 50ms 刻みで見張り、増えた瞬間だけ `(演奏開始からの ms, frames)` で残す。
///
/// 合計だけでは「立ち上がりで 1 回途切れた」のか「小節が変わるたびに途切れている」のかが
/// 区別できない。判断 3 の結論はそこで分かれるので、時刻ごと残す。
fn watch_dropouts(
    play_server: &RealtimePlayServerSupervisor,
    started: std::time::Instant,
    window: Duration,
    before: u64,
) -> Vec<(u128, u64)> {
    let mut dropouts = Vec::new();
    let mut previous = before;
    while started.elapsed() < window {
        std::thread::sleep(Duration::from_millis(50));
        let now = play_server.underrun_frames();
        if now > previous {
            dropouts.push((started.elapsed().as_millis(), now - previous));
            previous = now;
        }
    }
    dropouts
}

/// 小節ごとの state load が小節の中に収まっているか。
///
/// 判定材料は 3 つとも**サーバーログを読まずに**取れる:
///
/// - `next_ms`: 次の小節を先読みするのに掛かった時間（＝ state load の実測値）
/// - `prepare_ms`: **小節境界で**演奏スレッドが止まっていた時間。先読みが効いていれば 0
/// - `note_on_ms`: 全 track ぶんの note on を送り終えるまでの実時間
///   （= 最初と最後の note on の間隔の上限）
/// - `timing_metrics().late_events_total`: サーバーが取りこぼしたイベント数
/// - `underrun_frames()`: 音が途切れたフレーム数（[`watch_dropouts`] で時刻ごと）
///
/// **1 小節目だけは別枠**で見る。起動直後の instance は既定プラグインが載っているので、
/// 最初の prepare には cache-player への instance_swap が含まれる。定常状態の数字と
/// 混ぜると、実際には 2 小節目以降ずっと軽いことが見えなくなる。
///
/// # 実測（2026-09-02。小節 2400ms・小節ごとに別パス・**note on を 1 バッチにまとめた後**）
///
/// | サーバー | track | `prepare_ms`（定常） | 占有率 | `note_on_ms` | 1 小節目以降の途切れ |
/// |---|---|---|---|---|---|
/// | release | 1 | 15.8〜22.7ms | 0.9% | 0.6〜0.7ms | 0 frames |
/// | release | 4 | 92.8〜112.3ms | 4.7% | 0.6〜0.7ms | 0 frames |
/// | release | 8 | 188.6〜205.7ms | 8.6% | 0.6〜0.9ms | 0 frames |
/// | debug | 1 | 71.5〜102.0ms | 4.2% | 0.7ms | 毎小節 約 1000〜2500 frames |
/// | debug | 4 | 264.1〜269.4ms | 11.2% | 0.6〜1.1ms | 毎小節 約 5100〜5700 frames |
/// | debug | 8 | 518.7〜525.7ms | 21.9% | 0.7ms | 毎小節 約 11800〜12800 frames |
///
/// **`note_on_ms` は track 数に比例しない。** 8 track でも 1ms 未満で、
/// 1 オーディオブロック（約 10.7ms）の中に収まる＝全 track が同じブロックで鳴る。
/// サーバーログでも 8 track の小節は `event=apply-midi ... count=8` の **1 行**になる
/// （1 件ずつ送っていたころは `count=1` が 8 行だった）。
///
/// 参考: **1 track ずつ「prepare → note on」と交互に送っていたころ**（2 パス化する前）の
/// 同条件の実測。当時は送信を分けていなかったので 1 列しか無く、その値がそのまま
/// 「最初と最後の note on の間隔」だった。
///
/// | サーバー | track | 1 小節ぶんの送信 | 占有率 |
/// |---|---|---|---|
/// | release | 1 | 17〜32ms | 0.9% |
/// | release | 4 | 96〜147ms | 6.1% |
/// | release | 8 | 200〜254ms | 10.6% |
/// | debug | 1 | 67〜94ms | 3.9% |
/// | debug | 4 | 308〜355ms | 14.2% |
/// | debug | 8 | 609〜696ms | 28.3% |
///
/// **debug ビルドのサーバーだと小節境界ごとに音が途切れる。** 差は WAV のデコード
/// （hound の 1 サンプルずつの変換）で、prepare 1 件あたり release 10〜13ms に対し
/// debug 60〜85ms。出力リングの先読みは `lead_frames=512..2528`（約 52ms）しかないので、
/// 1 件で使い切ってしまう。だから assert は「途切れの量が state load の実測で
/// 説明できる範囲か」を見る形にしてある（どちらのビルドでも通り、
/// **別の原因で途切れ始めたら**赤くなる）。
#[test]
fn a_real_server_loads_every_measure_state_well_inside_one_measure() {
    let Some((play_server, source_wav)) = real_server_from_env() else {
        // 実サーバーが無い環境では何もしない（CI でも常に green）。
        return;
    };
    let tracks: usize = std::env::var(TRACKS_ENV)
        .ok()
        .map(|value| value.parse().expect("track 数は数値"))
        .unwrap_or(1);
    assert!(tracks >= 1, "演奏 track が無いと何も測れない");

    let wavs = DistinctCacheWavs::new(&source_wav, tracks);
    let measure_samples = (MEASURE_SECONDS * f64::from(SAMPLE_RATE) * 2.0) as usize;
    let play_state = Arc::new(Mutex::new(crate::DawPlayState::Playing));
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let grid_rows = FIRST_PLAYABLE_TRACK + tracks;
    let play_server = Arc::new(play_server);

    // **無音時を基準にはできない。** underrun frames はサーバーの live が active な
    // あいだしか数えないので（`player/audio_output.rs` の `fill_output`）、
    // 鳴らしていない窓では構造的に 0 になる。代わりに演奏中を 50ms 刻みで刻んで、
    // 「いつ音が途切れたか」を小節境界と突き合わせる。
    let window = Duration::from_secs_f64((MEASURES - 1) as f64 * MEASURE_SECONDS + 1.0);

    // **先に 1 コマンド送って SHM へ繋いでおく。** 未接続の `underrun_frames()` は
    // 0 を返すので（`live_ipc.rs` の `fast_underrun_reader` が None）、繋ぐ前に基準を
    // 取ると「サーバー起動以来の累計」が演奏開始の瞬間に一気に見えて、途切れの時刻を
    // 読み違える。auto gain off は演奏ループも最初にやることなので副作用にならない。
    play_server
        .set_live_auto_gain_enabled(false)
        .expect("SHM へ繋げること");

    let before_timing = play_server.timing_metrics();
    let before_underrun = play_server.underrun_frames();

    let play_loop = crate::playback::live_cache::LiveCachePlayLoop {
        play_server: Arc::clone(&play_server),
        play_state: Arc::clone(&play_state),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(crate::AbRepeatState::Off)),
        measure_mmls: Arc::new(Mutex::new(vec!["cde".to_string(); MEASURES])),
        measure_samples: Arc::new(Mutex::new(measure_samples)),
        log_lines: Arc::clone(&log_lines),
        sample_rate: SAMPLE_RATE,
        tempo_bpm: 120.0,
        beat_numerator: 4,
        tracks: grid_rows,
        ready_cache_wav: Arc::new(move |measure_index, row| Some(wavs.path(measure_index, row))),
        initial_track_gains: live_track_gains(grid_rows, |_| -3, |_| true),
        sent_track_gains: Arc::new(Mutex::new(Vec::new())),
        startup: crate::playback::DawPlaybackStartupState::default(),
    };

    let started = std::time::Instant::now();
    let handle = std::thread::spawn(move || play_loop.run(0));
    // 最後の小節の送信が終わったところで止める（巡回して同じパスを 2 度使わない）。
    // 待っているあいだに、音が途切れた時刻を刻んでおく。
    let dropouts = watch_dropouts(&play_server, started, window, before_underrun);
    *play_state.lock().unwrap() = crate::DawPlayState::Idle;
    handle.join().expect("演奏ループが停止すること");

    let logged: Vec<String> = log_lines.lock().unwrap().iter().cloned().collect();
    let failures: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(failures.is_empty(), "送信に失敗した行がある: {failures:?}");

    // `prepare_ms` は小節境界で止まっていた時間（先読みが効いていれば 1 小節目以外 0）、
    // `next_ms` は小節の途中で出した先読みの時間。**state load の実測値は後者。**
    let prepare_millis = measure_millis(&logged, "prepare_ms");
    let note_on_millis = measure_millis(&logged, "note_on_ms");
    let preload_millis = measure_millis(&logged, "next_ms");
    let boundary_millis = totals(&prepare_millis, &note_on_millis);
    assert_eq!(
        boundary_millis.len(),
        MEASURES,
        "測れた小節数が想定と違う: {logged:?}"
    );
    assert_eq!(note_on_millis.len(), MEASURES, "note on の時間が欠けている");
    assert_eq!(preload_millis.len(), MEASURES, "先読みの時間が欠けている");
    // 1 小節目だけは先読みの元が無いので境界で載せる。定常状態は 2 小節目以降。
    let (first, steady) = preload_millis.split_at(1);
    let after_timing = play_server.timing_metrics();
    let after_underrun = play_server.underrun_frames();
    let late = after_timing
        .late_events_total
        .saturating_sub(before_timing.late_events_total);
    let underrun = after_underrun.saturating_sub(before_underrun);
    let budget_ms = MEASURE_SECONDS * 1_000.0;

    // 演奏の**頭**の途切れは live の立ち上がり（出力リングが空から始まる）ぶんで、
    // 小節ごとの state load とは別物。ここで見たいのは「小節が変わるたびに途切れるか」。
    let after_first_measure: u64 = dropouts
        .iter()
        .filter(|(ms, _)| *ms as f64 > MEASURE_SECONDS * 1_000.0 * 0.5)
        .map(|(_, frames)| frames)
        .sum();

    eprintln!(
        "live-cache state load: tracks={tracks} measures={MEASURES} measure_ms={budget_ms:.0} \
         preload_first_ms={:.1} preload_steady_max_ms={:.1} preload_steady_median_ms={:.1} \
         preload_steady_occupancy={:.1}% preload_all={preload_millis:?} \
         boundary_all={boundary_millis:?} \
         note_on_max_ms={:.2} note_on_all={note_on_millis:?} \
         late={late} underrun_frames={underrun} cpu_p95={:.0}% \
         after_first_measure_dropout_frames={after_first_measure} dropouts={dropouts:?}\n\
         {}",
        first[0],
        max(steady),
        median(steady),
        max(steady) / budget_ms * 100.0,
        max(&note_on_millis),
        after_timing.process_load_p95,
        logged
            .iter()
            .filter(|line| line.contains(": live-cache "))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );

    assert!(
        max(steady) < budget_ms * STEADY_BUDGET_RATIO,
        "先読みが小節長の {:.0}% を超えた（次の小節に間に合わない）: \
         {steady:?} (小節 {budget_ms:.0}ms)",
        STEADY_BUDGET_RATIO * 100.0
    );
    assert!(
        first[0] < budget_ms,
        "1 小節目の state load が小節長を超えた（演奏が丸ごと 1 小節止まる）: {:.1}ms",
        first[0]
    );
    // **小節境界には state load が無い。** ここが 0 でなくなったら先読みが壊れている。
    // 対策前はこの列が毎小節 98.6〜129.8ms（8 track の debug サーバーなら 520ms）で、
    // それがそのまま小節の頭の無音になっていた。
    for line in logged
        .iter()
        .filter(|line| line.contains(": live-cache "))
        .skip(1)
    {
        assert!(
            line.contains(" preload=hit ") && line.contains(" prepare_ms=0.0 "),
            "小節境界で state load を出している: {line}"
        );
    }
    assert_eq!(late, 0, "サーバーがイベントを取りこぼしている");
    // 小節の頭のズレ。全 track の note on を 1 コマンドへまとめているので、
    // 「最初の track と最後の track の note on の間隔」の上限がこの値になる。
    // 1 track ずつ交互に送る形へ戻ると、`prepare` の応答待ちが挟まってここが跳ねる
    // （実測: 8 track で release 250ms / debug 650ms）。
    assert!(
        max(&note_on_millis) < ONE_AUDIO_BLOCK_MS,
        "小節の頭で track がずれている: note_on_ms={note_on_millis:?} \
         （1 オーディオブロック {ONE_AUDIO_BLOCK_MS}ms 未満であること）"
    );
    // 途切れの量そのものは「state load でどれだけ render が止まったか」で決まるので、
    // 実測した送信時間ぶんを上限にする。ここを超えたら state load 以外の原因
    // （演奏ループが audio thread を塞ぐ等）が混ざっている。
    let explained_frames =
        (preload_millis.iter().sum::<f64>() / 1_000.0 * f64::from(SAMPLE_RATE)).ceil() as u64;
    assert!(
        after_first_measure <= explained_frames,
        "state load で説明できない音の途切れがある: {after_first_measure} frames \
         (state load ぶんの上限 {explained_frames} frames) dropouts={dropouts:?}"
    );

    play_server
        .stop_live_all()
        .expect("停止できること（次のテストへ音を残さない）");
}
