//! Guitar Articulation 画面の 1 音モードで鳴らす列（[`column_events`]）が、行の LIVE 演奏経路で
//! 1 音だけ鳴り、その列の奏法の KS が効くかを、live mix の出力で確かめる。
//!
//! - a: 列 1 に H/P を ON にした表の列 1
//! - b: ルール無しの列 1
//! - c: 同じ server でルール無しのフレーズ全体を鳴らした後に、a と同じ列 1
//!
//! 音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib single_note_capture -- --ignored --nocapture
//! ```

use cmrt_guitar_articulation::{
    articulate, column_events, convert, notes_from_events, Rule, RuleTable, Take,
};

use super::capture_support::{peak, CaptureSetup, Segment, DIFFERENT, SAME};

const MML: &str = "o3 l8 e f+ g";
const COLUMN: usize = 1;
/// 1 音の頭から比べる長さ。l8（0.25 秒）の音と余韻の頭。
const COMPARE_SECONDS: f64 = 0.5;
/// これを超える peak があれば鳴っている（-40 dBFS）。
const SILENCE_PEAK: f64 = 0.01;
/// 音の立ち上がりとみなす包絡の高さ（区間の peak に対する比）。
const ONSET_RATIO: f64 = 0.3;
/// 直前 [`LOOKBACK_SECONDS`] の包絡の最小からこの倍率を超えて上がったら立ち上がり。
const RISE: f64 = 2.0;
/// 立ち上がりの比べ先を探す幅。立ち上がりを 1 つ数えた後、同じ幅は数えない
/// （ピックの雑音と本体の立ち上がりを 2 つに数えないため）。
const LOOKBACK_SECONDS: f64 = 0.1;
/// 包絡を取る窓の長さ。
const ENVELOPE_SECONDS: f64 = 0.020;

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn a_single_column_sounds_once_with_its_own_keyswitch() {
    let Some(setup) = CaptureSetup::from_env("single-note-capture") else {
        return;
    };
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let mut hammer = RuleTable::default();
    hammer.toggle(COLUMN, Rule::HammerPull);
    let plain = RuleTable::default();
    let note = |rules: &RuleTable| {
        let notes = notes_from_events(&raw);
        let articulated = articulate(&notes, rules);
        column_events(&notes, &articulated, rules, COLUMN, Take::Converted)
    };
    let hammer_note = note(&hammer);
    let plain_note = note(&plain);
    let whole_plain = convert(&raw, &plain);

    let a = setup.record("a-hammer-note", std::slice::from_ref(&hammer_note));
    let b = setup.record("b-plain-note", std::slice::from_ref(&plain_note));
    let c = setup.record("c-whole-then-hammer-note", &[whole_plain, hammer_note]);
    drop(setup);

    let a_onsets = describe("a", &a[0]);
    let b_onsets = describe("b", &b[0]);
    describe("c(2 行目)", &c[1]);
    let a_b = a[0].correlation(&b[0], 0.0, COMPARE_SECONDS);
    let a_c = a[0].correlation(&c[1], 0.0, COMPARE_SECONDS);
    eprintln!("single-note-capture: a vs b ncc={a_b:.4}");
    eprintln!("single-note-capture: a vs c(2 行目) ncc={a_c:.4}");

    assert_eq!(a_onsets, 1, "H/P の 1 音で立ち上がりが 1 つでない");
    assert_eq!(b_onsets, 1, "既定の 1 音で立ち上がりが 1 つでない");
    assert!(
        peak(&a[0].samples) > SILENCE_PEAK,
        "H/P の 1 音が鳴っていない"
    );
    assert!(
        peak(&b[0].samples) > SILENCE_PEAK,
        "既定の 1 音が鳴っていない"
    );
    assert!(
        a_b < DIFFERENT,
        "H/P の 1 音が既定の 1 音と同じ音（1 音だけだと KS が効いていない）"
    );
    assert!(
        a_c > SAME,
        "前にフレーズ全体を鳴らすと 1 音が変わる（前の演奏の KS のラッチに左右されている）"
    );
}

/// 立ち上がりの数・peak・頭から peak の -40 dB へ落ちきるまでの秒を出し、立ち上がりの数を返す。
fn describe(label: &str, segment: &Segment) -> usize {
    let envelope = envelope(segment);
    let top = envelope.iter().copied().fold(0.0, f64::max);
    let onsets = onset_count(&envelope, top);
    let last_loud = envelope.iter().rposition(|level| *level > top * 0.01);
    let first_loud = envelope.iter().position(|level| *level > top * 0.01);
    let audible_seconds = match (first_loud, last_loud) {
        (Some(first), Some(last)) => (last + 1 - first) as f64 * ENVELOPE_SECONDS,
        _ => 0.0,
    };
    eprintln!(
        "single-note-capture: {label} onsets={onsets} peak={top:.4} audible_seconds={audible_seconds:.3}"
    );
    onsets
}

/// [`ENVELOPE_SECONDS`] ごとの絶対値の peak。
fn envelope(segment: &Segment) -> Vec<f64> {
    let block = ((ENVELOPE_SECONDS * f64::from(segment.sample_rate)) as usize).max(1);
    segment.samples.chunks(block).map(peak).collect()
}

/// 立ち上がりの数。前の音が鳴り続けたまま弱く入るレガートの音（H/P の 2 音目以降）は数えない。
fn onset_count(envelope: &[f64], top: f64) -> usize {
    let lookback = (LOOKBACK_SECONDS / ENVELOPE_SECONDS).round() as usize;
    let mut last: Option<usize> = None;
    let mut count = 0;
    for (index, level) in envelope.iter().enumerate() {
        let floor = envelope[index.saturating_sub(lookback)..index]
            .iter()
            .copied()
            .fold(f64::MAX, f64::min)
            .min(*level);
        let rested = last.is_none_or(|last| index - last > lookback);
        if rested && *level > top * ONSET_RATIO && *level > floor * RISE {
            count += 1;
            last = Some(index);
        }
    }
    count
}
