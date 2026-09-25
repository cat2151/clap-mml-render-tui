//! MML の行を LIVE で順に鳴らし、device へ出た波形を録って数える診断コマンド。
//!
//! 送信は [`cmrt_mml_overlay::MmlOverlaySender::play_line`] に任せ、本番と同じ経路を通す。
//! 行から「音色 + effect chain + 演奏」への変換も、画面の試聴と同じ
//! [`cmrt_mml_overlay::live_line`] を使う。
//! server は `CMRT_OUTPUT_CAPTURE_WAV` 付きで起動し、device の callback が出した frame
//! （underrun の 0 や世代の切り捨ても含む）を録らせる。録れた WAV からクリック候補・
//! 音の途中の無音・各行の頭を数える。`--offline` を付けると、同じ行を offline render
//! して同じ解析に掛けた対照も出す。
//!
//! `--fade-previous-ms` は 2 行目以降を送る直前に前の行を fadeout する（EFFECT CHAIN の試聴の
//! 移動と同じ送り方）。`--residual-reference` に 1 行目だけの録音を渡すと、2 行目の送信の後に
//! 残る 1 行目の音をそれと比べる（[`residual`]）。

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result};
use cmrt_mml_overlay::{live_line, LiveLine, MmlOverlaySender};
use cmrt_offline_render::OfflineRenderer;
use cmrt_realtime_play::RealtimePlayServerSupervisor;
use cmrt_runtime::Config;

use crate::live_chord_check::{read_capture, restore_env, wait_for_command, wait_for_line};

mod alignment;
mod analysis;
mod report;
mod residual;

const CAPTURE_PATH_ENV: &str = "CMRT_OUTPUT_CAPTURE_WAV";
const CAPTURE_SECONDS_ENV: &str = "CMRT_OUTPUT_CAPTURE_SECONDS";
/// 最後の行を送ってから録り続ける長さ。余韻と停止の後まで入れる。
const TAIL_SECONDS: f64 = 1.5;
/// 録り終えてから WAV が書き出されるまで待つ上限。
const WRITE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, PartialEq, Eq)]
pub struct LiveLineCheckRequest {
    pub config: Option<PathBuf>,
    /// 先頭 JSON 込みの MML の行。この順に送る。
    pub lines: Vec<String>,
    pub step_ms: u64,
    pub out: Option<PathBuf>,
    pub offline: bool,
    /// 2 行目以降を送る直前に、前の行をこの長さで fadeout する。
    pub fade_previous_ms: Option<u32>,
    pub residual: Option<ResidualRequest>,
}

/// 2 行目の送信の後に残る 1 行目の音の測り方。
#[derive(Debug, PartialEq, Eq)]
pub struct ResidualRequest {
    /// 1 行目だけを鳴らした録音（同じコマンドで行を 1 つだけ渡して録ったもの）。
    pub reference: PathBuf,
    /// 除く 2 行目の音の MIDI note number。
    pub notch_note: u8,
}

/// 送る 1 行。
struct CheckLine {
    source: String,
    live: LiveLine,
}

/// 1 行を送った時刻。
struct Sent {
    requested: Instant,
    started: Instant,
}

pub fn run(cfg: &Config, request: &LiveLineCheckRequest) -> Result<()> {
    if !(100..=10_000).contains(&request.step_ms) {
        anyhow::bail!("--step-ms は 100..=10000 で指定してください");
    }
    let lines = check_lines(&request.lines)?;
    let capture_path = capture_path(request.out.as_deref())?;
    let capture_seconds = lines.len() as f64 * request.step_ms as f64 / 1000.0 + TAIL_SECONDS;

    println!("[live-line-check]");
    for (index, line) in lines.iter().enumerate() {
        println!("  line {}       : {}", index + 1, line.source);
    }
    println!("  step_ms       : {}", request.step_ms);
    if let Some(fade_ms) = request.fade_previous_ms {
        println!("  fade_previous : {fade_ms} ms");
    }
    println!("  capture       : {}", capture_path.display());
    println!();

    let sent = {
        let _env = CaptureEnvironment::set(&capture_path, capture_seconds);
        send_lines(
            cfg,
            &lines,
            request.step_ms,
            request.fade_previous_ms,
            &capture_path,
            capture_seconds,
        )?
    };

    let capture = read_capture(&capture_path)?;
    if capture.channels != 2 {
        anyhow::bail!(
            "output capture が stereo ではありません: {}",
            capture.channels
        );
    }
    // server は最初の準備で再生を始め、録音はそこから始まる。最初の準備は 1 行目の送信の中で
    // 走るので、1 行目の送信を依頼した時刻を原点にする。
    let origin = sent[0].requested;
    let live_sends = sent
        .iter()
        .map(|sent| {
            duration_frames(
                sent.requested.saturating_duration_since(origin),
                capture.sample_rate,
            )
        })
        .collect::<Vec<_>>();
    let prepare_ms = sent
        .iter()
        .map(|sent| sent.started.duration_since(sent.requested).as_secs_f64() * 1000.0)
        .collect::<Vec<_>>();
    report::print(
        "live",
        &capture.samples,
        capture.sample_rate,
        &live_sends,
        Some(&prepare_ms),
    );
    if let Some(residual) = &request.residual {
        residual::report(residual, &capture, &live_sends, request.step_ms)?;
    }

    if request.offline {
        let offline_path = capture_path.with_extension("offline.wav");
        let sample_rate = cfg.sample_rate as u32;
        let renders = offline_renders(cfg, &lines)?;
        let sends = (0..lines.len())
            .map(|index| {
                duration_frames(
                    Duration::from_millis(request.step_ms) * index as u32,
                    sample_rate,
                )
            })
            .collect::<Vec<_>>();
        let samples = mix_renders(&renders, &sends);
        cmrt_core::write_wav(&samples, sample_rate, &offline_path)?;
        println!();
        println!("  offline wav   : {}", offline_path.display());
        report::print("offline", &samples, sample_rate, &sends, None);

        if capture.sample_rate == sample_rate {
            // 頭の比較は、live で各行の timeline が始まった点（準備の後）から測る。offline は
            // live で各行が鳴り始めた点へ置き直し、前の行の余韻の重なり方を live と揃える。
            let live_starts = sent
                .iter()
                .map(|sent| {
                    duration_frames(sent.started.saturating_duration_since(origin), sample_rate)
                })
                .collect::<Vec<_>>();
            let heads = |label: &str, live: &[f32], renders: &[Vec<f32>]| {
                print_heads(
                    label,
                    live,
                    renders,
                    &live_starts,
                    sample_rate,
                    request.step_ms,
                    cfg.buffer_size,
                )
            };
            heads("head", &capture.samples, &renders);
            if let Some(residual) = &request.residual {
                // 前の行の音が残ると次の行の頭を検出できないので、次の行の音の帯域だけでも比べる。
                let hz = residual::note_hz(residual.notch_note);
                let band = |samples: &[f32]| residual::band_stereo(samples, sample_rate, hz);
                heads(
                    "head-band",
                    &band(&capture.samples),
                    &renders
                        .iter()
                        .map(|render| band(render))
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    Ok(())
}

/// 各行の頭を live と offline で比べて `[label]` 節に出す。offline は各行の render を、live で
/// その行が鳴り始めた点へ置いて足し合わせる。
fn print_heads(
    label: &str,
    live: &[f32],
    renders: &[Vec<f32>],
    live_starts: &[usize],
    sample_rate: u32,
    step_ms: u64,
    block_frames: usize,
) {
    let live_heads = analysis::line_heads(live, sample_rate, live_starts);
    let aligned_sends = live_starts
        .iter()
        .zip(&live_heads)
        .map(|(start, head)| start + head.unwrap_or(0))
        .collect::<Vec<_>>();
    report::print_head_alignment(
        label,
        report::Track {
            samples: live,
            sends: live_starts,
        },
        report::Track {
            samples: &mix_renders(renders, &aligned_sends),
            sends: &aligned_sends,
        },
        sample_rate,
        step_ms,
        block_frames,
    );
}

fn check_lines(sources: &[String]) -> Result<Vec<CheckLine>> {
    if sources.is_empty() {
        anyhow::bail!("MML の行を 1 つ以上指定してください");
    }
    sources
        .iter()
        .map(|source| {
            Ok(CheckLine {
                source: source.clone(),
                live: live_line(source).map_err(anyhow::Error::msg)?,
            })
        })
        .collect()
}

/// server を録音タップ付きで起動し、行を `step_ms` おきに送って WAV が書き出されるまで待つ。
fn send_lines(
    cfg: &Config,
    lines: &[CheckLine],
    step_ms: u64,
    fade_previous_ms: Option<u32>,
    capture_path: &Path,
    capture_seconds: f64,
) -> Result<Vec<Sent>> {
    let supervisor = Arc::new(RealtimePlayServerSupervisor::with_live_instance_count(
        cfg, 2,
    ));
    supervisor
        .start_owned_for_fast_midi()
        .context("realtime play server を診断用に起動できません")?;
    let sender = MmlOverlaySender::new(Arc::clone(&supervisor), cfg.sample_rate);

    let step = Duration::from_millis(step_ms);
    let first = Instant::now();
    let mut sent = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        sleep_until(first + step * index as u32);
        let requested = Instant::now();
        if let Some(fade_ms) = fade_previous_ms.filter(|_| index > 0) {
            let faded = sender
                .fade_out_line(fade_ms)
                .map_err(anyhow::Error::msg)
                .context("前の行の fadeout を送れません")?;
            println!("  fade line={index} fade_ms={fade_ms} sent={faded}");
        }
        let command_id = sender.play_line(line.live.patch.clone(), line.live.program.clone());
        wait_for_line(&sender, command_id)?;
        let started = Instant::now();
        println!(
            "  send line={}/{} command_id={command_id} prepare_ms={:.1}",
            index + 1,
            lines.len(),
            started.duration_since(requested).as_secs_f64() * 1000.0
        );
        sent.push(Sent { requested, started });
    }
    sleep_until(first + step * lines.len() as u32);
    let stop_id = sender.stop();
    wait_for_command(&sender, stop_id)?;
    drop(sender);

    // 録音は最初の行が鳴り始めた frame から `capture_seconds` 秒ぶん。満ちたら書き出される。
    let deadline = sent[0].started + Duration::from_secs_f64(capture_seconds) + WRITE_TIMEOUT;
    wait_for_written(capture_path, deadline)?;
    drop(supervisor);
    Ok(sent)
}

fn sleep_until(deadline: Instant) {
    let now = Instant::now();
    if deadline > now {
        std::thread::sleep(deadline - now);
    }
}

fn duration_frames(duration: Duration, sample_rate: u32) -> usize {
    (duration.as_secs_f64() * f64::from(sample_rate)).round() as usize
}

/// 各行を offline render する（インターリーブステレオ）。
fn offline_renders(cfg: &Config, lines: &[CheckLine]) -> Result<Vec<Vec<f32>>> {
    let renderer = OfflineRenderer::new(Arc::new(cfg.clone()));
    lines
        .iter()
        .map(|line| {
            renderer
                .render_phrase(&line.source)
                .map(|rendered| rendered.samples)
                .with_context(|| format!("offline render に失敗しました: {}", line.source))
        })
        .collect()
}

/// 各行の render を `offsets` の frame に置いて足し合わせる（前の行を切らない）。
fn mix_renders(renders: &[Vec<f32>], offsets: &[usize]) -> Vec<f32> {
    let mut mixed = Vec::new();
    for (samples, &offset) in renders.iter().zip(offsets) {
        mix_at(&mut mixed, samples, offset);
    }
    mixed
}

/// インターリーブステレオの `samples` を `offset` frame 目から `mixed` へ足す。
fn mix_at(mixed: &mut Vec<f32>, samples: &[f32], offset: usize) {
    let start = offset * 2;
    if mixed.len() < start + samples.len() {
        mixed.resize(start + samples.len(), 0.0);
    }
    for (target, sample) in mixed[start..].iter_mut().zip(samples) {
        *target += sample;
    }
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
                "cmrt-live-line-check-{}-{nonce}.wav",
                std::process::id()
            ))
        }
    };
    if path.exists() {
        anyhow::bail!("capture 先は既に存在します: {}", path.display());
    }
    Ok(path)
}

/// WAV が書き出され、大きさが落ち着くまで待つ。
fn wait_for_written(path: &Path, deadline: Instant) -> Result<()> {
    let mut previous_len = None;
    loop {
        if let Ok(metadata) = std::fs::metadata(path) {
            let len = metadata.len();
            if len > 44 && previous_len == Some(len) {
                return Ok(());
            }
            previous_len = Some(len);
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "output capture WAV が書き出されませんでした: {}",
                path.display()
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// server へ引き継がせる環境変数。抜けるときに元へ戻す。
struct CaptureEnvironment {
    path: Option<std::ffi::OsString>,
    seconds: Option<std::ffi::OsString>,
}

impl CaptureEnvironment {
    fn set(path: &Path, seconds: f64) -> Self {
        let previous = Self {
            path: std::env::var_os(CAPTURE_PATH_ENV),
            seconds: std::env::var_os(CAPTURE_SECONDS_ENV),
        };
        std::env::set_var(CAPTURE_PATH_ENV, path);
        std::env::set_var(CAPTURE_SECONDS_ENV, format!("{seconds:.3}"));
        previous
    }
}

impl Drop for CaptureEnvironment {
    fn drop(&mut self) {
        restore_env(CAPTURE_PATH_ENV, self.path.take());
        restore_env(CAPTURE_SECONDS_ENV, self.seconds.take());
    }
}

#[cfg(test)]
mod tests;
