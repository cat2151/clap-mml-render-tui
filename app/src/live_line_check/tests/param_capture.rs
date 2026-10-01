//! Guitar Articulation の行全体のパラメータ（`u`）を既定から動かすと、行の LIVE 演奏経路で実際に音が変わるかを、
//! live mix の出力で確かめる。同じ MML・同じ列ルールでパラメータを動かさない基準と比べる。
//!
//! - CC22（ミュートの長さ）= 0: パームミュートの音。長さが変わるだけで波形は似るので、RMS の差（dB）で見る。
//! - CC46（テンション）= 127: 伸ばした音。波形の相関で見る。
//! - CC112（トリルの速さ）= 127: 半音のトリル。波形の相関で見る。
//!
//! 音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib param_capture -- --ignored --nocapture
//! ```

use cmrt_chord::TimedMidiEvent;
use cmrt_guitar_articulation::{convert, param_of, Rule, RuleTable};

use super::capture_support::{rms, CaptureSetup, Segment, DIFFERENT};

/// A2（45）を 1 秒。
const ONE_SECOND_MML: &str = "o3 l2 a";
/// A2 を 2 秒。トリルが何度か往復する長さ。
const TWO_SECONDS_MML: &str = "o3 l1 a";
/// 音の終わりより手前で窓を切る幅。
const GUARD_SECONDS: f64 = 0.020;
/// 音量だけが変わる条件で、違う音とみなす RMS の差（dB）。
const DIFFERENT_DB: f64 = 3.0;

/// 基準との違いをどう測るか。
#[derive(Clone, Copy)]
enum Measure {
    Correlation,
    Level,
}

/// (label, MML, 先頭の列に ON にするルール, CC, 値, 測る長さ, 測り方)。
type Condition = (
    &'static str,
    &'static str,
    Option<Rule>,
    u8,
    u8,
    f64,
    Measure,
);

const CONDITIONS: [Condition; 3] = [
    (
        "mute-length-0",
        ONE_SECOND_MML,
        Some(Rule::PalmMute),
        22,
        0,
        1.0,
        Measure::Level,
    ),
    (
        "tension-127",
        ONE_SECOND_MML,
        None,
        46,
        127,
        1.0,
        Measure::Correlation,
    ),
    (
        "trill-speed-127",
        TWO_SECONDS_MML,
        Some(Rule::TrillHalf),
        112,
        127,
        2.0,
        Measure::Correlation,
    ),
];

fn db(value: f64) -> f64 {
    20.0 * value.max(1e-9).log10()
}

/// `mml` の先頭の列に `rule` を ON にし、`param` があればその CC を値へ動かして変換する。
fn phrase(mml: &str, rule: Option<Rule>, param: Option<(u8, u8)>) -> Vec<TimedMidiEvent> {
    let raw = cmrt_chord::timed_performance(mml)
        .expect("MML を解釈できない")
        .events;
    let mut rules = RuleTable::default();
    if let Some(rule) = rule {
        rules.toggle(0, rule);
    }
    if let Some((cc, value)) = param {
        let default = param_of(cc).expect("パラメータの表に無い CC").default;
        rules.step_param(cc, i16::from(value) - i16::from(default));
        assert_eq!(rules.param(cc), value);
    }
    let events = convert(&raw, &rules);
    if let Some((cc, value)) = param {
        assert!(
            events.iter().any(|e| e.message == [0xB0, cc, value]),
            "CC{cc}={value} を送っていない"
        );
    }
    events
}

fn record(setup: &CaptureSetup, label: &str, events: Vec<TimedMidiEvent>) -> Segment {
    setup.record(label, &[events]).remove(0)
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn changed_params_change_the_sound() {
    let Some(setup) = CaptureSetup::from_env("param-capture") else {
        return;
    };
    let recorded = CONDITIONS
        .iter()
        .map(|&(label, mml, rule, cc, value, seconds, measure)| {
            let base = record(&setup, &format!("{label}-base"), phrase(mml, rule, None));
            let target = record(&setup, label, phrase(mml, rule, Some((cc, value))));
            (label, seconds, measure, base, target)
        })
        .collect::<Vec<_>>();
    drop(setup);

    let mut failures = Vec::new();
    for (label, seconds, measure, base, target) in &recorded {
        let seconds = seconds - GUARD_SECONDS;
        let ncc = base.correlation(target, 0.0, seconds);
        let base_db = db(rms(base.window(0.0, seconds)));
        let target_db = db(rms(target.window(0.0, seconds)));
        let diff_db = target_db - base_db;
        eprintln!(
            "param-capture: {label} ncc={ncc:.4} rms={base_db:.1}/{target_db:.1}dB diff={diff_db:+.1}dB"
        );
        let differs = match measure {
            Measure::Correlation => ncc < DIFFERENT,
            Measure::Level => diff_db.abs() >= DIFFERENT_DB,
        };
        if !differs {
            failures.push(format!(
                "{label}: 基準と同じ音（ncc={ncc:.4}、差 {diff_db:+.1}dB）"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
