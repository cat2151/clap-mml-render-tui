mod state_load;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::super::real_server::real_server_from_env;
use super::{
    format_live_cache_measure_log, measure_live_cues, note_on_events, ready_cache_wav_for_measure,
    send_measure_cues, LiveCacheCue, MeasureSendTiming,
};
use crate::live_instance::MAX_LIVE_TRACKS;
use crate::tracks::FIRST_PLAYABLE_TRACK;

fn wav(row: usize) -> PathBuf {
    PathBuf::from(format!("C:/daw_cache/track{row}_meas1.wav"))
}

/// 全行にキャッシュがある小節は、行 2 から順に instance 0 から埋まる。
#[test]
fn every_row_with_a_cache_gets_the_instance_that_matches_its_grid_row() {
    let cues = measure_live_cues(5, |row| Some(wav(row)));

    assert_eq!(
        cues.cues,
        vec![
            LiveCacheCue {
                row: 2,
                instance: 0,
                wav: wav(2)
            },
            LiveCacheCue {
                row: 3,
                instance: 1,
                wav: wav(3)
            },
            LiveCacheCue {
                row: 4,
                instance: 2,
                wav: wav(4)
            },
        ]
    );
    assert!(cues.silent_rows.is_empty());
    assert!(cues.rows_over_instance_limit.is_empty());
}

/// Tempo 行（0）と chord 行（1）は音を鳴らさないので、送信対象にも無音扱いにも入らない。
#[test]
fn the_tempo_and_chord_rows_are_not_part_of_the_measure_at_all() {
    let cues = measure_live_cues(4, |row| Some(wav(row)));

    let touched: Vec<usize> = cues
        .cues
        .iter()
        .map(|cue| cue.row)
        .chain(cues.silent_rows.iter().copied())
        .chain(cues.rows_over_instance_limit.iter().copied())
        .collect();
    assert_eq!(touched, vec![2, 3]);
    assert_eq!(FIRST_PLAYABLE_TRACK, 2);
}

/// キャッシュがまだ無い行には何も送らない（承認済みの設計判断 1: そこは無音のまま）。
#[test]
fn a_row_without_a_cached_wav_is_left_silent_instead_of_reusing_the_previous_measure() {
    let cues = measure_live_cues(5, |row| (row != 3).then(|| wav(row)));

    assert_eq!(
        cues.cues.iter().map(|cue| cue.row).collect::<Vec<_>>(),
        vec![2, 4]
    );
    assert_eq!(cues.silent_rows, vec![3]);
    // 送らない行の instance は空いたままにする（詰めて別の行を鳴らしたりしない）。
    assert_eq!(
        cues.cues.iter().map(|cue| cue.instance).collect::<Vec<_>>(),
        vec![0, 2]
    );
}

/// instance 数はサーバー起動時にしか決まらないので、溢れた行は鳴らさず内訳へ落とす。
#[test]
fn rows_beyond_the_server_instance_limit_are_reported_instead_of_played() {
    let last_playable_row = FIRST_PLAYABLE_TRACK + MAX_LIVE_TRACKS - 1;
    let cues = measure_live_cues(last_playable_row + 3, |row| Some(wav(row)));

    assert_eq!(cues.cues.len(), MAX_LIVE_TRACKS);
    assert_eq!(
        cues.cues.last().map(|cue| (cue.row, cue.instance)),
        Some((last_playable_row, (MAX_LIVE_TRACKS - 1) as u8))
    );
    assert_eq!(
        cues.rows_over_instance_limit,
        vec![last_playable_row + 1, last_playable_row + 2]
    );
    assert!(cues.silent_rows.is_empty());
}

/// 演奏 track が 1 つも無いグリッドでは送信も無音行も生まれない。
#[test]
fn a_grid_without_playable_rows_produces_nothing_to_send() {
    let cues = measure_live_cues(FIRST_PLAYABLE_TRACK, |row| Some(wav(row)));

    assert!(cues.cues.is_empty());
    assert!(cues.silent_rows.is_empty());
    assert!(cues.rows_over_instance_limit.is_empty());
}

/// ログ 1 行だけで「送った / 無音 / 上限超え」の内訳と、送信に掛かった時間が読める。
///
/// 送信時間は **state load（`prepare_ms`）と note on（`note_on_ms`）に分かれている**。
/// track 数に比例して伸びてよいのは前者だけで、後者が伸びたら「まとめて 1 コマンド」が
/// 壊れた合図になる。
#[test]
fn the_measure_log_line_separates_what_was_sent_from_why_the_rest_was_not() {
    let cues = measure_live_cues(5, |row| (row != 3).then(|| wav(row)));

    assert_eq!(
        format_live_cache_measure_log(
            2,
            &cues,
            MeasureSendTiming {
                prepare: Duration::from_micros(1_500),
                note_on: Duration::from_micros(120),
            }
        ),
        "meas3: live-cache sent=row2/i0,row4/i2 silent=row3 over_limit=- \
         prepare_ms=1.5 note_on_ms=0.1"
    );
}

/// 全 track の note on が、同じオーディオブロックで適用される 1 バッチになる。
///
/// ここが 1 件ずつのコマンドへ戻ると、`prepare` の応答待ちが note on のあいだに挟まって
/// 小節の頭が track ごとにずれる（Stage 6 の実測で 8 track / 250ms）。
#[test]
fn every_track_gets_its_note_on_in_one_batch_at_the_same_offset() {
    let cues = measure_live_cues(5, |row| (row != 3).then(|| wav(row)));

    let events = note_on_events(&cues.cues);
    assert_eq!(
        events
            .iter()
            .map(|event| event.instance_id)
            .collect::<Vec<_>>(),
        vec![0, 2],
        "キャッシュのある行だけが、その行の instance で鳴る"
    );
    assert!(
        events.iter().all(|event| event.offset_frames == 0),
        "小節の頭に揃えるので offset は全部 0: {events:?}"
    );
    assert!(
        events
            .iter()
            .all(|event| event.message == super::CACHE_PLAYER_NOTE_ON),
        "cache-player は音高を見ないので全部同じ note on: {events:?}"
    );
}

/// 鳴らすものが無い小節では 1 コマンドも送らない。
///
/// サーバーは空の MIDI バッチを `InvalidPayload` で弾く（1..=128 件しか受けない）。
/// 「キャッシュがまだ無いので無音」は正常な状態（判断 1）なので、エラーにしてはいけない。
#[test]
fn a_measure_with_nothing_to_play_produces_no_midi_batch_at_all() {
    let cues = measure_live_cues(5, |_| None);

    assert!(note_on_events(&cues.cues).is_empty());
}

/// 何も送っていない小節でも「空だった」ことが 1 行で分かる。
#[test]
fn the_measure_log_line_uses_a_dash_when_a_category_is_empty() {
    let cues = measure_live_cues(4, |_| None);

    assert_eq!(
        format_live_cache_measure_log(0, &cues, MeasureSendTiming::default()),
        "meas1: live-cache sent=- silent=row2,row3 over_limit=- prepare_ms=0.0 note_on_ms=0.0"
    );
}

/// 実サーバーへ本物のコマンドを送る、**通常は skip される**テスト。
///
/// `CMRT_LIVE_CACHE_TEST_PORT`（起動済み play server のポート）と
/// `CMRT_LIVE_CACHE_TEST_WAV`（キャッシュ WAV の絶対パス）が両方あるときだけ走る。
/// 個人のパスやポートをコードへ書かずに、演奏ループが使う送信経路そのもの
/// （[`send_measure_cues`]）を実サーバーで通せるようにするための形。
///
/// ```text
/// CMRT_REALTIME_PLAY_SERVER_PORT=8712 CMRT_LIVE_INSTANCE_COUNT=2 \
///   ../clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe > server.log 2>&1 &
/// CMRT_LIVE_CACHE_TEST_PORT=8712 CMRT_LIVE_CACHE_TEST_WAV=<絶対パス> \
///   cargo test -p cmrt-daw --lib live_cache -- --nocapture
/// ```
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
    send_measure_cues(&play_server, &cues, &log_lines);

    let logged = log_lines.lock().unwrap().clone();
    assert!(
        logged.is_empty(),
        "送信に失敗した行がある: {logged:?}（ログへ出るのは失敗時だけ）"
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

/// 実サーバーで**演奏ループを丸ごと**走らせる、通常は skip されるテスト。
///
/// [`a_real_server_makes_sound_only_for_the_row_that_has_a_cached_wav`] が 1 小節ぶんの
/// 送信だけを見るのに対し、こちらは [`LiveCachePlayLoop::run`] をそのまま走らせて
/// **小節が進むこと・進むたびに送り直すこと・停止で止まること**まで見る。
/// `DawApp` を組み立てずに済むのは、ループが `DawApp` ではなく Arc の束だけを持つため。
///
/// 起動と実行は上のテストと同じ環境変数で、同じコマンドで走る。
/// サーバーログ側では小節ぶんの `cmrt-bank-patch ... kind=prepare ... result=ok` と
/// `cmrt-live: event=apply-midi`、停止後の `apply-stop-all` で裏取りできる。
#[test]
fn a_real_server_gets_a_fresh_prepare_and_note_on_every_time_the_measure_advances() {
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
    let play_loop = super::LiveCachePlayLoop {
        play_server: Arc::new(play_server),
        play_state: Arc::clone(&play_state),
        play_position: Arc::clone(&play_position),
        ab_repeat: Arc::new(Mutex::new(crate::AbRepeatState::Off)),
        // 2 小節ぶんの中身がある。空でない小節の数がループの長さになる。
        measure_mmls: Arc::new(Mutex::new(vec!["cde".to_string(), "efg".to_string()])),
        measure_samples: Arc::new(Mutex::new(MEASURE_SAMPLES)),
        log_lines: Arc::clone(&log_lines),
        sample_rate: SAMPLE_RATE,
        tracks: 4, // 行 2 と行 3
        // 行 2 だけキャッシュがあり、行 3 はどの小節にも無い。
        ready_cache_wav: Arc::new(move |_measure_index, row| (row == 2).then(|| wav.clone())),
        initial_track_gains: crate::playback::live_gain::live_track_gains(4, |_| -3, |_| true),
        sent_track_gains: Arc::clone(&sent_track_gains),
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
        .filter(|line| line.contains("live-cache sent="))
        .collect();
    assert!(
        sent.len() >= 3,
        "小節が進んでいない（送信ログ {} 行）: {logged:?}",
        sent.len()
    );
    for line in &sent {
        assert!(
            line.contains("live-cache sent=row2/i0 silent=row3 over_limit=-"),
            "小節ごとの内訳が変わっている: {line}"
        );
        // 小節ごとに state load と note on の実時間が残る
        // （判断 3 と、track 間のずれを数字で見直せるように）。
        assert!(
            line.contains(" prepare_ms="),
            "state load の時間がログに無い: {line}"
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

/// セルを編集すると、その小節のキャッシュ WAV が消えて live 経路は無音になる。
///
/// 判断 1（「まだ出来ていない小節は無音のまま」）が成り立つ前提は、**編集で WAV が
/// 実際に消えること**。ここが崩れると、演奏ループは古い WAV を鳴らし続けてしまい、
/// 「編集したのに前の音が鳴る」という一番たちの悪い形の嘘になる。
/// 実サーバーもユーザーの実キャッシュも要らない（temp の cache dir で完結する）。
#[test]
fn editing_a_cell_removes_its_cache_wav_so_that_measure_falls_silent() {
    let tmp = std::env::temp_dir().join("cmrt_test_live_cache_invalidates_wav");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
        let (mut app, _cache_rx) = crate::input::tests::build_test_app();
        let row = FIRST_PLAYABLE_TRACK;
        app.editor.data[row][1] = "cde".to_string();
        app.sync_cache_states();

        // render 済みのキャッシュがある状態を作る（中身は読まないので実 WAV でなくてよい）。
        let cache_wav = crate::cache::cache_wav_path(app.workspace_kind, row, 1)
            .expect("row 2 / meas 1 has a cache path");
        std::fs::write(&cache_wav, b"cached audio").unwrap();
        assert_eq!(
            ready_cache_wav_for_measure(app.workspace_kind, 0, row),
            Some(cache_wav.clone()),
            "the loop should see the cache while it exists"
        );

        app.invalidate_cell(row, 1);

        assert!(!cache_wav.exists(), "editing the cell removes the wav");
        assert_eq!(
            ready_cache_wav_for_measure(app.workspace_kind, 0, row),
            None,
            "with no wav the loop sends nothing, so the measure is silent"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}
