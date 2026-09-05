//! 実サーバーへ本物のコマンドを送る、**通常は skip される**テスト。
//!
//! `CMRT_LIVE_CACHE_TEST_PORT`（起動済み play server のポート）と
//! `CMRT_LIVE_CACHE_TEST_WAV`（キャッシュ WAV の絶対パス）が両方あるときだけ走る。
//! 個人のパスやポートをコードへ書かずに、演奏ループが使う送信経路そのものを
//! 実サーバーで通せるようにするための形。
//!
//! ```text
//! CMRT_REALTIME_PLAY_SERVER_PORT=8712 CMRT_LIVE_INSTANCE_COUNT=2 \
//!   ../clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe > server.log 2>&1 &
//! CMRT_LIVE_CACHE_TEST_PORT=8712 CMRT_LIVE_CACHE_TEST_WAV=<絶対パス> \
//!   cargo test -p cmrt-daw --lib live_ -- --test-threads=1 --nocapture
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::playback::live_cache::measure_live_cues;
use crate::playback::live_cache::send::{prepare_measure_cues, send_measure_note_on};
use crate::playback::live_cache::timeline::MeasureTimeline;
use crate::playback::real_server::real_server_from_env;

/// 実サーバーへ繋ぐテストで使う小節長（フレーム）。0.25 秒 @48kHz。
const TEST_MEASURE_FRAMES: u64 = 12_000;

/// キャッシュがある行だけが鳴る（1 小節ぶんの送信だけを見る）。
///
/// **音が出たことの判定は `live_auto_gain_db()` で機械的に行う。** サーバーの auto gain は
/// 「実際に鳴った音の RMS」からしか動かないので、1 ブロックも音を出していない instance は
/// 0 のままになる。演奏中の DAW は auto gain を切るが、このテストでは判定材料として
/// 一時的に on にし、最後に DAW と同じ off へ戻す。
///
/// 送信そのものが届いたかは、サーバーログでも裏取りできる:
/// `cmrt-bank-patch ... kind=prepare ... result=ok` と `cmrt-live: event=apply-midi`、
/// 停止で `apply-stop-all`。
#[test]
fn a_real_server_makes_sound_only_for_the_row_that_has_a_cached_wav() {
    let Some((play_server, wav)) = real_server_from_env() else {
        // 実サーバーが無い環境では何もしない（CI でも常に green）。
        return;
    };

    // 行 2 だけキャッシュがあり、行 3 は無い小節。行 3 へは 1 バイトも送らないはず。
    let cues = measure_live_cues(4, |row| (row == 2).then(|| wav.clone()));
    assert_eq!(cues.cues.len(), 1);
    assert_eq!(cues.silent_rows, vec![3]);
    let sounding = usize::from(cues.cues[0].instance);
    let silent = sounding + 1;

    play_server
        .set_live_auto_gain_enabled(true)
        .expect("auto gain を判定材料として on にできること");
    let before = play_server.live_auto_gain_db();

    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    // note on は timeline へ予約する形なので、**先に timeline を 1 本張らないと
    // サーバーはイベントを捨てる**（`realtime timeline MIDI received without an
    // active timeline`）。ここが抜けると「送信は成功しているのに無音」になる。
    let mut timeline = MeasureTimeline::begin(&play_server, 48_000, 120.0, 4, &log_lines);
    let measure = prepare_measure_cues(&play_server, 0, 0, cues, &log_lines, &mut |_| {});
    // **ロードのあとにクロックを起こす**のが演奏ループと同じ順番（`live_cache.rs`）。
    // ここを省くとサーバーは眠ったままで、note on を送っても 1 サンプルも鳴らない。
    timeline.start_clock(&play_server, &log_lines);
    let at = timeline.restart_at(std::time::Instant::now(), TEST_MEASURE_FRAMES);
    send_measure_note_on(&play_server, &timeline, &measure, at, &log_lines);

    let logged: Vec<String> = log_lines.lock().unwrap().iter().cloned().collect();
    assert!(
        logged
            .iter()
            .any(|line| line.contains("timeline begin ") && line.ends_with("result=ok")),
        "timeline を張れていない（これが無いと note on は全部捨てられる）: {logged:?}"
    );
    let failures: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(
        failures.is_empty(),
        "送信に失敗した行がある: {failures:?}（ログへ出るのは失敗時だけ）"
    );

    // 数ブロック鳴らせば auto gain が動く。
    std::thread::sleep(std::time::Duration::from_millis(1_000));
    let after = play_server.live_auto_gain_db();
    eprintln!("live-cache: auto_gain_db before={before:?} after={after:?}");
    assert_ne!(
        after[sounding], 0.0,
        "キャッシュを載せた instance {sounding} から音が出ていない"
    );
    assert_eq!(
        after[silent], before[silent],
        "キャッシュが無い instance {silent} が鳴ってしまっている"
    );

    play_server.stop_live_all().expect("停止できること");
    // DAW の演奏中と同じ状態（auto gain off）へ戻してから終わる。
    play_server
        .set_live_auto_gain_enabled(false)
        .expect("auto gain を戻せること");
}

/// 実サーバーで**演奏ループを丸ごと**走らせ、小節境界に state load が無いことを見る。
///
/// [`a_real_server_makes_sound_only_for_the_row_that_has_a_cached_wav`] が 1 小節ぶんの
/// 送信だけを見るのに対し、こちらは `LiveCachePlayLoop::run` をそのまま走らせて
/// **小節が進むこと・先読みが当たること・停止で止まること**まで見る。
/// `DawApp` を組み立てずに済むのは、ループが `DawApp` ではなく Arc の束だけを持つため。
///
/// **この Stage の受け入れ条件はここ。** 2 小節目以降が
/// `preload=hit prepare_ms=0.0` になっていれば、小節境界で `prepare_live_patch` を
/// 1 件も出していないということ（`prepare_ms` は境界での state load だけを計っている）。
/// サーバーログ側でも、`cmrt-bank-patch ... kind=prepare` が小節境界ではなく
/// **小節の途中**に並ぶ形になる。
#[test]
fn a_real_server_gets_its_next_measure_preloaded_so_the_boundary_only_sends_note_on() {
    let Some((play_server, wav)) = real_server_from_env() else {
        return;
    };

    // `measure_samples` は**ステレオのインターリーブ済み要素数**なので、実時間は
    // `samples / (sample_rate * 2)`。24000 要素 = 0.25 秒で、待たされずに
    // 複数小節ぶんの往復が見られる。
    const MEASURE_SAMPLES: usize = 24_000;
    const SAMPLE_RATE: u32 = 48_000;

    let play_state = Arc::new(Mutex::new(crate::DawPlayState::Playing));
    let play_position = Arc::new(Mutex::new(None));
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let sent_track_gains = Arc::new(Mutex::new(Vec::new()));
    let play_loop = crate::playback::live_cache::LiveCachePlayLoop {
        play_server: Arc::new(play_server),
        play_state: Arc::clone(&play_state),
        play_position: Arc::clone(&play_position),
        ab_repeat: Arc::new(Mutex::new(crate::AbRepeatState::Off)),
        // 2 小節ぶんの中身がある。空でない小節の数がループの長さになる。
        measure_mmls: Arc::new(Mutex::new(vec!["cde".to_string(), "efg".to_string()])),
        measure_samples: Arc::new(Mutex::new(MEASURE_SAMPLES)),
        log_lines: Arc::clone(&log_lines),
        sample_rate: SAMPLE_RATE,
        tempo_bpm: 120.0,
        beat_numerator: 4,
        tracks: 4, // 行 2 と行 3
        // 行 2 だけキャッシュがあり、行 3 はどの小節にも無い。
        ready_cache_wav: Arc::new(move |_measure_index, row| (row == 2).then(|| wav.clone())),
        initial_track_gains: crate::playback::live_gain::live_track_gains(4, |_| -3, |_| true),
        sent_track_gains: Arc::clone(&sent_track_gains),
        startup: crate::playback::DawPlaybackStartupState::default(),
    };

    let handle = std::thread::spawn(move || play_loop.run(0));
    // 0.25 秒 × 2 小節ぶん + 余裕。ループが 1 周して meas1 へ戻るところまで見る。
    std::thread::sleep(std::time::Duration::from_millis(900));
    let position_while_playing = play_position.lock().unwrap().clone();
    *play_state.lock().unwrap() = crate::DawPlayState::Idle;
    handle.join().expect("演奏ループが停止すること");

    let logged: Vec<String> = log_lines.lock().unwrap().iter().cloned().collect();
    let failures: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(failures.is_empty(), "送信に失敗した行がある: {failures:?}");

    // 小節が進むたびに、キャッシュのある行だけへ送り直している。
    let sent: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains(": live-cache "))
        .collect();
    assert!(
        sent.len() >= 3,
        "小節が進んでいない（送信ログ {} 行）: {logged:?}",
        sent.len()
    );
    for line in &sent {
        assert!(
            line.contains(" sent=row2/i0 silent=row3 over_limit=- "),
            "小節ごとの内訳が変わっている: {line}"
        );
        assert!(
            line.contains(" note_on_ms="),
            "note on の時間がログに無い: {line}"
        );
    }
    // 2 小節を巡回している（meas1 → meas2 → meas1 …）。
    assert!(sent.iter().any(|line| line.starts_with("meas1:")));
    assert!(sent.iter().any(|line| line.starts_with("meas2:")));
    assert!(
        !sent.iter().any(|line| line.starts_with("meas3:")),
        "空の小節まで進んでいる: {sent:?}"
    );

    // ── この Stage の本題 ──────────────────────────────────────
    // 1 小節目だけは先読みの元が無いので `miss`。2 小節目以降は必ず当たっていること。
    let (first, steady) = sent.split_at(1);
    assert!(
        first[0].contains(" preload=miss "),
        "演奏開始の 1 小節目は先読みの元が無いので miss のはず: {}",
        first[0]
    );
    for line in steady {
        assert!(
            line.contains(" preload=hit "),
            "先読みが当たっていない（境界で state load している）: {line}"
        );
        assert!(
            line.contains(" prepare_ms=0.0 "),
            "小節境界で state load を出している: {line}"
        );
    }
    // 先読みそのものは動いている（小節の途中で実際に state load している）。
    let preload_millis: Vec<f64> = sent
        .iter()
        .filter_map(|line| line.split(" next_ms=").nth(1))
        .map(|rest| rest.split_whitespace().next().unwrap().parse().unwrap())
        .collect();
    eprintln!("live-cache: next_ms={preload_millis:?}");
    assert_eq!(preload_millis.len(), sent.len(), "先読みの時間がログに無い");
    assert!(
        preload_millis.iter().all(|ms| *ms > 0.0),
        "先読みを 1 度も出していない: {preload_millis:?}"
    );
    // 隣の小節は別スロットへ載る（同じスロットだと鳴らす直前に上書きしてしまう）。
    assert!(
        sent.iter()
            .any(|line| line.contains("meas1: live-cache slot=0 ")),
        "meas1 がスロット 0 に載っていない: {sent:?}"
    );
    assert!(
        sent.iter()
            .any(|line| line.contains("meas2: live-cache slot=1 ")),
        "meas2 がスロット 1 に載っていない: {sent:?}"
    );

    // 演奏開始時に mixer の音量を全 track ぶんまとめて送っている。
    // ここを送り忘れると、演奏を始めた瞬間だけ前回の gain のまま鳴る。
    assert_eq!(
        logged.first().map(String::as_str),
        Some("live-gain: row2/i0=-3dB row3/i1=-3dB"),
        "演奏開始時の gain 送信が先頭に無い: {logged:?}"
    );
    // 停止で記録は空へ戻る（次の演奏では全 track ぶん送り直す）。
    assert!(sent_track_gains.lock().unwrap().is_empty());

    // 演奏中はどこを鳴らしているかが `PlayPosition` に出ている。
    let position = position_while_playing.expect("演奏中は再生位置が入っていること");
    assert!(position.measure_index < 2);
    assert_eq!(
        position.measure_duration,
        std::time::Duration::from_secs_f64(MEASURE_SAMPLES as f64 / (f64::from(SAMPLE_RATE) * 2.0))
    );
}
