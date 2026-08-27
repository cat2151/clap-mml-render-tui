//! 先読み（`PrepareStandbyPatch`）を**実サーバーへ繋いで**確かめるテスト。
//!
//! 2 repository に二重定義された SHM プロトコルの食い違いも、bank worker が
//! 分かれているかも、ここでしか機械的に検出できない。

use super::harness::{
    number_field, thread_id_of, TestPlayServer, PATCH_LOAD_DELAY_ENV, PLAY_SERVER_EXE_ENV,
    SLOW_PATCH_LOAD_MS, SPARE_INSTANCES_ENV, STANDBY_PATCH_ENV,
};

/// 先読み専用コマンドが**実サーバー**へ届き、bank 1 の要求として処理されること。
///
/// 共有メモリのプロトコルは 2 repository に二重定義されている（こちらの
/// `fast_midi_ipc/windows/protocol.rs` と play-server の
/// `realtime-ipc/src/windows/protocol.rs`）。片方だけ直しても両 repository の
/// 単体テストは緑のまま通り、**実際に繋いだときだけ黙って動かない**。
/// ここを通すことでしか、その食い違いは機械的に検出できない。
///
/// ```text
/// $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
/// cargo test -p cmrt-realtime-play -- --include-ignored standby
/// ```
///
/// サーバーは自分で起こす（supervisor には起こさせない）。supervisor 経由だと
/// ポートが config.toml 固定になり、**起動中の TUI とポートを取り合う**ため。
/// 環境変数は子プロセスにだけ渡すので、このプロセスの環境は汚さない。
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn the_standby_preload_reaches_a_real_play_server() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    // 起動中の TUI の既定ポート（62154）から離す。
    let port = 45_000 + (std::process::id() % 1_000) as u16;
    // 2 instance = 1 instance ずつの 2 bank。起動を最小にしつつ bank 境界は成立する。
    let server = TestPlayServer::spawn(&exe, port, 2);

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor = crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, 2);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");

    // instance 1 = 待機 bank（bank 1）。先読み専用コマンドで届く。
    supervisor
        .prepare_standby_patch(1, None)
        .expect("待機 bank への先読みが失敗した");
    // instance 0 = 演奏 bank（bank 0）。既存の経路も壊れていない。
    supervisor
        .prepare_live_patch(0, None)
        .expect("現在 bank の音色差し替えが失敗した");
    // 設定した live instance 数の外はサーバーが断る（bank 境界の検証）。
    let error = supervisor
        .prepare_standby_patch(2, None)
        .expect_err("範囲外の instance が通ってしまった");
    let message = format!("{error:#}");
    assert!(message.contains("outside"), "{message}");

    // サーバー側でも「先読みとして」「bank 1 の要求として」扱われたこと。
    // クライアントが成功を受け取っただけでは、bank の判定までは確かめられない。
    server.wait_for_stderr_line(|line| line.contains("kind=prepare-standby-patch instance=1"));
    let handled = server.wait_for_stderr_line(|line| {
        line.starts_with("cmrt-standby-patch: bank=1 local=0 instance=1")
    });
    assert!(handled.contains("result=ok"), "{handled}");
}

/// **先読みのロードが、演奏中の bank を回しているのとは別の OS thread で走ること。**
///
/// bank worker 分離の合否そのもの（受け入れ条件 1）。サーバーが出す 2 種類の行の
/// `thread=` を突き合わせて機械判定する。クライアント側からは「成功した」以上のことは
/// 分からないので、ここはサーバーの stderr でしか確かめられない。
///
/// ```text
/// $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
/// cargo test -p cmrt-realtime-play -- --include-ignored bank
/// ```
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn the_standby_load_runs_on_a_different_thread_than_the_active_bank_render() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    // 同時に走る別のテストのサーバーとも、起動中の TUI（既定 62154）とも衝突させない。
    let port = 46_000 + (std::process::id() % 1_000) as u16;
    let server = TestPlayServer::spawn(&exe, port, 2);

    // bank ごとに worker が 1 本ずつ立っていること。
    let bank0_worker = server
        .wait_for_stderr_line(|line| line.starts_with("cmrt-bank-worker: bank=0 event=started"));
    let bank1_worker = server
        .wait_for_stderr_line(|line| line.starts_with("cmrt-bank-worker: bank=1 event=started"));
    assert_ne!(
        thread_id_of(&bank0_worker),
        thread_id_of(&bank1_worker),
        "bank 0 と bank 1 が同じ thread に載っている:\n{bank0_worker}\n{bank1_worker}"
    );

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor = crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, 2);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");

    // instance 0 = 演奏 bank（bank 0）。鳴らし始めて render を回す。
    supervisor
        .send_midi(0, &[[0x90, 60, 100]])
        .expect("演奏 bank への note on が失敗した");
    let render = server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-render: bank=0 "));

    // instance 1 = 待機 bank（bank 1）。先読み専用コマンドでロードする。
    supervisor
        .prepare_standby_patch(1, None)
        .expect("待機 bank への先読みが失敗した");
    let load = server.wait_for_stderr_line(|line| {
        line.starts_with("cmrt-bank-patch: bank=1 ") && line.contains("kind=prepare")
    });
    assert!(load.contains("result=ok"), "{load}");

    // 分離できていれば、ロードした thread と演奏 bank を render している thread は別。
    assert_ne!(
        thread_id_of(&render),
        thread_id_of(&load),
        "先読みが演奏 bank と同じ thread で走っている:\n{render}\n{load}"
    );
    // ロードしたのは bank 1 の worker 自身であること（coordinator でも IPC でもない）。
    assert_eq!(
        thread_id_of(&load),
        thread_id_of(&bank1_worker),
        "bank 1 の worker 以外がロードしている:\n{load}\n{bank1_worker}"
    );
    assert_eq!(thread_id_of(&render), thread_id_of(&bank0_worker));
}

/// **人工的に 500ms 止めた先読みロードの最中も、演奏 bank が render を続けること。**
///
/// 受け入れ条件 2 そのもの。実プラグインのロード時間はマシン依存で、テストから
/// 任意の長さ止めるのも不安定なので、サーバー側に唯一のテスト注入点
/// （[`PATCH_LOAD_DELAY_ENV`]）を置いて、その環境変数を**子プロセスにだけ**渡す。
///
/// 判定はサーバーが出す 1 行で完結する。
///
/// ```text
/// cmrt-standby-load: bank=1 event=finish instance=1 elapsed_ms=5xx blocks_elsewhere=NN underrun_frames=0 skipped=0 result=ok
/// ```
///
/// `blocks_elsewhere` は「このロードの間に**対象 bank 以外**が render したブロック数」。
/// coordinator がロードの返事を待って止まっていれば 0 になる（Stage 2 まではそうだった）。
///
/// ```text
/// $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
/// cargo test -p cmrt-realtime-play -- --include-ignored slow_standby
/// ```
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn the_active_bank_keeps_rendering_during_a_slow_standby_load() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let port = 47_000 + (std::process::id() % 1_000) as u16;
    let server = TestPlayServer::spawn_with_env(
        &exe,
        port,
        2,
        &[(PATCH_LOAD_DELAY_ENV, SLOW_PATCH_LOAD_MS.to_string())],
    );

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor = crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, 2);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");

    // instance 0 = 演奏 bank（bank 0）。鳴らし始めて render を回し続ける。
    supervisor
        .send_midi(0, &[[0x90, 60, 100]])
        .expect("演奏 bank への note on が失敗した");
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-render: bank=0 "));

    // instance 1 = 待機 bank（bank 1）。ここで 500ms 止まる。
    supervisor
        .prepare_standby_patch(1, None)
        .expect("待機 bank への先読みが失敗した");

    // 止めたのは bank 1 の worker の中だけであること。
    let delayed =
        server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-patch-delay: bank=1 "));
    assert!(
        delayed.contains(&format!("ms={SLOW_PATCH_LOAD_MS}")),
        "{delayed}"
    );

    let finished = server
        .wait_for_stderr_line(|line| line.starts_with("cmrt-standby-load: bank=1 event=finish"));
    assert!(finished.contains("result=ok"), "{finished}");
    let elapsed_ms = number_field(&finished, "elapsed_ms");
    assert!(
        elapsed_ms >= SLOW_PATCH_LOAD_MS,
        "人工遅延が効いていない: {finished}"
    );
    // ここが本題。ロード中も演奏 bank が回っていれば、ブロック数は 0 のままにならない。
    let blocks_elsewhere = number_field(&finished, "blocks_elsewhere");
    assert!(
        blocks_elsewhere >= 10,
        "ロード中に演奏 bank が {blocks_elsewhere} ブロックしか進んでいない: {finished}"
    );
    // 出力リングへの push も続いていること（止まっていれば underrun が増える）。
    assert_eq!(
        number_field(&finished, "underrun_frames"),
        0,
        "ロード中に出力が途切れた: {finished}"
    );
    // 待機 bank には鳴らすものが無い、という client 契約が守られていること。
    // 破れていれば、その instance の render がロードの後ろへ回された回数が出る。
    assert_eq!(
        number_field(&finished, "skipped"),
        0,
        "先読み中の bank に鳴らすものがあった: {finished}"
    );
}

/// **プラグイン種別が変わる先読みでも、演奏 bank が回り続けること。**
///
/// Stage 4 の中身そのもの。物理インスタンスの入れ替えも、予備の袋の出し入れも、
/// 袋が尽きたときの背景生成待ちも、すべて対象 bank worker の中で起きるようになった。
/// ここはその経路を**実プラグインで**通す。
///
/// 予備を 1 個（`CMRT_SPARE_INSTANCES=1`）に絞ると、割り当ては bank 0 へ寄って
/// **bank 1 の前払いは 0 になる**。つまりこの先読みは「袋が空 → その場で発注 →
/// 背景生成を待つ」という最も待たされる経路を必ず通る。それでも演奏 bank は
/// 止まってはならない（テスト計画の単体テスト 7）。
///
/// 音色パスはマシンごとに違うので環境変数で受ける。渡さなければ skip する
/// （既定音色ぶんの経路は他のテストが見ている）。
///
/// ```text
/// $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
/// $env:CMRT_TEST_STANDBY_PATCH = "...\Dexed_01.syx\00 test"
/// cargo test -p cmrt-realtime-play -- --include-ignored cross_plugin
/// ```
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_cross_plugin_standby_preload_keeps_the_active_bank_rendering() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let Ok(patch) = std::env::var(STANDBY_PATCH_ENV) else {
        // 未検証を成功と誤認しないよう、何が足りなくて飛ばしたのかを必ず出す。
        eprintln!("skip: {STANDBY_PATCH_ENV} が無いので、プラグインをまたぐ先読みは確かめていない");
        return;
    };
    let port = 48_000 + (std::process::id() % 1_000) as u16;
    let server = TestPlayServer::spawn_with_env(
        &exe,
        port,
        2,
        &[
            (PATCH_LOAD_DELAY_ENV, SLOW_PATCH_LOAD_MS.to_string()),
            // bank 1 の前払いを 0 にして、必ず背景生成待ちを通す。
            (SPARE_INSTANCES_ENV, "1".to_string()),
        ],
    );

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor = crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, 2);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");

    // instance 0 = 演奏 bank（bank 0）。鳴らし始めて render を回し続ける。
    supervisor
        .send_midi(0, &[[0x90, 60, 100]])
        .expect("演奏 bank への note on が失敗した");
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-render: bank=0 "));

    // instance 1 = 待機 bank（bank 1）。既定と違うプラグインの音色を先読みする。
    supervisor
        .prepare_standby_patch(1, Some(&patch))
        .unwrap_or_else(|error| panic!("待機 bank への先読みが失敗した ({patch}): {error:#}"));

    // 物理インスタンスの入れ替えが**bank 1 の worker の中で**起きたこと。
    // `swapped=false` なら、渡した音色が既定プラグインのものだった
    // （＝この実行はプラグインまたぎを確かめていない）。
    let load = server.wait_for_stderr_line(|line| {
        line.starts_with("cmrt-bank-patch: bank=1 ") && line.contains("kind=prepare")
    });
    assert!(load.contains("result=ok"), "{load}");
    assert!(
        load.contains("swapped=true"),
        "プラグイン種別が変わっていない（{STANDBY_PATCH_ENV} が既定プラグインの音色）: {load}"
    );

    let finished = server
        .wait_for_stderr_line(|line| line.starts_with("cmrt-standby-load: bank=1 event=finish"));
    assert!(finished.contains("result=ok"), "{finished}");
    let blocks_elsewhere = number_field(&finished, "blocks_elsewhere");
    assert!(
        blocks_elsewhere >= 10,
        "差し替えの間に演奏 bank が {blocks_elsewhere} ブロックしか進んでいない: {finished}"
    );
    assert_eq!(
        number_field(&finished, "underrun_frames"),
        0,
        "差し替えの間に出力が途切れた: {finished}"
    );
    assert_eq!(
        number_field(&finished, "skipped"),
        0,
        "先読み中の bank に鳴らすものがあった: {finished}"
    );
}
