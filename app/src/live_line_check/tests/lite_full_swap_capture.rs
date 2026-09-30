//! Guitar Articulation 画面の音色の差し替え（Lite で鳴らしながら裏で Full を読み、読み終えたら
//! Full で鳴らす）を、実 server に対して行の LIVE 演奏経路で行い、live mix の出力で確かめる。
//!
//! 画面と同じ順に、1 回の server で「Lite を読む → Lite の行 → Full を先読みに出す →
//! 読み込み中に Lite の行 → 先読みの完了を待つ → Full の行」を鳴らし、次を見る。
//!
//! - 読み込み中の Lite の行が、先読みに塞がれずに timeline へ積まれる
//! - server の先読みが成功し、その間に演奏 bank が underrun していない
//! - Full の行が読み込み無しに積まれ、最初の Lite の行と違う音の音がある
//!   （どちらも、その server で初めて鳴らす instance なので round robin の位置が揃う）
//!
//! 比べ方が壊れていないことの対照として、同じ手順をもう 1 回の server で繰り返し、
//! 同じ行どうしが同じ音になることも見る。録音の中の行の位置は送った時刻の差から決めるので、
//! 読み込みの間に server の render が止まると、読み込み中の行がずれて対照が崩れる。
//!
//! 実機の音色が要るので通常は skip。Lite は [`super::capture_support`] と同じ環境変数で、
//! Full はもう 1 つの環境変数で渡す:
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! $env:CMRT_TEST_KEYSWITCH_PATCH = "sfz/<library>/Programs/<Lite>.sfz"
//! $env:CMRT_TEST_GA_FULL_PATCH = "sfz/<library>/Programs/<Full>.sfz"
//! cargo test -p clap-mml-render-tui --lib lite_full_swap_capture -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

use cmrt_guitar_articulation::{convert, notes_from_events, RuleTable};
use cmrt_mml_overlay::line_play::{LinePerformance, LineProgram};
use cmrt_mml_overlay::{LivePatch, MmlOverlayPreload, MmlOverlaySender};

use super::capture_support::{
    logged_line_count, logged_lines_since, send_line, CaptureSetup, Segment, DIFFERENT, SAME,
    STEP_MS,
};
use super::{sleep_until, Sent};

const FULL_PATCH_ENV: &str = "CMRT_TEST_GA_FULL_PATCH";
/// 画面の既定の練習フレーズと同じ 3 音。
const MML: &str = "o3 l8 e f+ g";
/// 先読みの完了を待つ上限。冷えた状態の Full は 60 秒を超える。
const PRELOAD_TIMEOUT: Duration = Duration::from_secs(300);
/// 行を送ってから timeline に積まれるまでの上限。先読みに塞がれていれば読み込みの長さだけ遅れる。
const UNBLOCKED: Duration = Duration::from_millis(1_000);
/// live の録音の上限。先読みの待ちも録音に入るので、[`PRELOAD_TIMEOUT`] より長くする。
const CAPTURE_SECONDS: u32 = 420;
/// 1 行目から読み込み中の行までの間隔。3 音（0.75 秒）が鳴り終わる長さ。
const LOADING_LINE_GAP: Duration = Duration::from_millis(1_000);
/// server が先読みの終わりに出す行の目印。
const STANDBY_FINISH: &str = "cmrt-standby-load:";

/// 1 回の server の結果。
struct Session {
    /// [Lite の行, 読み込み中の Lite の行, Full の行]。
    segments: Vec<Segment>,
    /// 読み込み中の Lite の行を送ってから積まれるまで。
    lite_send: Duration,
    /// その行が積まれた時点で、先読みがまだ読み込み中だったか。
    loading_during_lite: bool,
    /// 先読みを出してから完了が見えるまで。
    preload_elapsed: Duration,
    /// Full の行を送ってから積まれるまで。
    full_send: Duration,
    /// server の `cmrt-standby-load: ... event=finish` の行。
    standby_finish: Vec<String>,
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH と CMRT_TEST_GA_FULL_PATCH）"]
fn lite_plays_while_full_loads_behind_and_the_next_play_is_full() {
    let Ok(full_patch) = std::env::var(FULL_PATCH_ENV) else {
        eprintln!("lite-full-swap: skip ({FULL_PATCH_ENV} が無い)");
        return;
    };
    let Some(setup) = CaptureSetup::from_env("lite-full-swap") else {
        return;
    };
    let lite = LivePatch::new(Some(setup.patch()));
    let full = LivePatch::new(Some(&full_patch));
    let first = session(&setup, "a", &lite, &full);
    let second = session(&setup, "b", &lite, &full);
    drop(setup);

    for (label, run) in [("a", &first), ("b", &second)] {
        eprintln!(
            "lite-full-swap: {label} lite_send_ms={} loading_during_lite={} preload_ms={} full_send_ms={}",
            run.lite_send.as_millis(),
            run.loading_during_lite,
            run.preload_elapsed.as_millis(),
            run.full_send.as_millis(),
        );
        for line in &run.standby_finish {
            eprintln!("lite-full-swap: {label} {line}");
        }
    }
    let swapped = compare("a: Lite vs Full", &first.segments[0], &first.segments[2]);
    let lite_again = compare("Lite: a vs b", &first.segments[0], &second.segments[0]);
    let loading_again = compare(
        "読み込み中の Lite: a vs b",
        &first.segments[1],
        &second.segments[1],
    );
    let full_again = compare("Full: a vs b", &first.segments[2], &second.segments[2]);

    assert!(
        first.loading_during_lite,
        "Lite の行を積む前に Full の読み込みが終わった（塞がれないことを測れていない）"
    );
    assert!(
        first.lite_send < UNBLOCKED,
        "読み込み中の Lite の行が {} ms 待たされた",
        first.lite_send.as_millis()
    );
    assert!(
        first.full_send < UNBLOCKED,
        "読み終えた Full の行が {} ms 待たされた（読み直している）",
        first.full_send.as_millis()
    );
    assert_standby_finished_cleanly(&first.standby_finish);
    assert!(
        lite_again.iter().all(|ncc| *ncc > SAME),
        "同じ手順の Lite が server ごとに違う音（比べ方が壊れている）"
    );
    assert!(
        loading_again.iter().all(|ncc| *ncc > SAME),
        "読み込み中の Lite の行が server ごとに違う音（読み込みの間に render が止まり、録音の位置がずれた）"
    );
    assert!(
        full_again.iter().all(|ncc| *ncc > SAME),
        "同じ手順の Full が server ごとに違う音（比べ方が壊れている）"
    );
    // Lite と Full は sample の大半を共有し、低い E2 はほぼ同じ波形になる。同じ版どうしは全音
    // 揃う（上の対照）ので、1 音でも違えば差し替わっている。
    assert!(
        swapped.iter().any(|ncc| *ncc < DIFFERENT),
        "Full の行が Lite と同じ音（差し替わっていない）"
    );
}

/// server を起こし直し、Lite を読んでから差し替えの手順を 1 回行う。
fn session(setup: &CaptureSetup, label: &str, lite: &LivePatch, full: &LivePatch) -> Session {
    let program = program();
    let log_start = logged_line_count();
    let mut timings = None;
    let segments = setup.record_session(label, lite, CAPTURE_SECONDS, |sender| {
        let (sent, measured) = swap(sender, lite, full, &program);
        timings = Some(measured);
        sent
    });
    let (lite_send, loading_during_lite, preload_elapsed, full_send) =
        timings.expect("手順が走っていない");
    let standby_finish = logged_lines_since(log_start)
        .into_iter()
        .filter(|line| line.contains(STANDBY_FINISH) && line.contains("event=finish"))
        .collect();
    Session {
        segments,
        lite_send,
        loading_during_lite,
        preload_elapsed,
        full_send,
        standby_finish,
    }
}

/// Lite の行を鳴らしてから Full を先読みに出し、読み込み中に Lite の行、完了後に Full の行を鳴らす。
/// 返すのは送った 3 行と、(読み込み中の Lite の送信, その時点で読み込み中か, 先読みの長さ, Full の送信)。
fn swap(
    sender: &MmlOverlaySender,
    lite: &LivePatch,
    full: &LivePatch,
    program: &LineProgram,
) -> (Vec<Sent>, (Duration, bool, Duration, Duration)) {
    let step = Duration::from_millis(STEP_MS);
    let first_line = send_line(sender, lite, program);
    let preload_started = Instant::now();
    sender.preload(full.clone());
    wait_for_preload(sender, |state| state.is_some(), "先読みが受け付けられない");

    // warm の Full は数秒で読み終わるので、読み込み中に届くよう 1 行目の余韻を待たずに送る。
    sleep_until(first_line.started + LOADING_LINE_GAP);
    let lite_line = send_line(sender, lite, program);
    let loading_during_lite = matches!(
        sender.status().preload(),
        Some(MmlOverlayPreload::Loading(_))
    );
    wait_for_preload(
        sender,
        |state| match state {
            Some(MmlOverlayPreload::Ready(patch)) => patch == full,
            Some(MmlOverlayPreload::Loading(_)) => false,
            None => panic!("Full の先読みが失敗した"),
        },
        "Full の先読みが終わらない",
    );
    let preload_elapsed = preload_started.elapsed();

    // Lite の行と余韻が録音で重ならないよう、1 行ぶんは空ける。
    sleep_until(lite_line.started + step);
    let full_line = send_line(sender, full, program);
    sleep_until(full_line.started + step);
    let timings = (
        lite_line.started - lite_line.requested,
        loading_during_lite,
        preload_elapsed,
        full_line.started - full_line.requested,
    );
    (vec![first_line, lite_line, full_line], timings)
}

fn wait_for_preload(
    sender: &MmlOverlaySender,
    mut done: impl FnMut(Option<&MmlOverlayPreload>) -> bool,
    what: &str,
) {
    let deadline = Instant::now() + PRELOAD_TIMEOUT;
    while !done(sender.status().preload()) {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// 画面が鳴らすのと同じ、ルール無しの Articulated を 1 回だけ。
fn program() -> LineProgram {
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let events = convert(&raw, &RuleTable::default());
    let loop_seconds = events.last().map_or(0.0, |event| event.seconds);
    LineProgram::once(LinePerformance {
        events,
        loop_seconds,
    })
}

fn assert_standby_finished_cleanly(lines: &[String]) {
    assert!(!lines.is_empty(), "server が先読みの終わりを出していない");
    for line in lines {
        assert!(line.contains("result=ok"), "先読みが失敗した: {line}");
        assert!(
            line.contains("underrun_frames=0 "),
            "先読みの間に演奏 bank が underrun した: {line}"
        );
    }
}

/// 音ごとに、ずれを探した最大の正規化相関を出して返す。
fn compare(label: &str, left: &Segment, right: &Segment) -> Vec<f64> {
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let notes = notes_from_events(&raw);
    notes
        .iter()
        .map(|note| {
            let seconds = note.off_seconds - note.on_seconds;
            let ncc = left.correlation(right, note.on_seconds, seconds);
            eprintln!("lite-full-swap: {label} pitch={} ncc={ncc:.4}", note.pitch);
            ncc
        })
        .collect()
}
