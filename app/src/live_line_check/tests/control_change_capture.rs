//! Guitar Articulation の、CC を送る重ねられる列ルール（ロング/強め・パワーコード・ポジション移動の離し音）を
//! ON にすると、行の LIVE 演奏経路で実際に音が変わるかを、live mix の出力で確かめる。
//!
//! - ロング/強め（CC23）: Sus_Down の velocity 100（Sus_LT）と 127（Sus_EX）、パームミュート（Mute_EX）で、
//!   2 音目が同じ MML・同じ velocity で CC23 を送らない基準と違う音か（波形の相関）。
//! - パワーコード（CC32）: 2 音目が基準と違う音か。
//! - ポジション移動の離し音（CC24 = 72）: 1 音を離した後の窓の RMS が、同じ行の CC24 を 1（リリース音なし）に
//!   差し替えた行より大きいか。
//!
//! 音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib control_change_capture -- --ignored --nocapture
//! ```

use cmrt_chord::TimedMidiEvent;
use cmrt_guitar_articulation::{convert, Rule, RuleTable, POSITION_RELEASE_VALUE};

use super::capture_support::{rms, CaptureSetup, Segment, DIFFERENT, SAME};

/// E2 → G2 → A2。2 音目（1 秒）の列にルールを掛ける。
const MML: &str = "o3 l4 e g2 a";
const COLUMN: usize = 1;
/// 各音の頭の秒と長さ（`MML` のテンポ 120 の値）。
const NOTES: [(f64, f64); 3] = [(0.0, 0.5), (0.5, 1.0), (1.5, 0.5)];
/// 比べる窓を次の音の頭より手前で切る幅。
const NEXT_NOTE_GUARD_SECONDS: f64 = 0.020;
/// ミュートは短いので、2 音目の頭のこの長さだけを比べる。
const MUTE_SECONDS: f64 = 0.25;
/// onset を探し直すときに最大振幅を取る、1 音目の頭からの長さ（2 音目の頭を含まない）。
const HEAD_ONSET_SECONDS: f64 = 0.3;
/// Sus_Down + CC23 で Sus_LT を選ぶ velocity（111 以下）。MML の既定 127 は Sus_EX。
const SUS_LT_VELOCITY: u8 = 100;
/// G2 を 4 分で 1 つ。離した後の窓に次の音が入らない。
const RELEASE_MML: &str = "o3 l4 g";
/// 離した後の窓（note off からの秒）。
const RELEASE_FROM_SECONDS: f64 = 0.010;
const RELEASE_TO_SECONDS: f64 = 0.300;
/// リリース音を鳴らさない CC24 の値（0〜1 の帯）。
const NO_RELEASE_VALUE: u8 = 1;
const RELEASE_SHAPE_CC: u8 = 24;

/// (label, 2 音目の列に ON にするルール, 基準にも ON にするルール, velocity, 比べる長さ)。
type Condition = (
    &'static str,
    &'static [Rule],
    &'static [Rule],
    Option<u8>,
    f64,
);

const CONDITIONS: [Condition; 4] = [
    (
        "sus-lt",
        &[Rule::LongExtra],
        &[],
        Some(SUS_LT_VELOCITY),
        1.0,
    ),
    ("sus-ex", &[Rule::LongExtra], &[], None, 1.0),
    (
        "mute-ex",
        &[Rule::PalmMute, Rule::LongExtra],
        &[Rule::PalmMute],
        None,
        MUTE_SECONDS,
    ),
    ("power-chord", &[Rule::PowerChord], &[], None, 1.0),
];

/// `mml` の `column` 列に `rules` を ON にして変換する。`velocity` があれば MML の note on の velocity を差し替える。
fn phrase(mml: &str, column: usize, rules: &[Rule], velocity: Option<u8>) -> Vec<TimedMidiEvent> {
    let mut raw = cmrt_chord::timed_performance(mml)
        .expect("MML を解釈できない")
        .events;
    if let Some(velocity) = velocity {
        for event in &mut raw {
            if event.message[0] & 0xF0 == 0x90 && event.message[2] != 0 {
                event.message[2] = velocity;
            }
        }
    }
    let mut table = RuleTable::default();
    for &rule in rules {
        table.toggle(column, rule);
    }
    convert(&raw, &table)
}

fn record(setup: &CaptureSetup, label: &str, events: Vec<TimedMidiEvent>) -> Segment {
    setup
        .record(label, &[events])
        .remove(0)
        .with_head_onset(HEAD_ONSET_SECONDS)
}

/// 音 `index` の頭から `seconds`（音の長さで切る）の相関。
fn note_ncc(label: &str, left: &Segment, right: &Segment, index: usize, seconds: f64) -> f64 {
    let (at, length) = NOTES[index];
    let seconds = seconds.min(length - NEXT_NOTE_GUARD_SECONDS);
    let ncc = left.correlation(right, at, seconds);
    eprintln!(
        "control-change-capture: {label} note={} ncc={ncc:.4} rms={:.4}/{:.4}",
        index + 1,
        rms(left.window(at, seconds)),
        rms(right.window(at, seconds)),
    );
    ncc
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn long_extra_and_power_chord_change_the_sound_of_their_column() {
    let Some(setup) = CaptureSetup::from_env("control-change-capture") else {
        return;
    };
    let recorded = CONDITIONS
        .iter()
        .map(|&(label, rules, base_rules, velocity, seconds)| {
            let base_label = format!("{label}-base");
            let base = record(
                &setup,
                &base_label,
                phrase(MML, COLUMN, base_rules, velocity),
            );
            let target = record(&setup, label, phrase(MML, COLUMN, rules, velocity));
            (label, seconds, base, target)
        })
        .collect::<Vec<_>>();
    drop(setup);

    let mut failures = Vec::new();
    for (label, seconds, base, target) in &recorded {
        let head = note_ncc(label, base, target, 0, 1.0);
        if head <= SAME {
            failures.push(format!("{label}: 1 音目が基準と違う（ncc={head:.4}）"));
        }
        let ncc = note_ncc(label, base, target, COLUMN, *seconds);
        if ncc >= DIFFERENT {
            failures.push(format!("{label}: 2 音目が基準と同じ音（ncc={ncc:.4}）"));
        }
    }
    let (_, _, _, lt) = &recorded[0];
    let (_, _, _, ex) = &recorded[1];
    note_ncc("sus-lt vs sus-ex", lt, ex, COLUMN, 1.0);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// ポジション移動の離し音が、リリース音なし（CC24 = 1）より大きく鳴るか。
#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn position_release_sounds_after_the_note_off() {
    let Some(setup) = CaptureSetup::from_env("control-change-capture") else {
        return;
    };
    let position = phrase(RELEASE_MML, 0, &[Rule::PositionRelease], None);
    let silent: Vec<TimedMidiEvent> = position
        .iter()
        .map(|event| {
            let mut event = *event;
            if event.message[0] & 0xF0 == 0xB0
                && event.message[1] == RELEASE_SHAPE_CC
                && event.message[2] == POSITION_RELEASE_VALUE
            {
                event.message[2] = NO_RELEASE_VALUE;
            }
            event
        })
        .collect();
    assert_ne!(position, silent, "CC24 = 72 を送っていない");
    let off = NOTES[0].1;
    let window = |segment: &Segment| {
        rms(segment.window(
            off + RELEASE_FROM_SECONDS,
            RELEASE_TO_SECONDS - RELEASE_FROM_SECONDS,
        ))
    };
    let with = window(&record(&setup, "position-release", position));
    let without = window(&record(&setup, "no-release", silent));
    drop(setup);
    eprintln!("control-change-capture: release_rms cc24=72 {with:.5} cc24=1 {without:.5}");
    assert!(
        with > without,
        "CC24=72 の離した後の RMS {with:.5} が CC24=1 の {without:.5} 以下"
    );
}
