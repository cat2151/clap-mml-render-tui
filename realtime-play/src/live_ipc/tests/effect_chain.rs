//! 音色の準備に同梱した effect chain が、実サーバーの live instance の出力に掛かること。
//!
//! 同じ instance・同じ音色・同じ打鍵を、chain 無しと chain 付き（Surge XT Effects の
//! reverb 1 段）で 1 回ずつ鳴らし、device へ出た波形（`CMRT_OUTPUT_CAPTURE_WAV`）の
//! note off 後の余韻を比べる。reverb が掛かっていれば余韻が大きく残る。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-realtime-play -- --ignored effect_chain --nocapture
//! ```

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use super::harness::{pick_port, TestPlayServer, PLAY_SERVER_EXE_ENV};

const INSTANCE_COUNT: usize = 2;
const CAPTURE_PATH_ENV: &str = "CMRT_OUTPUT_CAPTURE_WAV";
const CAPTURE_SECONDS_ENV: &str = "CMRT_OUTPUT_CAPTURE_SECONDS";
const CHAIN: &str = r#"[{"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx"}]"#;
/// 1 回の打鍵に使う長さ。前半 `NOTE` で鳴らし、残りは余韻。
const NOTE: Duration = Duration::from_millis(300);
const TURN: Duration = Duration::from_millis(1_500);
/// 余韻を測る窓（各回の頭から）。note off の 200 ms 後から次の回の手前まで。
const TAIL_FROM: Duration = Duration::from_millis(500);
const TAIL_TO: Duration = Duration::from_millis(1_400);
/// chain 付きの余韻が dry より大きいとみなす差。
const WET_TAIL_MARGIN_DB: f64 = 10.0;

#[test]
#[ignore = "実機の play server 実行ファイルと Surge XT Effects が要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_bundled_effect_chain_changes_the_instance_output() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let capture = capture_path();
    let port = pick_port(53_000);
    let server = TestPlayServer::spawn_with_env(
        &exe,
        port,
        INSTANCE_COUNT,
        &[
            (CAPTURE_PATH_ENV, capture.display().to_string()),
            (CAPTURE_SECONDS_ENV, (2.5 * TURN.as_secs_f64()).to_string()),
        ],
    );
    let cfg = crate::tests::cfg_for_port(port);
    let supervisor =
        crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, INSTANCE_COUNT);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");

    supervisor
        .prepare_live_patch(0, None)
        .expect("chain 無しの準備が失敗した");
    // 録音は最初に音が出た frame から始まるので、1 回目の打鍵の時刻を原点にする。
    let origin = Instant::now();
    strike(&supervisor);
    sleep_until(origin + TURN);

    supervisor
        .prepare_live_patch_with_effect_chain(0, None, CHAIN)
        .expect("chain 付きの準備が失敗した");
    let chain_line = server.wait_for_stderr_line(|line| {
        line.starts_with("cmrt-bank-chain: bank=0 local=0 event=rebuilt")
    });
    assert!(
        chain_line.contains("patch_skipped=true"),
        "音色が同じで chain だけ変わったのに音色を読み直した: {chain_line}"
    );
    let wet_offset = origin.elapsed();
    strike(&supervisor);
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-output-capture: event=written"));

    let (samples, sample_rate) = read_capture(&capture);
    let _ = std::fs::remove_file(&capture);
    let dry_tail = tail_rms_db(&samples, sample_rate, Duration::ZERO);
    let wet_tail = tail_rms_db(&samples, sample_rate, wet_offset);
    eprintln!(
        "effect-chain: dry_tail_db={dry_tail:.1} wet_tail_db={wet_tail:.1} wet_offset_ms={}",
        wet_offset.as_millis()
    );
    assert!(
        wet_tail > dry_tail + WET_TAIL_MARGIN_DB,
        "chain 付きの余韻が dry と変わらない: dry={dry_tail:.1} dB wet={wet_tail:.1} dB"
    );
}

fn strike(supervisor: &crate::RealtimePlayServerSupervisor) {
    supervisor
        .send_midi(0, &[[0x90, 60, 100]])
        .expect("note on が失敗した");
    std::thread::sleep(NOTE);
    supervisor
        .send_midi(0, &[[0x80, 60, 0]])
        .expect("note off が失敗した");
}

fn sleep_until(deadline: Instant) {
    std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
}

fn capture_path() -> PathBuf {
    std::env::temp_dir().join(format!("cmrt-effect-chain-test-{}.wav", std::process::id()))
}

fn read_capture(path: &PathBuf) -> (Vec<f32>, u32) {
    let mut reader = hound::WavReader::open(path)
        .unwrap_or_else(|error| panic!("録音を読めない ({}): {error}", path.display()));
    let sample_rate = reader.spec().sample_rate;
    let samples = reader
        .samples::<f32>()
        .collect::<Result<Vec<_>, _>>()
        .expect("録音の sample を読めない");
    (samples, sample_rate)
}

/// `offset` から始まる回の、余韻の窓の RMS（dBFS）。
fn tail_rms_db(samples: &[f32], sample_rate: u32, offset: Duration) -> f64 {
    let frame = |at: Duration| (at.as_secs_f64() * f64::from(sample_rate)) as usize * 2;
    let window = &samples
        [frame(offset + TAIL_FROM).min(samples.len())..frame(offset + TAIL_TO).min(samples.len())];
    assert!(
        !window.is_empty(),
        "録音が短すぎる: {} sample",
        samples.len()
    );
    let mean_square = window
        .iter()
        .map(|&sample| f64::from(sample).powi(2))
        .sum::<f64>()
        / window.len() as f64;
    10.0 * mean_square.max(1e-20).log10()
}
