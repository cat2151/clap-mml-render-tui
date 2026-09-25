//! 実サーバーの live instance を fadeout すると、鳴っている音も effect chain の余韻も
//! 指定の長さで消え、次の音は等倍で鳴ること。
//!
//! reverb 1 段の chain を付けた instance で note を押したまま、途中で 50 ms の fadeout を
//! 送る。device へ出た波形（`CMRT_OUTPUT_CAPTURE_WAV`）で、fadeout の前・後・次の打鍵の
//! RMS を比べる。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-realtime-play -- --ignored fade_out --nocapture
//! ```

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use super::harness::{pick_port, TestPlayServer, PLAY_SERVER_EXE_ENV};

const INSTANCE_COUNT: usize = 2;
const CAPTURE_PATH_ENV: &str = "CMRT_OUTPUT_CAPTURE_WAV";
const CAPTURE_SECONDS_ENV: &str = "CMRT_OUTPUT_CAPTURE_SECONDS";
const PATCH: &str = "patches_factory/Templates/Init Sine.fxp";
const CHAIN: &str = r#"[{"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx"}]"#;
const FADE_MS: u32 = 50;
/// 打鍵から fadeout を送るまで。
const FADE_AT: Duration = Duration::from_millis(700);
/// 打鍵から次の行の準備まで。絞った行の余韻が残っていれば聞こえる長さを空ける。
const RESTRIKE_AT: Duration = Duration::from_millis(1_400);
const CAPTURE_SECONDS: f64 = 2.6;
/// fadeout の後の残りが、前より小さいとみなす差。
const FADED_MARGIN_DB: f64 = 40.0;
/// 次の打鍵が、fadeout の前と同じ水準で鳴っているとみなす差。
const RESTRIKE_TOLERANCE_DB: f64 = 6.0;

#[test]
#[ignore = "実機の play server 実行ファイルと Surge XT Effects が要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_fade_out_silences_the_instance_and_the_next_note_sounds() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let capture = capture_path();
    let port = pick_port(54_000);
    let server = TestPlayServer::spawn_with_env(
        &exe,
        port,
        INSTANCE_COUNT,
        &[
            (CAPTURE_PATH_ENV, capture.display().to_string()),
            (CAPTURE_SECONDS_ENV, CAPTURE_SECONDS.to_string()),
        ],
    );
    let cfg = crate::tests::cfg_for_port(port);
    let supervisor =
        crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, INSTANCE_COUNT);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");
    // 録音は最初の準備で再生が始まった frame から始まるので、準備の依頼時刻を原点にする。
    let origin = Instant::now();
    supervisor
        .prepare_live_patch_with_effect_chain(0, Some(PATCH), CHAIN)
        .expect("chain 付きの準備が失敗した");
    let struck = origin.elapsed();
    supervisor
        .send_midi(0, &[[0x90, 60, 100]])
        .expect("note on が失敗した");
    sleep_until(origin + struck + FADE_AT);
    let fade_sent = origin.elapsed();
    supervisor
        .fade_out_live_instances(&[0], FADE_MS)
        .expect("fadeout が失敗した");
    sleep_until(origin + struck + RESTRIKE_AT);
    // 次の行を始める（音色の準備）。絞った instance は、この後に届いたイベントから鳴る。
    supervisor
        .prepare_live_patch_with_effect_chain(0, Some(PATCH), CHAIN)
        .expect("次の行の準備が失敗した");
    let restruck = origin.elapsed();
    supervisor
        .send_midi(0, &[[0x90, 67, 100]])
        .expect("次の打鍵が失敗した");
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-output-capture: event=written"));
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-live: event=apply-fade-out"));

    let (samples, sample_rate) = read_capture(&capture);
    let _ = std::fs::remove_file(&capture);
    let ms = Duration::from_millis;
    let before = rms_db(&samples, sample_rate, fade_sent - ms(300), fade_sent);
    let after = rms_db(
        &samples,
        sample_rate,
        fade_sent + ms(u64::from(FADE_MS) + 100),
        restruck,
    );
    let next = rms_db(
        &samples,
        sample_rate,
        restruck + ms(150),
        restruck + ms(450),
    );
    eprintln!(
        "fade-out: before_db={before:.1} after_db={after:.1} next_db={next:.1} \
         struck_ms={} fade_sent_ms={} restruck_ms={}",
        struck.as_millis(),
        fade_sent.as_millis(),
        restruck.as_millis()
    );
    assert!(
        after < before - FADED_MARGIN_DB,
        "fadeout の後も音が残る: before={before:.1} dB after={after:.1} dB"
    );
    assert!(
        (next - before).abs() < RESTRIKE_TOLERANCE_DB,
        "fadeout の後の打鍵が等倍で鳴らない: before={before:.1} dB next={next:.1} dB"
    );
}

fn sleep_until(deadline: Instant) {
    std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
}

fn capture_path() -> PathBuf {
    std::env::temp_dir().join(format!("cmrt-fade-out-test-{}.wav", std::process::id()))
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

/// `from`〜`to`（録音の先頭から）の RMS（dBFS）。
fn rms_db(samples: &[f32], sample_rate: u32, from: Duration, to: Duration) -> f64 {
    let frame = |at: Duration| (at.as_secs_f64() * f64::from(sample_rate)) as usize * 2;
    let window = &samples[frame(from).min(samples.len())..frame(to).min(samples.len())];
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
