//! 実機の sampler 音色を行の LIVE 演奏経路（`play_line` → realtime play server）で鳴らし、
//! live mix の出力（`CMRT_LIVE_CAPTURE_WAV`）を録って比べる補助。
//!
//! 使う側は通常 skip の `#[ignore]` テストで、音色（`patches_dirs` からの相対）と server を
//! 環境変数で受ける:
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! $env:CMRT_TEST_KEYSWITCH_PATCH = "sfz/<library>/Programs/<program>.sfz"
//! $env:CMRT_TEST_KEYSWITCH_OUT_DIR = "<録った WAV を残す dir>"   # 省略時は temp
//! ```

use std::sync::Mutex;

use cmrt_mml_overlay::LivePatch;

use super::*;

const PLAY_SERVER_EXE_ENV: &str = "CMRT_TEST_PLAY_SERVER_EXE";
/// server は port を CLI 引数ではなく config か この環境変数で受ける。
const PLAY_SERVER_PORT_ENV: &str = "CMRT_REALTIME_PLAY_SERVER_PORT";
const PATCH_ENV: &str = "CMRT_TEST_KEYSWITCH_PATCH";
const OUT_DIR_ENV: &str = "CMRT_TEST_KEYSWITCH_OUT_DIR";

/// 1 行の頭から次の行までの間隔。数音と余韻が収まる長さ。
pub(super) const STEP_MS: u64 = 3_500;
/// live の録音の上限。読み込みの間も録るので、その長さぶん余裕を持たせる。
pub(super) const LIVE_CAPTURE_SECONDS: u32 = 120;
/// 音色の読み込みを待つ上限。冷えた状態の 1GB 級の sfz でも収まる長さ。
const LOAD_TIMEOUT: Duration = Duration::from_secs(120);
/// [`print_log`] が受けたログの行（server の stderr を含む）。
static LOGGED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// 相関を取るときに探すずれの幅。onset の検出が音色で数 ms 揺れるぶん。
const MAX_LAG_SECONDS: f64 = 0.010;
/// 同じ音とみなす相関の下限。
pub(super) const SAME: f64 = 0.99;
/// 違う音とみなす相関の上限。
pub(super) const DIFFERENT: f64 = 0.90;

/// 録音の設定。環境変数から作り、抜けるときに server の port の環境変数を元へ戻す。
pub(super) struct CaptureSetup {
    tag: &'static str,
    cfg: Config,
    patch: String,
    out_dir: PathBuf,
    previous_port: Option<std::ffi::OsString>,
}

impl CaptureSetup {
    /// 音色か server の環境変数が無ければ `None`（呼び出し側は skip する）。
    /// `tag` はログの接頭辞と WAV の名前に使う。
    pub(super) fn from_env(tag: &'static str) -> Option<Self> {
        let (Ok(patch), Some(exe)) = (
            std::env::var(PATCH_ENV),
            std::env::var_os(PLAY_SERVER_EXE_ENV),
        ) else {
            eprintln!("{tag}: skip ({PATCH_ENV} か {PLAY_SERVER_EXE_ENV} が無い)");
            return None;
        };
        let out_dir = std::env::var_os(OUT_DIR_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        // 失敗したときに server の stderr と送信の経過が見えるように。
        cmrt_realtime_play::set_log_sink(print_log);
        cmrt_mml_overlay::set_log_sink(print_log);
        let mut cfg = crate::config::load().expect("config を読めない");
        cfg.realtime_play_server_port = free_port();
        let previous_port = std::env::var_os(PLAY_SERVER_PORT_ENV);
        std::env::set_var(
            PLAY_SERVER_PORT_ENV,
            cfg.realtime_play_server_port.to_string(),
        );
        cfg.play_server_launch_override =
            Some(cmrt_runtime::PlayServerLaunch::Executable(exe.into()));
        Some(Self {
            tag,
            cfg,
            patch,
            out_dir,
            previous_port,
        })
    }

    /// 音色（`patches_dirs` からの相対）。
    pub(super) fn patch(&self) -> &str {
        &self.patch
    }

    /// server を起こし直し、同じ server で `lines` を [`STEP_MS`] おきに順に鳴らして、
    /// 行ごとの録音を返す。各行は最後のイベントの秒までを 1 回だけ鳴らす。
    pub(super) fn record(
        &self,
        label: &str,
        lines: &[Vec<cmrt_chord::TimedMidiEvent>],
    ) -> Vec<Segment> {
        let check_lines = lines
            .iter()
            .map(|events| {
                let source = format!(r#"{{"Surge XT patch": "{}"}} c"#, self.patch);
                let mut live = live_line(&source).expect("行を作れない");
                live.program.performance.loop_seconds =
                    events.last().map_or(0.0, |event| event.seconds);
                live.program.performance.events = events.clone();
                CheckLine { source, live }
            })
            .collect::<Vec<_>>();
        self.record_session(
            label,
            &check_lines[0].live.patch,
            LIVE_CAPTURE_SECONDS,
            |sender| send_steps(sender, &check_lines),
        )
    }

    /// server を起こし直して `first_patch` を読み終えてから、`play` に sender を渡して鳴らさせ、
    /// `play` が返した行ごとの録音を返す。`play` が返った後に止めて、録音を書き出させる。
    /// 1 行目の送信より前に音を出さないこと（行の頭を、録音で最初に鳴った所から数える）。
    pub(super) fn record_session(
        &self,
        label: &str,
        first_patch: &LivePatch,
        capture_seconds: u32,
        play: impl FnOnce(&MmlOverlaySender) -> Vec<Sent>,
    ) -> Vec<Segment> {
        let tag = self.tag;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let path = self.out_dir.join(format!("{tag}-{label}-{nonce}.wav"));
        let sent = {
            let _env = LiveCaptureEnvironment::set(&path, capture_seconds);
            send_after_load(&self.cfg, first_patch, &path, tag, label, play)
        };
        eprintln!("{tag}: {label} wav={}", path.display());
        let capture = read_capture(&path).expect("録音を読めない");
        let mono = capture
            .samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[left, right]| (left + right) * 0.5)
            .collect::<Vec<_>>();
        // 録音は音色の準備で live が始まった所から。1 行目の頭は読み込みの後の無音の先に探し、
        // 2 行目以降はそこから送った時刻の差だけ進めた所で探す。
        let step = duration_frames(Duration::from_millis(STEP_MS), capture.sample_rate);
        let first = onset(&mono);
        let margin = duration_frames(Duration::from_millis(100), capture.sample_rate);
        let segments = sent
            .iter()
            .map(|line| {
                let offset = duration_frames(
                    line.started.saturating_duration_since(sent[0].started),
                    capture.sample_rate,
                );
                let start = (first + offset).saturating_sub(margin).min(mono.len());
                let end = (start + step).min(mono.len());
                let samples = mono[start..end].to_vec();
                let onset = onset(&samples);
                Segment {
                    samples,
                    onset,
                    sample_rate: capture.sample_rate,
                }
            })
            .collect();
        eprintln!("{tag}: recorded {label}");
        segments
    }
}

impl Drop for CaptureSetup {
    fn drop(&mut self) {
        restore_env(PLAY_SERVER_PORT_ENV, self.previous_port.take());
    }
}

fn print_log(line: &str) {
    eprintln!("{line}");
    LOGGED.lock().unwrap().push(line.to_string());
}

/// これまでにログへ出た行の数。[`logged_lines_since`] の起点にする。
pub(super) fn logged_line_count() -> usize {
    LOGGED.lock().unwrap().len()
}

/// `start` 番目以降にログへ出た行。
pub(super) fn logged_lines_since(start: usize) -> Vec<String> {
    LOGGED.lock().unwrap()[start..].to_vec()
}

/// 1 行ぶんの録音（mono）と、その中で最初の音が鳴り始めた frame。
pub(super) struct Segment {
    pub(super) samples: Vec<f32>,
    pub(super) onset: usize,
    pub(super) sample_rate: u32,
}

impl Segment {
    /// `onset` から `start_seconds` 進めた所の `seconds` の長さと、`other` の同じ所とを、
    /// ずれを [`MAX_LAG_SECONDS`] の幅で探した最大の正規化相関。
    pub(super) fn correlation(&self, other: &Segment, start_seconds: f64, seconds: f64) -> f64 {
        let sample_rate = f64::from(self.sample_rate);
        let len = (seconds * sample_rate) as usize;
        let offset = (start_seconds * sample_rate) as usize;
        let max_lag = (MAX_LAG_SECONDS * sample_rate) as isize;
        let left = window(&self.samples, self.onset + offset, len);
        (-max_lag..=max_lag)
            .map(|lag| {
                let start = (other.onset + offset) as isize + lag;
                normalized_correlation(left, window(&other.samples, start.max(0) as usize, len))
            })
            .fold(f64::MIN, f64::max)
    }

    /// `onset` から `start_seconds` 進めた所の `seconds` の長さ。
    pub(super) fn window(&self, start_seconds: f64, seconds: f64) -> &[f32] {
        let sample_rate = f64::from(self.sample_rate);
        window(
            &self.samples,
            self.onset + (start_seconds * sample_rate) as usize,
            (seconds * sample_rate) as usize,
        )
    }
}

/// 行の LIVE 演奏の出力を録らせる環境変数。抜けるときに元へ戻す。
///
/// device 側の録音は音色の準備で再生が始まった所から決まった長さしか録らないので、
/// 読み込みの長さが読めない重い音色には使えない。こちらは停止で書き出される。
struct LiveCaptureEnvironment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl LiveCaptureEnvironment {
    const PATH: &'static str = "CMRT_LIVE_CAPTURE_WAV";
    const SECONDS: &'static str = "CMRT_LIVE_CAPTURE_SECONDS";

    fn set(path: &Path, seconds: u32) -> Self {
        let previous = [Self::PATH, Self::SECONDS]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        std::env::set_var(Self::PATH, path);
        std::env::set_var(Self::SECONDS, format!("{seconds}"));
        Self(previous)
    }
}

impl Drop for LiveCaptureEnvironment {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..) {
            restore_env(name, value);
        }
    }
}

/// server を起こして `first_patch` を読み終えてから、`play` に鳴らさせる。
///
/// [`send_lines`] は読み込みの時間も間隔に数えるので、重い音色だと 1 行目が
/// 鳴る前に次の行（や停止）が来てしまう。
fn send_after_load(
    cfg: &Config,
    first_patch: &LivePatch,
    capture_path: &Path,
    tag: &str,
    label: &str,
    play: impl FnOnce(&MmlOverlaySender) -> Vec<Sent>,
) -> Vec<Sent> {
    let supervisor = Arc::new(RealtimePlayServerSupervisor::with_live_instance_count(
        cfg, 2,
    ));
    supervisor
        .start_owned_for_fast_midi()
        .expect("realtime play server を起動できない");
    let sender = MmlOverlaySender::new(Arc::clone(&supervisor), cfg.sample_rate);

    let load_started = Instant::now();
    let prepare_id = sender.prepare(first_patch.clone());
    wait_for_command(&sender, prepare_id).expect("準備が始まらない");
    let deadline = load_started + LOAD_TIMEOUT;
    loop {
        let status = sender.status();
        assert!(
            status.prepare_error().is_none(),
            "{:?}",
            status.prepare_error()
        );
        if !status.is_loading() {
            break;
        }
        assert!(Instant::now() < deadline, "音色の読み込みが終わらない");
        std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!(
        "{tag}: {label} load_ms={}",
        load_started.elapsed().as_millis()
    );

    let sent = play(&sender);
    let stop_id = sender.stop();
    wait_for_command(&sender, stop_id).expect("止められない");
    drop(sender);
    let deadline = Instant::now() + WRITE_TIMEOUT;
    wait_for_written(capture_path, deadline).expect("録音が書き出されない");
    drop(supervisor);
    sent
}

/// 行を [`STEP_MS`] おきに送り、最後の行の後も同じ間隔だけ鳴らしておく。
fn send_steps(sender: &MmlOverlaySender, lines: &[CheckLine]) -> Vec<Sent> {
    let step = Duration::from_millis(STEP_MS);
    let first = Instant::now();
    let mut sent = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        sleep_until(first + step * index as u32);
        sent.push(send_line(sender, &line.live.patch, &line.live.program));
    }
    sleep_until(first + step * lines.len() as u32);
    sent
}

/// 1 行を送り、timeline に積まれるまで待つ。
pub(super) fn send_line(
    sender: &MmlOverlaySender,
    patch: &LivePatch,
    program: &cmrt_mml_overlay::line_play::LineProgram,
) -> Sent {
    let requested = Instant::now();
    let command_id = sender.play_line(patch.clone(), program.clone());
    wait_for_line(sender, command_id).expect("行を送れない");
    Sent {
        requested,
        started: Instant::now(),
    }
}

/// 最大振幅の 5% を初めて超えた frame。
fn onset(samples: &[f32]) -> usize {
    let peak = peak(samples) as f32;
    samples
        .iter()
        .position(|s| s.abs() > peak * 0.05)
        .unwrap_or(0)
}

fn window(samples: &[f32], start: usize, len: usize) -> &[f32] {
    let start = start.min(samples.len());
    &samples[start..(start + len).min(samples.len())]
}

fn normalized_correlation(left: &[f32], right: &[f32]) -> f64 {
    let len = left.len().min(right.len());
    let (mut dot, mut ll, mut rr) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (l, r) in left[..len].iter().zip(&right[..len]) {
        let (l, r) = (f64::from(*l), f64::from(*r));
        dot += l * r;
        ll += l * l;
        rr += r * r;
    }
    if ll == 0.0 || rr == 0.0 {
        return 0.0;
    }
    dot / (ll * rr).sqrt()
}

pub(super) fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
}

pub(super) fn peak(samples: &[f32]) -> f64 {
    samples
        .iter()
        .fold(0.0, |peak, s| peak.max(f64::from(s.abs())))
}

pub(super) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

pub(super) fn std_dev(values: &[f64]) -> f64 {
    let mean = mean(values);
    (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
}

/// 標準偏差を平均で割った値。
pub(super) fn variation(values: &[f64]) -> f64 {
    std_dev(values) / mean(values)
}

/// 起動中の TUI の server と port がぶつからないよう、空いている port を借りる。
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .expect("空いている port を取れない")
}
