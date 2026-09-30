//! Guitar Articulation の汚し（[`RowRule::Humanize`]）が、行の LIVE 演奏経路で実際に音を
//! ばらつかせるかを、live mix の出力で確かめる。
//!
//! 同じ E2 の 8 分を休符を挟んで 6 回鳴らす行を、4 条件で鳴らす:
//! a: 各音の前に CC30=0、b: 各音の前に CC30=127、c: 汚し OFF の [`convert`]、d: 汚し ON の [`convert`]。
//! a と b で CC30（ピッキングノイズ層の音量）が音の頭を変えるか、c と d で音ごとの頭の大きさと
//! 発音の間隔がばらけるかを数える。
//!
//! 休符を挟むのは、音ごとの発音を無音からの立ち上がりで拾うため（同じ音高のレガートでは
//! 前の音の余韻と本体の遅れ `delay_cc30` に紛れて、発音の検出が汚しの幅より大きく揺れる）。
//! 音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib humanize_capture -- --ignored --nocapture
//! ```

use cmrt_guitar_articulation::{convert, RowRule, RuleTable};

use super::capture_support::{mean, peak, rms, std_dev, variation, CaptureSetup, Segment};

const MML: &str = "o3 l8 e r e r e r e r e r e r";
const NOTE_COUNT: usize = 6;
/// 音の頭どうしの間隔（`MML` のテンポ 120 の値）。
const NOTE_SECONDS: f64 = 0.5;
/// 同じ server で続けて鳴らす行の数。1 行 6 音では間隔の標準偏差が安定しないので数を稼ぐ。
const LINES: usize = 4;
/// ピッキングノイズを測る頭の窓。CC30 が大きいと本体は最大 20 ms 遅れるので、その手前。
const NOISE_HEAD_SECONDS: f64 = 0.010;
/// 音ごとの大きさを測る頭の窓。本体の遅れ（`delay_cc30`）を越えて本体の頭まで含む長さ。
const BODY_HEAD_SECONDS: f64 = 0.060;
/// 音ごとの発音を探す幅（予定の時刻の前後）。汚しのずれ（±8 ms）が収まる幅。
const SEARCH_SECONDS: f64 = 0.040;
/// 発音とみなす振幅（行の最大振幅に対する割合）と、それを測る塊の長さ。
const ONSET_RATIO: f64 = 0.02;
const BLOCK_SECONDS: f64 = 0.001;
/// 演奏音とみなす note number の下限（METAL-GTX の KS は 17〜26）。
const LOWEST_PLAYED_NOTE: u8 = 28;
const PICKING_CC: u8 = 30;

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn humanize_spreads_the_attacks_on_the_live_line_path() {
    let Some(setup) = CaptureSetup::from_env("humanize-capture") else {
        return;
    };
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let plain = convert(&raw, &RuleTable::default());
    let mut humanize = RuleTable::default();
    humanize.toggle_row(RowRule::Humanize);
    let humanized = convert(&raw, &humanize);

    let record = |label: &str, events: Vec<cmrt_chord::TimedMidiEvent>| {
        measure(label, &setup.record(label, &vec![events; LINES]))
    };
    let a = record("a-cc30-0", with_picking_cc(&plain, 0));
    let b = record("b-cc30-127", with_picking_cc(&plain, 127));
    let c = record("c-humanize-off", plain);
    let d = record("d-humanize-on", humanized);
    drop(setup);

    let mut failures = Vec::new();
    let (a_noise, b_noise) = (mean(&a.noise_rms), mean(&b.noise_rms));
    eprintln!("humanize-capture: noise_rms a={a_noise:.4} b={b_noise:.4}");
    if b_noise <= a_noise {
        failures.push(format!(
            "CC30=127 の頭 {NOISE_HEAD_SECONDS} 秒の RMS {b_noise:.4} が CC30=0 の {a_noise:.4} 以下"
        ));
    }
    let (c_spread, d_spread) = (variation(&c.body_peak), variation(&d.body_peak));
    eprintln!("humanize-capture: body_peak cv c={c_spread:.4} d={d_spread:.4}");
    if d_spread <= c_spread {
        failures.push(format!(
            "汚し ON の頭の peak のばらつき {d_spread:.4} が OFF の {c_spread:.4} 以下"
        ));
    }
    let (c_intervals, d_intervals) = (std_dev(&c.intervals), std_dev(&d.intervals));
    eprintln!("humanize-capture: interval std c={c_intervals:.5}s d={d_intervals:.5}s");
    if d_intervals <= c_intervals {
        failures.push(format!(
            "汚し ON の発音間隔の標準偏差 {d_intervals:.5} 秒が OFF の {c_intervals:.5} 秒以下"
        ));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// 各演奏音の note on の直前に CC30 を `value` で足す。
fn with_picking_cc(
    events: &[cmrt_chord::TimedMidiEvent],
    value: u8,
) -> Vec<cmrt_chord::TimedMidiEvent> {
    let mut out = Vec::with_capacity(events.len() * 2);
    for event in events {
        let [status, note, velocity] = event.message;
        if status & 0xF0 == 0x90 && velocity > 0 && note >= LOWEST_PLAYED_NOTE {
            out.push(cmrt_chord::TimedMidiEvent {
                seconds: event.seconds,
                message: [0xB0 | (status & 0x0F), PICKING_CC, value],
            });
        }
        out.push(*event);
    }
    out
}

/// 全行の音ごとの頭の大きさと、行の中の発音の間隔（秒）。
struct Attacks {
    noise_rms: Vec<f64>,
    body_peak: Vec<f64>,
    intervals: Vec<f64>,
}

fn measure(label: &str, segments: &[Segment]) -> Attacks {
    let mut attacks = Attacks {
        noise_rms: Vec::new(),
        body_peak: Vec::new(),
        intervals: Vec::new(),
    };
    for (line, segment) in segments.iter().enumerate() {
        let rate = f64::from(segment.sample_rate);
        let first = segment.onset as f64 / rate;
        let threshold = peak(&segment.samples) * ONSET_RATIO;
        let onsets = (0..NOTE_COUNT)
            .map(|index| attack(segment, first + index as f64 * NOTE_SECONDS, threshold))
            .collect::<Vec<_>>();
        let head = |at: f64, seconds: f64| {
            let start = ((at * rate) as usize).min(segment.samples.len());
            let end = (start + (seconds * rate) as usize).min(segment.samples.len());
            &segment.samples[start..end]
        };
        for (index, at) in onsets.iter().enumerate() {
            let noise = rms(head(*at, NOISE_HEAD_SECONDS));
            let body = peak(head(*at, BODY_HEAD_SECONDS));
            eprintln!(
                "humanize-capture: {label} line={} note={} onset={:.4} noise_rms={noise:.4} body_peak={body:.4}",
                line + 1,
                index + 1,
                at - onsets[0],
            );
            attacks.noise_rms.push(noise);
            attacks.body_peak.push(body);
        }
        attacks
            .intervals
            .extend(onsets.windows(2).map(|pair| pair[1] - pair[0]));
    }
    attacks
}

/// 予定の秒 `expected` の前後 [`SEARCH_SECONDS`] で、[`BLOCK_SECONDS`] ごとの最大振幅が
/// `threshold` 未満から以上へ上がった最初の塊の頭の秒。見つからなければ `expected`。
fn attack(segment: &Segment, expected: f64, threshold: f64) -> f64 {
    let rate = f64::from(segment.sample_rate);
    let block = ((BLOCK_SECONDS * rate) as usize).max(1);
    let from = ((expected - SEARCH_SECONDS).max(0.0) * rate) as usize;
    let to = (((expected + SEARCH_SECONDS) * rate) as usize).min(segment.samples.len());
    let loud = |start: usize| peak(&segment.samples[start..(start + block).min(to)]) >= threshold;
    (from..to)
        .step_by(block)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|pair| !loud(pair[0]) && loud(pair[1]))
        .map_or(expected, |pair| pair[1] as f64 / rate)
}
