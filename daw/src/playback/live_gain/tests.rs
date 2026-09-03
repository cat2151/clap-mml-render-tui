use std::time::{Duration, Instant};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use super::super::real_server::real_server_from_env;
use super::{
    changed_live_track_gains, format_live_gain_log, live_track_gains, LiveTrackGain,
    SILENT_TRACK_GAIN_DB,
};
use crate::live_instance::MAX_LIVE_TRACKS;
use crate::tracks::FIRST_PLAYABLE_TRACK;

/// SHM のコマンドが受け付ける instance gain の上限（振幅倍率）。
///
/// play server 側 `realtime-ipc/src/windows/command.rs` の `MAX_INSTANCE_GAIN_MILLI`
/// （4000 = 振幅 4.0 = 約 +12 dB）と同じ値。これを超える dB を送るとコマンドが
/// **拒否される**（実サーバーで踏んだ: `instance gain must be between 0.0 and 4`）。
const MAX_SHM_INSTANCE_GAIN: f32 = 4.0;

/// 判定用に一瞬だけ送る大きめの gain。
///
/// [`MAX_SHM_INSTANCE_GAIN`] の 4.0 はちょうど +12.041 dB で、dB から戻すと丸めで
/// 4.0000005 になり**拒否される**。1 段内側の切りのよい値にしてある。
const LOUD_GAIN_DB: f32 = 12.0;

fn amplitude_from_db(gain_db: f32) -> f32 {
    10.0f32.powf(gain_db / 20.0)
}

/// mixer の音量そのままで、全 track が可聴な状態。
fn audible(tracks: usize, volume_db: impl Fn(usize) -> i32) -> Vec<LiveTrackGain> {
    live_track_gains(tracks, volume_db, |_| true)
}

/// 送る dB は mixer が見せている dB そのもの（振幅へ直して焼き込まない）。
#[test]
fn the_db_sent_to_the_live_mix_is_the_one_the_mixer_shows() {
    let gains = audible(5, |row| match row {
        2 => -6,
        3 => 0,
        _ => 3,
    });

    assert_eq!(
        gains,
        vec![
            LiveTrackGain {
                row: 2,
                instance: 0,
                gain_db: -6.0
            },
            LiveTrackGain {
                row: 3,
                instance: 1,
                gain_db: 0.0
            },
            LiveTrackGain {
                row: 4,
                instance: 2,
                gain_db: 3.0
            },
        ]
    );
}

/// 音を出さない行（Tempo / chord）は instance を持たないので送る先が無い。
#[test]
fn the_rows_that_never_make_sound_have_nothing_to_send() {
    let gains = audible(4, |_| 0);

    assert_eq!(
        gains.iter().map(|gain| gain.row).collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(FIRST_PLAYABLE_TRACK, 2);
}

/// instance 上限を超えた行は鳴らないので、gain も送らない。
#[test]
fn rows_beyond_the_instance_limit_get_no_gain() {
    let last_playable_row = FIRST_PLAYABLE_TRACK + MAX_LIVE_TRACKS - 1;
    let gains = audible(last_playable_row + 3, |_| 0);

    assert_eq!(gains.len(), MAX_LIVE_TRACKS);
    assert_eq!(gains.last().map(|gain| gain.row), Some(last_playable_row));
}

/// 可聴でなくなった track へは無音相当の dB が送られる（0.0 ではなく -120dB）。
#[test]
fn a_track_that_is_not_audible_is_told_to_go_silent() {
    // 行 3 だけ solo。行 2 と行 4 は聞こえなくなる。
    let gains = live_track_gains(5, |_| -6, |row| row == 3);

    assert_eq!(
        gains
            .iter()
            .map(|gain| (gain.row, gain.gain_db))
            .collect::<Vec<_>>(),
        vec![
            (2, SILENT_TRACK_GAIN_DB),
            (3, -6.0),
            (4, SILENT_TRACK_GAIN_DB)
        ]
    );
}

/// 音量キー 1 回で送るコマンドはちょうど 1 つ。触っていない track は送り直さない。
#[test]
fn one_volume_step_sends_exactly_one_instance_gain() {
    let before = audible(5, |_| 0);
    let after = audible(5, |row| if row == 3 { -3 } else { 0 });

    let changed = changed_live_track_gains(&before, &after);

    assert_eq!(
        changed,
        vec![LiveTrackGain {
            row: 3,
            instance: 1,
            gain_db: -3.0
        }]
    );
}

/// solo を入れると、可聴でなくなった track だけへ無音相当が飛ぶ。
#[test]
fn turning_solo_on_only_sends_the_tracks_that_stopped_being_audible() {
    let before = audible(5, |_| -6);
    let after = live_track_gains(5, |_| -6, |row| row == 3);

    let changed = changed_live_track_gains(&before, &after);

    assert_eq!(
        changed
            .iter()
            .map(|gain| (gain.row, gain.gain_db))
            .collect::<Vec<_>>(),
        vec![(2, SILENT_TRACK_GAIN_DB), (4, SILENT_TRACK_GAIN_DB)]
    );
}

/// mixer が変わっていない同期では 1 バイトも送らない。
///
/// `sync_playback_mml_state()` はセル編集でも呼ばれるので、ここが空でないと
/// 打鍵のたびに IPC が飛ぶ。
#[test]
fn a_sync_that_did_not_touch_the_mixer_sends_nothing() {
    let gains = audible(6, |row| row as i32 - 4);

    assert!(changed_live_track_gains(&gains, &gains).is_empty());
}

/// まだ 1 度も送っていない状態（＝演奏開始直後）では全 track ぶん送る。
#[test]
fn the_first_push_of_a_performance_sends_every_track() {
    let gains = audible(6, |_| -3);

    assert_eq!(changed_live_track_gains(&[], &gains), gains);
}

/// ログ 1 行で「どの行へ何 dB 送ったか」が読める。
#[test]
fn the_log_line_shows_the_row_the_instance_and_the_db() {
    let gains = live_track_gains(5, |_| -6, |row| row != 4);

    assert_eq!(
        format_live_gain_log(&gains),
        "live-gain: row2/i0=-6dB row3/i1=-6dB row4/i2=mute"
    );
}

/// 送るものが無いときもログの形は崩れない。
#[test]
fn the_log_line_uses_a_dash_when_nothing_was_sent() {
    assert_eq!(format_live_gain_log(&[]), "live-gain: -");
}

/// mixer が出しうる dB は、SHM のコマンドが受け付ける範囲に収まっている。
///
/// 外れると `set_live_instance_gain_db` が丸ごと拒否されて**音量が効かなくなる**。
/// 実サーバー無しで、mixer 側の定数を動かした瞬間に気付けるようにしてある。
#[test]
fn every_db_the_mixer_can_produce_is_accepted_by_the_shared_memory_command() {
    for volume_db in cmrt_tui_core::mixer::MIXER_MIN_DB..=cmrt_tui_core::mixer::MIXER_MAX_DB {
        let amplitude = amplitude_from_db(volume_db as f32);
        assert!(
            (0.0..=MAX_SHM_INSTANCE_GAIN).contains(&amplitude),
            "mixer の {volume_db}dB が SHM の範囲外（振幅 {amplitude}）"
        );
    }
    let silent = amplitude_from_db(SILENT_TRACK_GAIN_DB);
    assert!((0.0..=MAX_SHM_INSTANCE_GAIN).contains(&silent));
    // 振幅は 1/1000 刻みで送られるので、-120dB は実際に振幅 0.0（＝完全な無音）になる。
    assert_eq!((silent * 1_000.0).round(), 0.0);
}

/// live mix の master limiter が効き始める / 収まるまで待って、掛かった時間を返す。
///
/// `peak_reduction_db` はサーバーが publish するたびに 0 へ戻る「前回 publish 以降の山」
/// なので、鳴り止めば自然に 0 へ落ちる。`current` と併せて見れば取りこぼしが無い。
fn wait_for_limiter(
    play_server: &RealtimePlayServerSupervisor,
    reached: impl Fn(f32) -> bool,
    timeout: Duration,
) -> Option<(Duration, f32)> {
    let started = Instant::now();
    loop {
        let meter = play_server.limiter_meter();
        let reduction = meter.current_reduction_db.max(meter.peak_reduction_db);
        if reached(reduction) {
            return Some((started.elapsed(), reduction));
        }
        if started.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 演奏中に送った gain が**その場で** live mix へ効くことを、実サーバーで機械的に見る。
///
/// 環境変数（`CMRT_LIVE_CACHE_TEST_PORT` / `CMRT_LIVE_CACHE_TEST_WAV`）が
/// 揃っているときだけ走る。走らせ方は [`super::super::real_server`] の doc を参照。
///
/// **判定は master limiter の gain reduction。** limiter は全 instance を混ぜた**後**に
/// 掛かる（サーバー側 `player/limiter.rs`）ので、instance gain が mix へ届いていなければ
/// 動かない。`auto_gain_db` は使えない: auto gain は instance の**生の出力**の RMS から
/// 決まる（`live_mix.rs` で `process_block(&samples, ..)` に渡すのは gain を掛ける前の
/// サンプル）ので、mixer の gain をいくら動かしても値は変わらない。
///
/// 音量: 判定のために一瞬だけ [`LOUD_GAIN_DB`] を送る。出力は limiter の天井
/// （-1 dBFS）で頭打ちになるので壊れはしないが、**はっきり聞こえる大きさで
/// 0.1 秒ほど鳴る**。検出できた時点ですぐ無音相当へ落としてある。
#[test]
fn a_real_server_applies_a_mixer_gain_change_within_a_few_audio_blocks() {
    let Some((play_server, wav)) = real_server_from_env() else {
        // 実サーバーが無い環境では何もしない（CI でも常に green）。
        return;
    };
    const INSTANCE: u8 = 0;
    // 小節長は約 2.4 秒。rodio 経路の遅れ（1 小節ぶん）と比べて桁違いに速いこと。
    const MEASURE: Duration = Duration::from_millis(2_400);

    // DAW の演奏中と同じ条件（auto gain off）から始める。
    play_server
        .set_live_auto_gain_enabled(false)
        .expect("auto gain を off にできること");
    // 無音相当の gain を載せてから鳴らす。ここでは音が出ない。
    play_server
        .set_live_instance_gain_db(INSTANCE, SILENT_TRACK_GAIN_DB)
        .expect("無音相当の gain を送れること");
    play_server
        .prepare_live_patch(INSTANCE, Some(&wav.to_string_lossy()))
        .expect("キャッシュ WAV を載せられること");
    play_server
        .send_midi(INSTANCE, &[[0x90, 60, 100]])
        .expect("note on を送れること");
    std::thread::sleep(Duration::from_millis(200));

    let idle = play_server.limiter_meter();
    assert!(
        idle.current_reduction_db < 0.5,
        "無音相当の gain なのに limiter が働いている: {idle:?}"
    );

    // 上げる → limiter が働くところまでを「効いた」とみなす。
    // これ以上大きい値は SHM のコマンド側で拒否される（`MAX_SHM_INSTANCE_GAIN`）。
    play_server
        .set_live_instance_gain_db(INSTANCE, LOUD_GAIN_DB)
        .expect("gain を上げられること");
    let louder = wait_for_limiter(&play_server, |reduction| reduction > 3.0, MEASURE);

    // 判定できたら即座に無音相当へ戻す（大音量で鳴らし続けない）。
    play_server
        .set_live_instance_gain_db(INSTANCE, SILENT_TRACK_GAIN_DB)
        .expect("gain を下げられること");
    let quieter = wait_for_limiter(&play_server, |reduction| reduction < 0.5, MEASURE);

    play_server.stop_live_all().expect("停止できること");

    let (raise_elapsed, reduction) =
        louder.expect("gain を上げても limiter が動かない = mix へ届いていない");
    let (lower_elapsed, _) = quieter.expect("gain を下げても limiter が戻らない");
    eprintln!(
        "live-gain: 上げて効くまで {raise_elapsed:?}（reduction {reduction} dB）/ \
         下げて戻るまで {lower_elapsed:?}"
    );
    // rodio 経路は次の小節まで（約 2.4 秒）掛からなかった。live 経路はその 1/4 未満。
    assert!(
        raise_elapsed < MEASURE / 4,
        "gain が効くまで {raise_elapsed:?} 掛かっている"
    );
    assert!(
        lower_elapsed < MEASURE / 4,
        "gain を戻すまで {lower_elapsed:?} 掛かっている"
    );
}
