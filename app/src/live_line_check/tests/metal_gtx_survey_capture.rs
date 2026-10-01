//! METAL-GTX の未採用の KS と CC が、行の LIVE 演奏経路で実際に音を変えるかを測る調査。
//!
//! 条件ごとに server を起こし直して（round robin の位置を揃える）1 行を鳴らし、対照の行と
//! 同じ窓の正規化相関と RMS を比べる。対照は 2 回録り、対照どうしの相関（`amp_random` などの
//! 揺らぎ）を基準として並べる。assert はせず表を出すだけ。環境変数は [`super::capture_support`] と同じ
//! （音色は Full を渡す）。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib metal_gtx_survey -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;

use cmrt_chord::TimedMidiEvent;

use super::capture_support::{rms, CaptureSetup, Segment};
use super::metal_gtx_survey_probes::probes;

/// 行の最後に置く、どの region も見ない CC。行の長さを余韻の後まで延ばす。
const PAD_CC: u8 = 3;
const PAD_SECONDS: f64 = 3.0;

pub(super) const A2: u8 = 45;
pub(super) const C3: u8 = 48;

#[derive(Clone, Default)]
pub(super) struct Line {
    events: Vec<TimedMidiEvent>,
}

impl Line {
    pub(super) fn cc(mut self, controller: u8, value: u8) -> Self {
        self.push(0.0, [0xB0, controller, value]);
        self
    }

    /// KS を `seconds` に押して 0.05 秒で離す（`sw_last` はラッチなので長さは効かない）。
    pub(super) fn ks_at(mut self, seconds: f64, key: u8) -> Self {
        self.push(seconds + 0.001, [0x90, key, 127]);
        self.push(seconds + 0.05, [0x80, key, 0]);
        self
    }

    pub(super) fn ks(self, key: u8) -> Self {
        self.ks_at(0.0, key)
    }

    pub(super) fn note(mut self, on: f64, off: f64, key: u8, velocity: u8) -> Self {
        self.push(on + 0.002, [0x90, key, velocity]);
        self.push(off, [0x80, key, 0]);
        self
    }

    /// pitch bend（-8192〜8191）。
    pub(super) fn bend_at(mut self, seconds: f64, value: i32) -> Self {
        let raw = (value + 8192).clamp(0, 16383) as u16;
        self.push(seconds, [0xE0, (raw & 0x7F) as u8, (raw >> 7) as u8]);
        self
    }

    fn push(&mut self, seconds: f64, message: [u8; 3]) {
        self.events.push(TimedMidiEvent { seconds, message });
    }

    fn build(mut self) -> Vec<TimedMidiEvent> {
        self.push(PAD_SECONDS, [0xB0, PAD_CC, 0]);
        self.events
            .sort_by(|a, b| a.seconds.partial_cmp(&b.seconds).unwrap());
        self.events
    }
}

pub(super) fn sus(key: u8, seconds: f64) -> Line {
    Line::default().note(0.0, seconds, key, 100)
}

pub(super) struct Probe {
    label: &'static str,
    control: &'static str,
    line: Line,
    /// 比べる窓（onset からの秒）。
    from: f64,
    len: f64,
}

pub(super) fn probe(
    label: &'static str,
    control: &'static str,
    line: Line,
    from: f64,
    len: f64,
) -> Probe {
    Probe {
        label,
        control,
        line,
        from,
        len,
    }
}

fn controls() -> BTreeMap<&'static str, Line> {
    let two = |ks: u8| {
        Line::default()
            .ks(ks)
            .note(0.0, 0.5, A2, 100)
            .note(0.5, 1.0, A2, 100)
    };
    BTreeMap::from([
        ("silence", Line::default()),
        ("sus-a1", sus(33, 1.0)),
        ("sus-f#2", sus(42, 1.0)),
        ("sus-f#3", Line::default().note(0.0, 1.0, 54, 90)),
        ("sus-e4", sus(64, 1.0)),
        ("sus-a2", sus(A2, 1.0)),
        ("sus-a2-long", sus(A2, 2.0)),
        ("sus-a2-v120", Line::default().note(0.0, 1.0, A2, 120)),
        ("sus-c5-long", sus(72, 2.0)),
        ("sus-a2-x2", two(17)),
        ("mute-a2", Line::default().ks(20).note(0.0, 1.0, A2, 100)),
        ("mute-a2-x2", two(20)),
        (
            "legato-a2-c3",
            Line::default()
                .note(0.0, 0.5, A2, 100)
                .note(0.5, 1.0, C3, 100),
        ),
        (
            "sus-a2-bend",
            Line::default().note(0.0, 1.0, A2, 100).bend_at(0.3, 8191),
        ),
        ("vib-a2", sus(A2, 2.0).cc(20, 100)),
        ("rel-none", sus(A2, 0.5).cc(24, 1).cc(25, 127)),
        ("rel-f5-none", sus(77, 0.5).cc(24, 1).cc(25, 127)),
        (
            "rel-mute-none",
            Line::default()
                .ks(20)
                .note(0.0, 0.5, A2, 100)
                .cc(24, 1)
                .cc(25, 127),
        ),
        (
            "slide-in-cc27-8",
            Line::default().ks(27).note(0.0, 1.0, A2, 100).cc(27, 8),
        ),
        (
            "trill-ht-cc28-0",
            Line::default().ks(99).note(0.0, 2.0, A2, 100).cc(28, 0),
        ),
        (
            "unison-auto-cc28-0",
            Line::default().ks(94).note(0.0, 2.0, 72, 100).cc(28, 0),
        ),
        (
            "trill-ht-cc112-0",
            Line::default().ks(99).note(0.0, 2.0, A2, 100).cc(112, 0),
        ),
        (
            "bend-ht-cc52-0",
            Line::default().ks(91).note(0.0, 2.0, A2, 100).cc(52, 0),
        ),
        (
            "bend-ht-cc53-0",
            Line::default().ks(91).note(0.0, 2.0, A2, 100).cc(53, 0),
        ),
        ("f5-rep-cc31-0", sus(77, 0.5).cc(24, 104).cc(31, 0)),
        ("stop-rep-none", sus(A2, 0.3).cc(24, 104).cc(25, 127)),
        (
            "mute-fret-cc29-0",
            Line::default().ks(14).note(0.0, 1.0, A2, 100).cc(29, 0),
        ),
    ])
}

fn db(value: f64) -> f64 {
    20.0 * value.max(1e-9).log10()
}

fn window_rms(segment: &Segment, from: f64, len: f64) -> f64 {
    rms(segment.window(from, len))
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH、Full）"]
fn survey_metal_gtx_keyswitches_and_controls() {
    let Some(setup) = CaptureSetup::from_env("metal-gtx-survey") else {
        return;
    };
    let only = std::env::var("CMRT_TEST_SURVEY_ONLY").ok();
    let probes = probes()
        .into_iter()
        .filter(|p| only.as_deref().is_none_or(|only| p.label.contains(only)))
        .collect::<Vec<_>>();
    let controls = controls();
    let mut recorded: BTreeMap<&str, (Segment, Segment)> = BTreeMap::new();
    let mut rows = Vec::new();
    for p in &probes {
        if !recorded.contains_key(p.control) {
            let events = controls[p.control].clone().build();
            let a = setup.record(
                &format!("control-{}-a", p.control),
                std::slice::from_ref(&events),
            );
            let b = setup.record(&format!("control-{}-b", p.control), &[events]);
            recorded.insert(
                p.control,
                (a.into_iter().next().unwrap(), b.into_iter().next().unwrap()),
            );
        }
        let segment = setup
            .record(p.label, &[p.line.clone().build()])
            .into_iter()
            .next()
            .unwrap();
        let (a, b) = &recorded[p.control];
        let base_ncc = a.correlation(b, p.from, p.len);
        let ncc = a.correlation(&segment, p.from, p.len);
        let base_rms = window_rms(a, p.from, p.len);
        let probe_rms = window_rms(&segment, p.from, p.len);
        let row = format!(
            "{:34} vs {:20} ncc={ncc:.4} (base {base_ncc:.4}) rms={:7.1}dB (ctrl {:7.1}dB, d={:+5.1}dB)",
            p.label,
            p.control,
            db(probe_rms),
            db(base_rms),
            db(probe_rms) - db(base_rms),
        );
        eprintln!("metal-gtx-survey: {row}");
        rows.push(row);
    }
    drop(setup);
    eprintln!("metal-gtx-survey: ---- summary ----");
    for row in rows {
        eprintln!("metal-gtx-survey: {row}");
    }
}
