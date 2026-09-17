//! **行の patch を差し替えた直後に積んだ note on が、実サーバーで音になるか**を
//! [`GridMidiSender`] の queue 越しに確かめる。
//!
//! # 何を耳の代わりにするか
//! サーバーの auto gain（`live_auto_gain_db()[i]`）は、その instance が -60 dBFS を
//! 超えるブロックを返したときだけ 0 から動き、live の patch 差し替えで 0 へ戻る。
//! したがって
//!
//! - 鳴らす → 差し替える → **何もしない** で 0 のまま = 差し替えが音を消し、物差しも勝手には動かない
//! - 鳴らす → 差し替える → **直後に note off / on を積む** で 0 から動く = ロード後に届いて鳴った
//!
//! と読める。前者が無いと、後者は「差し替えても元の音が鳴り続けていた」と区別できない。
//! 音色が新しい方かまでは分からないが、live の instance に plugin は同時 1 つしか
//! 無いので、ロード後に鳴った音は新しい patch のものしかあり得ない。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-grid-sequencer -- --include-ignored patch_reload_reattack --nocapture
//! ```

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use cmrt_realtime_play::{RealtimePlayServerSupervisor, TimelineMidiEvent};

use super::{
    super::GridMidiSender,
    test_play_server::{cfg_for_port, pick_port, TestPlayServer, PLAY_SERVER_EXE_ENV},
};

/// 1 track × 2 bank。差し替えるのは bank 0 の instance 0。
const INSTANCE_COUNT: usize = 2;
const ROW: usize = 0;
const INSTANCE_ID: u8 = 0;
const KEY: u8 = 60;
const TEMPO_BPM: f64 = 120.0;
/// auto gain が数ブロック測れるだけ鳴らす。512 フレーム ÷ 48kHz ≒ 10ms/ブロック。
const SOUND_WINDOW: Duration = Duration::from_millis(400);
/// 0 からの動きとみなす下限。丸めや 1 ブロックぶんの誤差を除くための余裕。
const MOVED_DB: f32 = 0.01;
/// cold start の Surge は 3 秒まで伸びるので、その倍を上限にする。
const LOAD_TIMEOUT: Duration = Duration::from_secs(20);

/// **差し替えの直後に積んだ note off / on が、ロード完了後に届いて鳴ること。**
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_note_queued_right_after_a_row_patch_reload_sounds_once_the_load_is_done() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let port = pick_port(55_000);
    let server = TestPlayServer::spawn(&exe, port, INSTANCE_COUNT, &[]);

    let cfg = cfg_for_port(port);
    let supervisor = Arc::new(RealtimePlayServerSupervisor::with_live_instance_count(
        &cfg,
        INSTANCE_COUNT,
    ));
    supervisor
        .ensure_started_for_fast_midi()
        .unwrap_or_else(|error| {
            panic!(
                "起動済みサーバーへ繋がらない: {error:#} / {}",
                server.stderr_text()
            )
        });
    supervisor
        .set_live_auto_gain_enabled(true)
        .expect("auto gain を有効にできない");

    let sender = GridMidiSender::new(Arc::clone(&supervisor));
    let timeline_id = sender.begin_timeline(48_000.0, TEMPO_BPM);
    let started = Instant::now();

    // 1. 鳴らす。物差しの基準。
    sender.send_scheduled(vec![note(timeline_id, 0x90, 0.0)], Duration::ZERO);
    std::thread::sleep(SOUND_WINDOW);
    let sounding = gain_db(&supervisor);
    eprintln!("patch-reload-reattack: auto_gain_db(sounding)={sounding}");
    assert!(
        sounding.abs() > MOVED_DB,
        "差し替え前の instance {INSTANCE_ID} が音を出していない"
    );

    // 2. 差し替えるだけ。音が消え、物差しも 0 のまま動かないことを見る。
    sender.set_row_patch(ROW, INSTANCE_ID, None, "reload-only");
    wait_until_loaded(&sender);
    std::thread::sleep(SOUND_WINDOW);
    let after_reload = gain_db(&supervisor);
    eprintln!("patch-reload-reattack: auto_gain_db(reload only)={after_reload}");
    assert_eq!(
        after_reload, 0.0,
        "差し替えだけで instance {INSTANCE_ID} が鳴っている（物差しが役に立たない）"
    );

    // 3. 差し替え、直後に note off / on を積む。screen 層の `prepare_patch` と同じ順。
    let at = started.elapsed().as_secs_f64();
    sender.set_row_patch(ROW, INSTANCE_ID, None, "reload-then-reattack");
    sender.send_scheduled(
        vec![note(timeline_id, 0x80, at), note(timeline_id, 0x90, at)],
        Duration::ZERO,
    );
    let load = wait_until_loaded(&sender);
    std::thread::sleep(SOUND_WINDOW);
    let reattacked = gain_db(&supervisor);
    eprintln!(
        "patch-reload-reattack: auto_gain_db(reload then reattack)={reattacked} load={load:?}"
    );
    drop(sender);
    assert!(
        reattacked.abs() > MOVED_DB,
        "差し替え直後に積んだ note on が instance {INSTANCE_ID} を鳴らしていない"
    );
    drop(server);
}

fn note(timeline_id: u64, status: u8, timeline_seconds: f64) -> TimelineMidiEvent {
    TimelineMidiEvent {
        timeline_id,
        instance_id: INSTANCE_ID,
        timeline_seconds,
        message: [status, KEY, 100],
    }
}

fn gain_db(supervisor: &RealtimePlayServerSupervisor) -> f32 {
    supervisor.live_auto_gain_db()[INSTANCE_ID as usize]
}

/// 行の patch ロードが終わるまで待ち、かかった時間を返す。
fn wait_until_loaded(sender: &GridMidiSender) -> Duration {
    let started = Instant::now();
    loop {
        let status = sender.status();
        if !status.row_patch_is_loading() {
            assert!(
                status.row_patch.is_none(),
                "行の patch ロードが失敗した: {:?}",
                status.row_patch
            );
            return started.elapsed();
        }
        assert!(
            started.elapsed() < LOAD_TIMEOUT,
            "行の patch ロードが {LOAD_TIMEOUT:?} 経っても終わらない"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
