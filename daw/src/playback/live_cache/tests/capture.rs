//! **ユーザーの実キャッシュで演奏ループを丸ごと回し、混ざった出力を 1 本の WAV へ録る。**
//!
//! ここが他の実サーバーテストと違うのは、判定材料が「ログの数値」ではなく
//! **出てきた波形そのもの**だという点。
//!
//! 小節 1 本ぶんのキャッシュ WAV をいくら調べても、`at_frames` の間隔をいくら測っても、
//! 「素材は正しい・予約も正しい・なのにモタって聴こえる」という形の不具合は捕まらない。
//! 素材と予約の間にある**鳴らし方**（どのスロットの音が、いつ、どれだけ続いたか）は、
//! 混ざったあとの波形にしか出ないため。だからここでは録って、あとから測る。
//!
//! 通常は skip される。次の 2 つが揃ったときだけ走る:
//!
//! - `CMRT_LIVE_CACHE_TEST_PORT` … 起動済み play server のポート
//! - `CMRT_LIVE_CACHE_CAPTURE_DIR` … 実キャッシュのディレクトリ
//!   （`track<行>_meas<小節>.wav` が並んでいるところ）
//!
//! サーバー側は `CMRT_LIVE_CAPTURE_WAV` で録る先を受け取る。**録った WAV が書かれるのは
//! サーバーを止めたとき**なので、このテストは録るだけで、中身の判定はしない
//! （判定は `scripts/capture_daw_live_mix.py` が波形を測って行う）。
//!
//! 手で並べると手数が多いので、実行は次の 1 コマンドで足りる:
//!
//! ```text
//! python scripts/capture_daw_live_mix.py
//! ```

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::playback::real_server::real_server_from_env_port;

/// 実キャッシュのディレクトリ。マシン固有なのでコードへ書かない。
const CAPTURE_DIR_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_DIR";
/// 録る小節数（ループを何小節ぶん進めるか）。
const CAPTURE_MEASURES_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_MEASURES";
/// 演奏する track 数（行 2 から数えて）。
const CAPTURE_TRACKS_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_TRACKS";
/// 演奏の BPM。小節長はここから決まる。
const CAPTURE_BPM_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_BPM";
/// 1 track あたりの gain（dB）。実演奏は auto trim が決めた 0dB 以下の値を使う。
///
/// **0dB のまま全 track を足すと master limiter で潰れる。** 潰れた波形は包絡が
/// 平らになるので、録っても「どこが小節の頭か」が読めなくなる。
const CAPTURE_GAIN_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_GAIN_DB";
/// ループの長さ（小節数）。キャッシュに残っている古い小節まで鳴らさないための上限。
///
/// **実演奏のループ長と揃えること。** キャッシュのファイル数で決めると、テンポを
/// 変える前に焼いた古い小節（＝いまの BPM と長さが違う WAV）まで鳴ってしまい、
/// 実演奏には無いモタりを録ってしまう。
const CAPTURE_LOOP_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_LOOP";
/// この行だけを聴こえる状態にする（グリッドの行番号。hi-hat を 1 本だけ録るため）。
///
/// **鳴らす行を減らすのではなく、mixer の gain で他を落とす。** 7 本が混ざった波形からは
/// 個々のアタック位置が読めないので 1 本だけ録りたいが、`ready_cache_wav` を絞って
/// 「1 行しか鳴らさない」形にすると **state load が 7 本から 1 本へ減る**。ロードの重さは
/// この不具合の当事者（クロックが止まる時間そのもの）なので、それを変えてしまうと
/// 実演奏と違う条件を測ることになる。
///
/// だからロードも note on も 7 本ぶんそのまま出し、**聴こえ方だけ**を変える。
/// 落とす側は [`crate::playback::live_gain::SILENT_TRACK_GAIN_DB`]（-120dB）なので、
/// master limiter にも 32bit float の可聴域にも届かない。
const CAPTURE_ONLY_ROW_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_ONLY_ROW";
/// 演奏を始める小節（**1 始まり**。既定は 1）。
///
/// 実アプリの「カーソルの小節から演奏」（`start_play_from_cursor_measure`）と同じ入口で、
/// `LiveCachePlayLoop::run` の引数そのもの。ここを 1 以外にすると **1 小節目が
/// `preload=miss`** になり、`restart_at` でグリッドを張り直す経路を通る。
///
/// **ループ長が `SLOT_COUNT` の倍数 +1 のときは、ここが末尾の小節だと踏み潰しが起きる**
/// （末尾と先頭が同じスロットで、末尾の note on は `lead` ぶん先に予約されるのに、
/// 先頭のロードはその前に届く）。`docs/adr/0018-page-replacement-clears-the-cache.md`。
const CAPTURE_START_MEASURE_ENV: &str = "CMRT_LIVE_CACHE_CAPTURE_START_MEASURE";

const SAMPLE_RATE: u32 = 48_000;
const BEATS_PER_MEASURE: u16 = 4;

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// 実キャッシュに実際に置かれている小節番号（1 始まり）を数える。
///
/// ループの長さは「中身のある小節の数」で決まるので、ここが実演奏と食い違うと
/// 録れた波形も実演奏と違うものになる。**行 2 に何小節あるか**で決める。
fn measures_in_cache(dir: &std::path::Path) -> usize {
    (1..=64)
        .take_while(|n| dir.join(format!("track2_meas{n}.wav")).is_file())
        .count()
}

/// 実キャッシュで演奏ループを回す。判定はせず、鳴らすことだけが仕事。
#[test]
fn a_real_server_plays_the_real_cache_so_the_mix_can_be_captured() {
    let (Some(play_server), Ok(dir)) =
        (real_server_from_env_port(), std::env::var(CAPTURE_DIR_ENV))
    else {
        // 実サーバーとキャッシュが揃っていない環境では何もしない。
        return;
    };
    let dir = PathBuf::from(dir);
    assert!(dir.is_dir(), "キャッシュのディレクトリが無い: {dir:?}");

    let in_cache = measures_in_cache(&dir);
    assert!(
        in_cache > 0,
        "track2_meas1.wav が無い（実キャッシュのディレクトリを指していない）: {dir:?}"
    );
    let in_cache = env_usize(CAPTURE_LOOP_ENV, in_cache).min(in_cache);
    let tracks = env_usize(CAPTURE_TRACKS_ENV, 8);
    let play_measures = env_usize(CAPTURE_MEASURES_ENV, 8);
    let bpm: f64 = std::env::var(CAPTURE_BPM_ENV)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(113.0);
    let gain_db: i32 = std::env::var(CAPTURE_GAIN_ENV)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(-12);
    let only_row: Option<usize> = std::env::var(CAPTURE_ONLY_ROW_ENV)
        .ok()
        .and_then(|v| v.parse().ok());

    // 実演奏とまったく同じ式で小節長を出す。`measure_samples` はステレオの
    // インターリーブ済み要素数なので、フレーム数の 2 倍。
    let measure_frames =
        (f64::from(BEATS_PER_MEASURE) * 60.0 / bpm * f64::from(SAMPLE_RATE)).round() as usize;
    let measure_samples = measure_frames * 2;
    eprintln!(
        "live-capture: cache_dir={dir:?} measures_in_cache={in_cache} tracks={tracks} bpm={bpm} gain_db={gain_db} measure_frames={measure_frames} only_row={only_row:?}"
    );

    // 中身のある小節の数がループの長さになる。実キャッシュにある数へ合わせる。
    let mmls: Vec<String> = (0..in_cache).map(|i| format!("meas{}", i + 1)).collect();

    let play_state = Arc::new(Mutex::new(crate::DawPlayState::Playing));
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let lookup_dir = dir.clone();
    let play_loop = crate::playback::live_cache::LiveCachePlayLoop {
        play_server: Arc::new(play_server),
        play_state: Arc::clone(&play_state),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(crate::AbRepeatState::Off)),
        measure_mmls: Arc::new(Mutex::new(mmls)),
        measure_samples: Arc::new(Mutex::new(measure_samples)),
        log_lines: Arc::clone(&log_lines),
        sample_rate: SAMPLE_RATE,
        tempo_bpm: bpm,
        beat_numerator: BEATS_PER_MEASURE,
        tracks,
        ready_cache_wav: Arc::new(move |measure_index, row| {
            let path = lookup_dir.join(format!("track{row}_meas{}.wav", measure_index + 1));
            path.is_file().then_some(path)
        }),
        // 実演奏と同じく mixer の gain を送る。**limiter に当てないこと**が要点で、
        // 当ててしまうと包絡が平らになって小節の頭が読めなくなる。
        initial_track_gains: crate::playback::live_gain::live_track_gains(
            tracks,
            |_| gain_db,
            // 指定が無ければ全行そのまま。指定があるとその行以外が -120dB になる。
            |row| only_row.is_none_or(|only| row == only),
        ),
        sent_track_gains: Arc::new(Mutex::new(Vec::new())),
        startup: crate::playback::DawPlaybackStartupState::default(),
    };

    // 1 始まりで受けて 0 始まりへ直す。範囲外はループ長で畳む（実アプリと同じ扱い）。
    let start_measure_index = env_usize(CAPTURE_START_MEASURE_ENV, 1).saturating_sub(1) % in_cache;
    eprintln!("live-capture: start_measure_index={start_measure_index}");
    let handle = std::thread::spawn(move || play_loop.run(start_measure_index));
    // 小節長 × 小節数ぶん鳴らす。余韻が録りきれるように 1 小節ぶん足す。
    let play_ms = (measure_frames as f64 / f64::from(SAMPLE_RATE)
        * (play_measures + 1) as f64
        * 1000.0) as u64;
    std::thread::sleep(std::time::Duration::from_millis(play_ms));
    *play_state.lock().unwrap() = crate::DawPlayState::Idle;
    handle.join().expect("演奏ループが停止すること");

    let logged: Vec<String> = log_lines.lock().unwrap().iter().cloned().collect();
    for line in logged.iter().filter(|line| line.contains(": live-cache ")) {
        eprintln!("{line}");
    }
    let failures: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(failures.is_empty(), "送信に失敗した行がある: {failures:?}");
}
