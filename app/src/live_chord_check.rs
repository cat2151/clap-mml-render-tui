//! Chord Chart の和音送りを画面なしで再現する診断コマンド。
//!
//! 初回は画面へ入ったときと同じく行全体を送り、以後は右カーソルと
//! 同じく 2 番目以降の chord を 1 つずつ送る。送信は
//! [`cmrt_mml_overlay::MmlOverlaySender`] に任せ、本番と同じ stop / live timeline
//! 経路を通す。server の live capture を WAV に落とし、各 chord の中央区間を
//! peak / RMS で測る。

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result};
use cmrt_mml_overlay::{
    line_play::{chord_chart_line_events, LineProgram, LineStatus},
    MmlOverlaySender,
};
use cmrt_realtime_play::RealtimePlayServerSupervisor;
use cmrt_runtime::Config;

const CAPTURE_PATH_ENV: &str = "CMRT_LIVE_CAPTURE_WAV";
const CAPTURE_SECONDS_ENV: &str = "CMRT_LIVE_CAPTURE_SECONDS";
const CAPTURE_MIN_SECONDS_ENV: &str = "CMRT_LIVE_CAPTURE_MIN_SECONDS";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
const SILENCE_PEAK: f32 = 1.0e-6;

#[derive(Debug, PartialEq, Eq)]
pub struct LiveChordCheckRequest {
    pub config: Option<PathBuf>,
    pub patch: String,
    pub key: String,
    pub degrees: String,
    pub step_ms: u64,
    pub out: Option<PathBuf>,
    pub verify: bool,
}

pub fn run(cfg: &Config, request: &LiveChordCheckRequest) -> Result<()> {
    validate_request(request)?;
    let chords = split_chords(&request.degrees)?;
    let programs = chord_programs(&request.key, &request.degrees, &chords)?;
    let capture_path = capture_path(request.out.as_deref())?;
    let capture_seconds = capture_seconds(chords.len(), request.step_ms);
    let minimum_capture_seconds = expected_capture_seconds(chords.len(), request.step_ms) * 0.9;
    let _capture_env =
        CaptureEnvironment::set(&capture_path, capture_seconds, minimum_capture_seconds);

    println!("[live-chord-check]");
    println!("  patch         : {}", request.patch);
    println!("  key           : {}", request.key);
    println!("  degrees       : {}", request.degrees);
    println!("  step_ms       : {}", request.step_ms);
    println!("  capture       : {}", capture_path.display());
    println!();

    let supervisor = Arc::new(RealtimePlayServerSupervisor::with_live_instance_count(
        cfg, 2,
    ));
    supervisor
        .start_owned_for_fast_midi()
        .context("realtime play server を診断用に起動できません")?;
    let sender = MmlOverlaySender::new(Arc::clone(&supervisor), cfg.sample_rate);

    for (index, ((label, program), chord)) in programs.into_iter().zip(&chords).enumerate() {
        let command_id = sender.play_line(Some(request.patch.as_str()), program);
        wait_for_line(&sender, command_id)?;
        println!(
            "  send chord={}/{} label='{}' source='{}' command_id={command_id}",
            index + 1,
            chords.len(),
            label,
            chord
        );
        std::thread::sleep(Duration::from_millis(request.step_ms));
    }

    let stop_id = sender.stop();
    wait_for_command(&sender, stop_id)?;
    drop(sender);
    wait_for_capture(&capture_path)?;

    let capture = read_capture(&capture_path)?;
    let duration_ms = capture.samples.len() as u64 * 1000
        / u64::from(capture.sample_rate)
        / u64::from(capture.channels);
    let expected_ms = request.step_ms.saturating_mul(chords.len() as u64);
    let capture_too_short = duration_ms < expected_ms.saturating_mul(3) / 4;
    let results = (!capture_too_short).then(|| {
        analyze_segments(
            &capture.samples,
            capture.channels,
            capture.sample_rate,
            request.step_ms,
            &chords,
        )
    });
    println!();
    println!(
        "  captured      : frames={} duration_ms={} sample_rate={} channels={}",
        capture.samples.len() / usize::from(capture.channels),
        duration_ms,
        capture.sample_rate,
        capture.channels
    );
    match &results {
        Some(results) => {
            for result in results {
                println!(
                    "  result chord={}/{} source='{}' peak={:.6} rms={:.6} silent={}",
                    result.index + 1,
                    results.len(),
                    result.chord,
                    result.peak,
                    result.rms,
                    if result.silent { "yes" } else { "no" }
                );
            }
        }
        None => println!("  result        : unavailable (capture が短すぎます)"),
    }

    let silent = results
        .as_ref()
        .map(|results| results.iter().filter(|result| result.silent).count());
    println!();
    println!("[まとめ]");
    println!("  和音数         : {}", chords.len());
    println!(
        "  無音           : {}",
        silent.map_or_else(|| "判定不可".to_string(), |count| format!("{count} 件"))
    );
    println!(
        "  capture 長    : {}",
        if capture_too_short {
            "short (stop-all で途中終了した可能性あり)"
        } else {
            "ok"
        }
    );

    drop(supervisor);
    if request.verify && (silent.is_some_and(|count| count > 0) || capture_too_short) {
        anyhow::bail!(
            "live-chord-check verify 失敗: silent={} capture_too_short={capture_too_short}",
            silent.map_or_else(|| "unknown".to_string(), |count| count.to_string())
        );
    }
    Ok(())
}

fn validate_request(request: &LiveChordCheckRequest) -> Result<()> {
    if request.patch.trim().is_empty() {
        anyhow::bail!("--patch は空にできません");
    }
    if !(200..=10_000).contains(&request.step_ms) {
        anyhow::bail!("--step-ms は 200..=10000 で指定してください");
    }
    Ok(())
}

fn split_chords(degrees: &str) -> Result<Vec<String>> {
    let chords = cmrt_chord::chord_source_ranges(degrees)
        .into_iter()
        .map(|range| {
            degrees
                .get(range)
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("chord の文字範囲が不正です"))
        })
        .collect::<Result<Vec<_>>>()?;
    if chords.is_empty() {
        anyhow::bail!("Chord Chart degrees を chord に分割できません: {degrees:?}");
    }
    Ok(chords)
}

fn chord_programs(
    key: &str,
    degrees: &str,
    chords: &[String],
) -> Result<Vec<(String, LineProgram)>> {
    let key = (!key.trim().is_empty()).then_some(key);
    chords
        .iter()
        .enumerate()
        .map(|(index, chord)| {
            let source = if index == 0 { degrees } else { chord };
            let (status, performance) = chord_chart_line_events(source, key);
            match status {
                LineStatus::Played { .. } => {
                    Ok((source.to_string(), LineProgram::once(performance)))
                }
                LineStatus::Idle => anyhow::bail!("chord {source:?} が空です"),
                LineStatus::Error(error) => {
                    anyhow::bail!("chord {source:?} を演奏データにできません: {error}")
                }
            }
        })
        .collect()
}

pub(crate) fn wait_for_line(sender: &MmlOverlaySender, command_id: u64) -> Result<()> {
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    loop {
        let status = sender.status();
        if let Some(error) = status.prepare_error() {
            anyhow::bail!("patch の準備に失敗しました: {error}");
        }
        if status
            .line_playback()
            .is_some_and(|playback| playback.command_id() == command_id)
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("command {command_id} の timeline 送信がタイムアウトしました");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn wait_for_command(sender: &MmlOverlaySender, command_id: u64) -> Result<()> {
    let deadline = Instant::now() + COMMAND_TIMEOUT;
    while sender.status().command_id() != command_id {
        if Instant::now() >= deadline {
            anyhow::bail!("command {command_id} の開始がタイムアウトしました");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn capture_seconds(chord_count: usize, step_ms: u64) -> f64 {
    expected_capture_seconds(chord_count, step_ms) + 5.0
}

fn expected_capture_seconds(chord_count: usize, step_ms: u64) -> f64 {
    chord_count as f64 * step_ms as f64 / 1000.0
}

fn capture_path(requested: Option<&Path>) -> Result<PathBuf> {
    let path = match requested {
        Some(path) => path.to_path_buf(),
        None => {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            std::env::temp_dir().join(format!(
                "cmrt-live-chord-check-{}-{nonce}.wav",
                std::process::id()
            ))
        }
    };
    if path.exists() {
        anyhow::bail!("capture 先は既に存在します: {}", path.display());
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("capture 先の親ディレクトリがありません"))?;
    if !parent.is_dir() {
        anyhow::bail!(
            "capture 先の親ディレクトリがありません: {}",
            parent.display()
        );
    }
    Ok(path)
}

struct CaptureEnvironment {
    path: Option<std::ffi::OsString>,
    seconds: Option<std::ffi::OsString>,
    minimum_seconds: Option<std::ffi::OsString>,
}

impl CaptureEnvironment {
    fn set(path: &Path, seconds: f64, minimum_seconds: f64) -> Self {
        let previous = Self {
            path: std::env::var_os(CAPTURE_PATH_ENV),
            seconds: std::env::var_os(CAPTURE_SECONDS_ENV),
            minimum_seconds: std::env::var_os(CAPTURE_MIN_SECONDS_ENV),
        };
        std::env::set_var(CAPTURE_PATH_ENV, path);
        std::env::set_var(CAPTURE_SECONDS_ENV, format!("{seconds:.3}"));
        std::env::set_var(CAPTURE_MIN_SECONDS_ENV, format!("{minimum_seconds:.3}"));
        previous
    }
}

impl Drop for CaptureEnvironment {
    fn drop(&mut self) {
        restore_env(CAPTURE_PATH_ENV, self.path.take());
        restore_env(CAPTURE_SECONDS_ENV, self.seconds.take());
        restore_env(CAPTURE_MIN_SECONDS_ENV, self.minimum_seconds.take());
    }
}

pub(crate) fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

fn wait_for_capture(path: &Path) -> Result<()> {
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    let mut previous_len = None;
    let mut stable = 0;
    loop {
        if let Ok(metadata) = std::fs::metadata(path) {
            let len = metadata.len();
            if len > 44 && previous_len == Some(len) {
                stable += 1;
                if stable >= 2 {
                    return Ok(());
                }
            } else {
                stable = 0;
                previous_len = Some(len);
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "live capture WAV が書き出されませんでした: {}",
                path.display()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub(crate) struct Capture {
    pub(crate) samples: Vec<f32>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

pub(crate) fn read_capture(path: &Path) -> Result<Capture> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("live capture WAV を読めません: {}", path.display()))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Float || spec.bits_per_sample != 32 {
        anyhow::bail!("live capture WAV が 32-bit float ではありません: {spec:?}");
    }
    let samples = reader
        .samples::<f32>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Capture {
        samples,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

struct SegmentResult<'a> {
    index: usize,
    chord: &'a str,
    peak: f32,
    rms: f64,
    silent: bool,
}

fn analyze_segments<'a>(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    step_ms: u64,
    chords: &'a [String],
) -> Vec<SegmentResult<'a>> {
    let channels = usize::from(channels).max(1);
    let frames = samples.len() / channels;
    let segment_frames = (u64::from(sample_rate).saturating_mul(step_ms) / 1000) as usize;
    chords
        .iter()
        .enumerate()
        .map(|(index, chord)| {
            let segment_start = (index * segment_frames).min(frames);
            let segment_end = ((index + 1) * segment_frames).min(frames);
            // generation 切替の前後は output ring の先読みが数 block 混ざりうる。
            // その影響を外すため、指定stepで区切った各区間の中央 50% だけを測る。
            let quarter = segment_end.saturating_sub(segment_start) / 4;
            let start = (segment_start + quarter) * channels;
            let end = (segment_end - quarter) * channels;
            let window = samples.get(start..end).unwrap_or_default();
            let peak = window
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            let rms = if window.is_empty() {
                0.0
            } else {
                let power: f64 = window
                    .iter()
                    .map(|sample| f64::from(*sample) * f64::from(*sample))
                    .sum();
                (power / window.len() as f64).sqrt()
            };
            SegmentResult {
                index,
                chord,
                peak,
                rms,
                silent: peak < SILENCE_PEAK,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
