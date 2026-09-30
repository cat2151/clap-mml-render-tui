//! Guitar Articulation の汚し（リリース、[`RowRule::HumanizeRelease`]）が、行の LIVE 演奏経路で
//! 実際に離した後の音を変えるかを、live mix の出力で確かめる。
//!
//! 同じ E2 の 8 分を休符を挟んで 6 回鳴らす行を、4 条件で鳴らす:
//! a: 各音の前に CC24=0（リリース音なし）、b: 各音の前に CC24=13・CC25=127（Basic を最大音量）、
//! c: 汚し（リリース）OFF の [`convert`]、d: 汚し（リリース）ON の [`convert`]。
//! 各音の note off の後の窓の RMS で、a と b でリリース層が鳴るか、c と d で音ごとにばらけるかを数える。
//!
//! 休符を挟むのは、離した後の窓に次の音の頭を入れないため。
//! 音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib release_capture -- --ignored --nocapture
//! ```

use cmrt_chord::TimedMidiEvent;
use cmrt_guitar_articulation::{convert, RowRule, RuleTable};

use super::capture_support::{mean, rms, variation, CaptureSetup, Segment};

const MML: &str = "o3 l8 e r e r e r e r e r e r";
/// 同じ server で続けて鳴らす行の数。1 行 6 音では cv が安定しないので数を稼ぐ。
const LINES: usize = 4;
/// 離した後の窓。本体の発音を含めず、次の音の頭（離してから 250 ms 後）より手前で終わる。
const RELEASE_FROM_SECONDS: f64 = 0.010;
const RELEASE_TO_SECONDS: f64 = 0.150;
/// 演奏音とみなす note number の下限（METAL-GTX の KS は 17〜26）。
const LOWEST_PLAYED_NOTE: u8 = 28;
const RELEASE_SHAPE_CC: u8 = 24;
const RELEASE_LEVEL_CC: u8 = 25;

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn humanize_release_spreads_the_release_sounds_on_the_live_line_path() {
    let Some(setup) = CaptureSetup::from_env("release-capture") else {
        return;
    };
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let plain = convert(&raw, &RuleTable::default());
    let mut release = RuleTable::default();
    release.toggle_row(RowRule::HumanizeRelease);
    let released = convert(&raw, &release);
    let offs = release_times(&plain);
    assert_eq!(offs, release_times(&released), "note off の時刻が変わった");

    let record = |label: &str, events: Vec<TimedMidiEvent>| {
        measure(label, &setup.record(label, &vec![events; LINES]), &offs)
    };
    let a = record(
        "a-cc24-0",
        with_ccs_before_note_ons(&plain, &[(RELEASE_SHAPE_CC, 0)]),
    );
    let b = record(
        "b-cc24-13-cc25-127",
        with_ccs_before_note_ons(&plain, &[(RELEASE_SHAPE_CC, 13), (RELEASE_LEVEL_CC, 127)]),
    );
    let c = record("c-release-off", plain);
    let d = record("d-release-on", released);
    drop(setup);

    let mut failures = Vec::new();
    let (a_mean, b_mean) = (mean(&a), mean(&b));
    let (c_mean, d_mean) = (mean(&c), mean(&d));
    eprintln!(
        "release-capture: release_rms mean a={a_mean:.5} b={b_mean:.5} c={c_mean:.5} d={d_mean:.5}"
    );
    if b_mean <= a_mean {
        failures.push(format!(
            "CC24=13・CC25=127 の離した後の RMS {b_mean:.5} が CC24=0 の {a_mean:.5} 以下"
        ));
    }
    let (c_spread, d_spread) = (variation(&c), variation(&d));
    eprintln!("release-capture: release_rms cv c={c_spread:.4} d={d_spread:.4}");
    if d_spread <= c_spread {
        failures.push(format!(
            "汚し（リリース）ON の離した後の RMS のばらつき {d_spread:.4} が OFF の {c_spread:.4} 以下"
        ));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// 各演奏音の note off の秒を、最初の演奏音の note on からの秒で。
fn release_times(events: &[TimedMidiEvent]) -> Vec<f64> {
    let played = |event: &&TimedMidiEvent| event.message[1] >= LOWEST_PLAYED_NOTE;
    let first_on = events
        .iter()
        .filter(played)
        .find(|event| event.message[0] & 0xF0 == 0x90 && event.message[2] > 0)
        .expect("演奏音が無い")
        .seconds;
    events
        .iter()
        .filter(played)
        .filter(|event| {
            let [status, _, velocity] = event.message;
            status & 0xF0 == 0x80 || (status & 0xF0 == 0x90 && velocity == 0)
        })
        .map(|event| event.seconds - first_on)
        .collect()
}

/// 各演奏音の note on の直前に `ccs` を足す。
fn with_ccs_before_note_ons(events: &[TimedMidiEvent], ccs: &[(u8, u8)]) -> Vec<TimedMidiEvent> {
    let mut out = Vec::with_capacity(events.len() * (ccs.len() + 1));
    for event in events {
        let [status, note, velocity] = event.message;
        if status & 0xF0 == 0x90 && velocity > 0 && note >= LOWEST_PLAYED_NOTE {
            out.extend(ccs.iter().map(|&(cc, value)| TimedMidiEvent {
                seconds: event.seconds,
                message: [0xB0 | (status & 0x0F), cc, value],
            }));
        }
        out.push(*event);
    }
    out
}

/// 全行の音ごとの、離した後の窓の RMS。行の頭（`onset`）を最初の演奏音の note on とみなす。
fn measure(label: &str, segments: &[Segment], offs: &[f64]) -> Vec<f64> {
    let mut values = Vec::with_capacity(segments.len() * offs.len());
    for (line, segment) in segments.iter().enumerate() {
        for (index, off) in offs.iter().enumerate() {
            let value = rms(segment.window(
                off + RELEASE_FROM_SECONDS,
                RELEASE_TO_SECONDS - RELEASE_FROM_SECONDS,
            ));
            eprintln!(
                "release-capture: {label} line={} note={} release_rms={value:.5}",
                line + 1,
                index + 1,
            );
            values.push(value);
        }
    }
    values
}
