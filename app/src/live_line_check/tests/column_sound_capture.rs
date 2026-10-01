//! Guitar Articulation の、列の最低音 1 つだけを鳴らす列ルール（クロマチックラン・スライドエフェクト・効果音）を
//! ON にすると、行の LIVE 演奏経路で音が出るかを、live mix の出力で確かめる。
//!
//! 1 音だけの MML にルールを 1 つ掛けて [`convert`] し、無音から鳴らして RMS を測る。
//! 対照に KS だけの行（音の出ない行）も録り、閾値がその RMS より上にあることを見る。
//! スライドエフェクト Down は velocity の 3 層が互いに違う音か（波形の相関）も見る。
//! 条件ごとに server を起こし直す。環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib column_sound_capture -- --ignored --nocapture
//! ```

use cmrt_chord::TimedMidiEvent;
use cmrt_guitar_articulation::{convert, Rule, RuleTable};

use super::capture_support::{rms, CaptureSetup, Segment, DIFFERENT};

/// G2（43）を 1 秒。どのルールでも音域の中（クロマチックランは 31 へ畳む）。
const MML: &str = "o3 l2 g";
/// F#3（54）を 1 秒。スライドエフェクト Down の音域（42〜66）の中で、畳まずに鳴る。
const SLIDE_FX_DOWN_MML: &str = "o4 l2 f+";
/// 測る長さ（音の長さ）。
const SECONDS: f64 = 1.0;
/// 鳴っているとみなす RMS の下限（dB）。
const AUDIBLE_DB: f64 = -80.0;
/// スライドエフェクト Down の 3 層（0〜80 / 81〜100 / 101〜127）から 1 つずつ選ぶ velocity。
const SLIDE_FX_DOWN_VELOCITIES: [u8; 3] = [70, 90, 120];

const CONDITIONS: [(&str, Rule); 8] = [
    ("chromatic-run", Rule::ChromaticRun),
    ("slide-fx-down", Rule::SlideFxDown),
    ("slide-fx-up", Rule::SlideFxUp),
    ("slide-fx-wow", Rule::SlideFxWow),
    ("fx-hello", Rule::EffectHello),
    ("fx-resonance", Rule::EffectResonance),
    ("fx-slide-noise", Rule::EffectSlideNoise),
    ("fx-hard-stop", Rule::EffectHardStop),
];

fn db(value: f64) -> f64 {
    20.0 * value.max(1e-9).log10()
}

/// `mml` の先頭の列に `rule` を ON にして変換する。`velocity` があれば MML の note on の velocity を差し替える。
fn phrase(mml: &str, rule: Rule, velocity: Option<u8>) -> Vec<TimedMidiEvent> {
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
    let mut rules = RuleTable::default();
    rules.toggle(0, rule);
    convert(&raw, &rules)
}

fn record(setup: &CaptureSetup, label: &str, events: Vec<TimedMidiEvent>) -> Segment {
    setup.record(label, &[events]).remove(0)
}

/// 既定の奏法の KS を押して離すだけの行。音は出ない。
fn keyswitch_only() -> Vec<TimedMidiEvent> {
    let key = cmrt_guitar_articulation::Articulation::SusDown.keyswitch();
    vec![
        TimedMidiEvent {
            seconds: 0.0,
            message: [0x90, key, 127],
        },
        TimedMidiEvent {
            seconds: SECONDS,
            message: [0x80, key, 0],
        },
    ]
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn each_column_sound_rule_is_audible_from_silence() {
    let Some(setup) = CaptureSetup::from_env("column-sound-capture") else {
        return;
    };
    let silent = record(&setup, "keyswitch-only", keyswitch_only());
    let recorded = CONDITIONS
        .iter()
        .map(|(label, rule)| (*label, record(&setup, label, phrase(MML, *rule, None))))
        .collect::<Vec<_>>();
    drop(setup);

    let mut failures = Vec::new();
    let floor = db(rms(silent.window(0.0, SECONDS)));
    eprintln!("column-sound-capture: keyswitch-only rms={floor:.1}dB");
    if floor >= AUDIBLE_DB {
        failures.push(format!(
            "対照（KS だけ）が {floor:.1}dB で、閾値 {AUDIBLE_DB}dB 以上"
        ));
    }
    for (label, segment) in &recorded {
        let level = db(rms(segment.window(0.0, SECONDS)));
        eprintln!("column-sound-capture: {label} rms={level:.1}dB");
        if level <= AUDIBLE_DB {
            failures.push(format!("{label}: {level:.1}dB で鳴っていない"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// スライドエフェクト Down は MML の velocity をそのまま使い、3 層がそれぞれ別の sample で鳴るか。
#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn slide_fx_down_velocity_layers_differ() {
    let Some(setup) = CaptureSetup::from_env("column-sound-capture") else {
        return;
    };
    let layers = SLIDE_FX_DOWN_VELOCITIES
        .iter()
        .map(|&velocity| {
            let label = format!("slide-fx-down-vel{velocity}");
            let events = phrase(SLIDE_FX_DOWN_MML, Rule::SlideFxDown, Some(velocity));
            (velocity, record(&setup, &label, events))
        })
        .collect::<Vec<_>>();
    drop(setup);

    let mut failures = Vec::new();
    for (i, (left_velocity, left)) in layers.iter().enumerate() {
        for (right_velocity, right) in &layers[i + 1..] {
            let ncc = left.correlation(right, 0.0, SECONDS);
            eprintln!(
                "column-sound-capture: slide-fx-down vel{left_velocity} vs vel{right_velocity} ncc={ncc:.4} rms={:.1}/{:.1}dB",
                db(rms(left.window(0.0, SECONDS))),
                db(rms(right.window(0.0, SECONDS))),
            );
            if ncc >= DIFFERENT {
                failures.push(format!(
                    "vel{left_velocity} と vel{right_velocity} が同じ音（ncc={ncc:.4}）"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
