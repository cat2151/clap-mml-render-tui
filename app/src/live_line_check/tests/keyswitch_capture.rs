//! sampler の key switch が、行の LIVE 演奏経路（`play_line` → realtime play server）で
//! 効くかを、live mix の出力（`CMRT_LIVE_CAPTURE_WAV`）で確かめる。
//!
//! 同じ 3 音を「KS なし」「各音の前に KS」「2 音目の前だけ別の KS」で鳴らし、音ごとの
//! 波形の相関を比べる。音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 続けて同じ server で 2 行を鳴らし、1 行目で押した KS が 2 行目まで残るかも測る。
//! 2 行目どうしは、それまでの履歴が同じ 2 つの server の間でだけ比べる（別の履歴と
//! 比べると round robin の位置の差が混ざる）。
//!
//! 実機の音色が要るので通常は skip。音色（`patches_dirs` からの相対）を環境変数で渡す:
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! $env:CMRT_TEST_KEYSWITCH_PATCH = "sfz/<library>/Programs/<program>.sfz"
//! $env:CMRT_TEST_KEYSWITCH_OUT_DIR = "<録った WAV を残す dir>"   # 省略時は temp
//! cargo test -p clap-mml-render-tui --lib keyswitch_capture -- --ignored --nocapture
//! ```
//!
//! KS の note number は METAL-GTX の配置（17 = Sus_Down、26 = Hammer-On）。

use super::*;

const PLAY_SERVER_EXE_ENV: &str = "CMRT_TEST_PLAY_SERVER_EXE";
/// server は port を CLI 引数ではなく config か この環境変数で受ける。
const PLAY_SERVER_PORT_ENV: &str = "CMRT_REALTIME_PLAY_SERVER_PORT";
const PATCH_ENV: &str = "CMRT_TEST_KEYSWITCH_PATCH";
const OUT_DIR_ENV: &str = "CMRT_TEST_KEYSWITCH_OUT_DIR";

const SUS_DOWN: u8 = 17;
const HAMMER_ON: u8 = 26;
const PITCHES: [u8; 3] = [40, 42, 43];
const NOTE_SECONDS: f64 = 0.5;
/// 1 行の頭から次の行までの間隔。3 音と余韻が収まる長さ。
const STEP_MS: u64 = 3_500;
/// live の録音の上限。読み込みの間も録るので、その長さぶん余裕を持たせる。
const LIVE_CAPTURE_SECONDS: u32 = 120;
/// 音色の読み込みを待つ上限。冷えた状態の 1GB 級の sfz でも収まる長さ。
const LOAD_TIMEOUT: Duration = Duration::from_secs(120);
/// 相関を取るときに探すずれの幅。onset の検出が音色で数 ms 揺れるぶん。
const MAX_LAG_SECONDS: f64 = 0.010;
/// 同じ音とみなす相関の下限。
const SAME: f64 = 0.99;
/// 違う音とみなす相関の上限。
const DIFFERENT: f64 = 0.90;

/// 各音の直前に押す KS。`None` はその音で KS を押さない。
type KeySwitches = [Option<u8>; 3];

const PLAIN: KeySwitches = [None, None, None];
const SUS_EACH: KeySwitches = [Some(SUS_DOWN), Some(SUS_DOWN), Some(SUS_DOWN)];
const HAMMER_SECOND: KeySwitches = [None, Some(HAMMER_ON), None];

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn key_switches_change_the_sound_on_the_live_line_path() {
    let (Ok(patch), Some(exe)) = (
        std::env::var(PATCH_ENV),
        std::env::var_os(PLAY_SERVER_EXE_ENV),
    ) else {
        eprintln!("keyswitch-capture: skip ({PATCH_ENV} か {PLAY_SERVER_EXE_ENV} が無い)");
        return;
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
    cfg.play_server_launch_override = Some(cmrt_runtime::PlayServerLaunch::Executable(exe.into()));

    let record = |label: &str, lines: &[KeySwitches]| {
        let segments = record_lines(&cfg, &patch, &out_dir, label, lines);
        eprintln!("keyswitch-capture: recorded {label}");
        segments
    };
    let plain = record("a-plain", &[PLAIN]);
    let sus_each = record("b-sus-each", &[SUS_EACH]);
    let hammer = record("c-hammer-second", &[HAMMER_SECOND]);
    let hammer_then_plain = record("d-hammer-then-plain", &[HAMMER_SECOND, PLAIN]);
    let hammer_then_sus = record("e-hammer-then-sus", &[HAMMER_SECOND, SUS_EACH]);

    restore_env(PLAY_SERVER_PORT_ENV, previous_port);

    let a_b = compare("a vs b", &plain[0], &sus_each[0]);
    let a_c = compare("a vs c", &plain[0], &hammer[0]);
    let c_d = compare("c vs d(1 行目)", &hammer[0], &hammer_then_plain[0]);
    let d_e = compare(
        "d(2 行目) vs e(2 行目)",
        &hammer_then_plain[1],
        &hammer_then_sus[1],
    );

    assert!(
        a_b.iter().all(|ncc| *ncc > SAME),
        "KS 17 を足すと音が変わった"
    );
    assert!(a_c[0] > SAME, "c の 1 音目が a と違う");
    assert!(a_c[1] < DIFFERENT, "KS 26 を押した 2 音目が a と同じ音");
    assert!(
        c_d.iter().all(|ncc| *ncc > SAME),
        "同じ入力を起こし直した server で鳴らすと音が変わる（比べ方が壊れている）"
    );
    assert!(
        d_e[0] < DIFFERENT,
        "KS なしの 2 行目が KS 17 の 2 行目と同じ音（前の行の KS が残っていない）"
    );
}

fn print_log(line: &str) {
    eprintln!("{line}");
}

/// 1 行ぶんの録音（mono）と、その中で 1 音目が鳴り始めた frame。
struct Segment {
    samples: Vec<f32>,
    onset: usize,
    sample_rate: u32,
}

/// 同じ server で `lines` を順に鳴らし、行ごとの録音を返す。
fn record_lines(
    cfg: &Config,
    patch: &str,
    out_dir: &Path,
    label: &str,
    lines: &[KeySwitches],
) -> Vec<Segment> {
    let check_lines = lines
        .iter()
        .map(|switches| {
            let source = format!(r#"{{"Surge XT patch": "{patch}"}} c"#);
            let mut live = live_line(&source).expect("行を作れない");
            live.program.performance.events = phrase(switches);
            live.program.performance.loop_seconds = NOTE_SECONDS * PITCHES.len() as f64;
            CheckLine { source, live }
        })
        .collect::<Vec<_>>();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = out_dir.join(format!("keyswitch-{label}-{nonce}.wav"));
    let sent = {
        let _env = LiveCaptureEnvironment::set(&path);
        send_after_load(cfg, &check_lines, &path, label)
    };
    eprintln!("keyswitch-capture: {label} wav={}", path.display());
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
    sent.iter()
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
        .collect()
}

/// 行の LIVE 演奏の出力を録らせる環境変数。抜けるときに元へ戻す。
///
/// device 側の録音は音色の準備で再生が始まった所から決まった長さしか録らないので、
/// 読み込みの長さが読めない重い音色には使えない。こちらは停止で書き出される。
struct LiveCaptureEnvironment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl LiveCaptureEnvironment {
    const PATH: &'static str = "CMRT_LIVE_CAPTURE_WAV";
    const SECONDS: &'static str = "CMRT_LIVE_CAPTURE_SECONDS";

    fn set(path: &Path) -> Self {
        let previous = [Self::PATH, Self::SECONDS]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect();
        std::env::set_var(Self::PATH, path);
        std::env::set_var(Self::SECONDS, format!("{LIVE_CAPTURE_SECONDS}"));
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

/// server を起こして音色を読み終えてから、行を [`STEP_MS`] おきに送る。
///
/// [`send_lines`] は読み込みの時間も間隔に数えるので、重い音色だと 1 行目が
/// 鳴る前に次の行（や停止）が来てしまう。
fn send_after_load(
    cfg: &Config,
    lines: &[CheckLine],
    capture_path: &Path,
    label: &str,
) -> Vec<Sent> {
    let supervisor = Arc::new(RealtimePlayServerSupervisor::with_live_instance_count(
        cfg, 2,
    ));
    supervisor
        .start_owned_for_fast_midi()
        .expect("realtime play server を起動できない");
    let sender = MmlOverlaySender::new(Arc::clone(&supervisor), cfg.sample_rate);

    let load_started = Instant::now();
    let prepare_id = sender.prepare(lines[0].live.patch.clone());
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
        "keyswitch-capture: {label} load_ms={}",
        load_started.elapsed().as_millis()
    );

    let step = Duration::from_millis(STEP_MS);
    let first = Instant::now();
    let mut sent = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        sleep_until(first + step * index as u32);
        let requested = Instant::now();
        let command_id = sender.play_line(line.live.patch.clone(), line.live.program.clone());
        wait_for_line(&sender, command_id).expect("行を送れない");
        sent.push(Sent {
            requested,
            started: Instant::now(),
        });
    }
    sleep_until(first + step * lines.len() as u32);
    let stop_id = sender.stop();
    wait_for_command(&sender, stop_id).expect("止められない");
    drop(sender);
    let deadline = Instant::now() + WRITE_TIMEOUT;
    wait_for_written(capture_path, deadline).expect("録音が書き出されない");
    drop(supervisor);
    sent
}

/// 3 音の単音フレーズ。同時刻は note off → KS → 演奏音の順に積む。
fn phrase(switches: &KeySwitches) -> Vec<cmrt_chord::TimedMidiEvent> {
    let event = |seconds: f64, message: [u8; 3]| cmrt_chord::TimedMidiEvent { seconds, message };
    let mut events = Vec::new();
    let mut held: Vec<u8> = Vec::new();
    for (index, (pitch, switch)) in PITCHES.iter().zip(switches).enumerate() {
        let at = index as f64 * NOTE_SECONDS;
        for key in held.drain(..) {
            events.push(event(at, [0x80, key, 0]));
        }
        if let Some(switch) = switch {
            events.push(event(at, [0x90, *switch, 127]));
            held.push(*switch);
        }
        events.push(event(at, [0x90, *pitch, 100]));
        held.push(*pitch);
    }
    let end = PITCHES.len() as f64 * NOTE_SECONDS;
    for key in held {
        events.push(event(end, [0x80, key, 0]));
    }
    events
}

/// 最大振幅の 5% を初めて超えた frame。
fn onset(samples: &[f32]) -> usize {
    let peak = samples.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
    samples
        .iter()
        .position(|s| s.abs() > peak * 0.05)
        .unwrap_or(0)
}

/// 音ごとに、ずれを探した最大の正規化相関を出して返す。RMS と頭 30ms の peak も並べる。
fn compare(label: &str, left: &Segment, right: &Segment) -> Vec<f64> {
    let sample_rate = left.sample_rate as f64;
    let note = (NOTE_SECONDS * sample_rate) as usize;
    let max_lag = (MAX_LAG_SECONDS * sample_rate) as isize;
    let head = (0.030 * sample_rate) as usize;
    let mut out = Vec::new();
    for index in 0..PITCHES.len() {
        let l = window(&left.samples, left.onset + index * note, note);
        let ncc = (-max_lag..=max_lag)
            .map(|lag| {
                let start = (right.onset + index * note) as isize + lag;
                let r = window(&right.samples, start.max(0) as usize, note);
                normalized_correlation(l, r)
            })
            .fold(f64::MIN, f64::max);
        let r = window(&right.samples, right.onset + index * note, note);
        eprintln!(
            "keyswitch-capture: {label} note={} ncc={ncc:.4} rms={:.4}/{:.4} head_peak={:.4}/{:.4}",
            index + 1,
            rms(l),
            rms(r),
            peak(&l[..head.min(l.len())]),
            peak(&r[..head.min(r.len())]),
        );
        out.push(ncc);
    }
    out
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

fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
}

fn peak(samples: &[f32]) -> f64 {
    samples
        .iter()
        .fold(0.0, |peak, s| peak.max(f64::from(s.abs())))
}

/// 起動中の TUI の server と port がぶつからないよう、空いている port を借りる。
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .expect("空いている port を取れない")
}
