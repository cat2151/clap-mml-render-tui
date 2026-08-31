//! 先読みが終わった待機 bank へ切り替えたあと、**その instance が本当に音を出すか**を
//! 実サーバーで確かめる（テスト計画「統合テスト用 instrument」5 の機械化）。
//!
//! # 耳を使わずに「鳴っているか」を判定する仕組み
//! サーバーの auto gain は instance ごとに、**その instance が返したブロックの RMS**から
//! trim を決める（play-server の `player/auto_gain.rs`）。無音（-60 dBFS 未満）の
//! ブロックでは値を動かさないので、
//!
//! - `live_auto_gain_db()[i]` が 0 のまま = その instance は 1 ブロックも音を出していない
//! - 0 から動いた = その instance が実際に音を出した
//!
//! と読める。この値は SHM 越しにクライアントから読めるので、耳もオーディオ出力の
//! 録音も要らない。**音色や音程が正しいかまでは分からない**（そこは今も人間の耳）。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-realtime-play -- --include-ignored bank_switch
//! ```

use std::time::Duration;

use super::harness::{pick_port, TestPlayServer, PLAY_SERVER_EXE_ENV};

/// 2 track × 2 bank。instance 0,1 が bank 0、2,3 が bank 1。
const INSTANCE_COUNT: usize = 4;
/// auto gain が数ブロック測れるだけ鳴らす。512 フレーム ÷ 48kHz ≒ 10ms/ブロック。
const SOUND_WINDOW: Duration = Duration::from_millis(400);
/// 0 からの動きとみなす下限。丸めや 1 ブロックぶんの誤差を除くための余裕。
const MOVED_DB: f32 = 0.01;

/// **先読みした待機 bank へ切り替えると、その instance が実際に音を出すこと。**
///
/// 「止めずにロードできた」だけでは、切り替えた先が無音でも気づけない。
/// ここは切替後の最初の note on が新しくロードした instance へ届き、
/// その instance が音を出したところまでを見る。
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn the_standby_bank_makes_sound_after_the_switch() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let port = pick_port(51_000);
    let server = TestPlayServer::spawn(&exe, port, INSTANCE_COUNT);

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor =
        crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, INSTANCE_COUNT);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");
    // auto gain は既定で切ってある。これが「鳴ったか」の物差しなので入れる。
    supervisor
        .set_live_auto_gain_enabled(true)
        .expect("auto gain を有効にできない");

    // 演奏 bank（bank 0）を鳴らす。ここが物差しの基準になる。
    for instance in 0..2u8 {
        supervisor
            .send_midi(instance, &[[0x90, 60 + instance * 5, 100]])
            .expect("演奏 bank への note on が失敗した");
    }
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-render: bank=0 "));
    std::thread::sleep(SOUND_WINDOW);

    let before = supervisor.live_auto_gain_db();
    eprintln!(
        "bank-switch: auto_gain_db(bank0 playing)={:?}",
        &before[..4]
    );
    for instance in 0..2 {
        assert!(
            before[instance].abs() > MOVED_DB,
            "演奏 bank の instance {instance} が音を出していない: {before:?}"
        );
    }
    // 待機 bank はまだ 1 音も鳴らしていない。物差しが「常に動く」ものでないこと。
    for instance in 2..4 {
        assert_eq!(
            before[instance], 0.0,
            "まだ鳴らしていない待機 bank の instance {instance} に値が付いている: {before:?}"
        );
    }

    // 待機 bank を 1 件ずつ先読みする。ここで見たいのは auto gain の値なので、
    // ロード完了まで待つ同期 wrapper のほうが素直（grid 本体は非同期 API を使う）。
    for instance in 2..INSTANCE_COUNT as u8 {
        supervisor
            .prepare_standby_patch(instance, None)
            .expect("待機 bank への先読みが失敗した");
    }
    server.wait_for_stderr_line(|line| {
        line.starts_with("cmrt-standby-load: bank=1 event=finish instance=3")
    });

    // bank 切替。切り替えた先へ最初の note on を送る。
    for instance in 2..INSTANCE_COUNT as u8 {
        supervisor
            .send_midi(instance, &[[0x90, 60 + instance * 5, 100]])
            .expect("切替後の bank への note on が失敗した");
    }
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-render: bank=1 "));
    std::thread::sleep(SOUND_WINDOW);

    let after = supervisor.live_auto_gain_db();
    eprintln!("bank-switch: auto_gain_db(bank1 playing)={:?}", &after[..4]);
    for instance in 2..4 {
        assert!(
            after[instance].abs() > MOVED_DB,
            "先読みした instance {instance} が切替後に音を出していない: {after:?}"
        );
    }
}
