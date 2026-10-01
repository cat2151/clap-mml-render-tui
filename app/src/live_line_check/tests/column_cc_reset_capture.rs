//! Guitar Articulation の、列で CC を送るルール（ビブラート CC20 / パワーコード CC32）を ON にした行を、
//! 最後の note off より前に同じ行の OFF で上書きする経路を、1 つの server の行の LIVE 演奏経路で
//! live mix に録り、上書きした OFF の行が鳴っている（無音でない）ことを確かめる。
//!
//! OFF の行に前の行の CC が残らないかは、録音どうしを比べては確かめない。METAL-GTX のラウンドロビンで、
//! 同じ行を server ごとに録り直しても相関が揺れるため。頭で列 CC の既定値を送ることは
//! `cmrt-guitar-articulation` の unit test（`control::tests::head_defaults`）で確かめる。
//!
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib column_cc_reset_capture -- --ignored --nocapture --test-threads=1
//! ```

use std::time::Duration;

use cmrt_chord::TimedMidiEvent;
use cmrt_guitar_articulation::{convert, Rule, RuleTable};
use cmrt_mml_overlay::LivePatch;

use super::capture_support::{peak, send_line, CaptureSetup, Segment, STEP_MS};
use super::{live_line, sleep_until};

/// E2 → G2 → A2。2 音目（0.5〜1.5 秒）の列にルールを掛ける。
const MML: &str = "o3 l4 e g2 a";
const COLUMN: usize = 1;
/// 各音の頭の秒と長さ（`MML` のテンポ 120 の値）。
const NOTES: [(f64, f64); 3] = [(0.0, 0.5), (0.5, 1.0), (1.5, 0.5)];
/// 1 本目を送ってから 2 本目で上書きするまで。ルールの列が鳴っている間で、最後の note off（2 秒）の前。
const OVERWRITE_AFTER: Duration = Duration::from_millis(1_000);
/// 2 本目で鳴っていることを見る音（`NOTES` の位置）。1 音目は上書きで離された 1 本目の余韻が重なるので見ない。
const CHECKED_NOTES: [usize; 2] = [1, 2];
/// onset を探し直すときに最大振幅を取る、1 音目の頭からの長さ（2 音目の頭を含まない）。
const HEAD_ONSET_SECONDS: f64 = 0.3;
/// これを超える peak があれば鳴っている（-40 dBFS）。
const SILENCE_PEAK: f64 = 0.01;

/// `MML` の `COLUMN` 列に `rules` を ON にして変換する。
fn phrase(rules: &[Rule]) -> Vec<TimedMidiEvent> {
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let mut table = RuleTable::default();
    for &rule in rules {
        table.toggle(COLUMN, rule);
    }
    convert(&raw, &table)
}

/// server を起こし直して `first` を送り、[`OVERWRITE_AFTER`] 後に `second` で上書きして、2 本の録音を返す。
fn overwrite(
    setup: &CaptureSetup,
    label: &str,
    first: Vec<TimedMidiEvent>,
    second: Vec<TimedMidiEvent>,
) -> Vec<Segment> {
    let lines = [first, second]
        .into_iter()
        .map(|events| {
            let source = format!(r#"{{"Surge XT patch": "{}"}} c"#, setup.patch());
            let mut live = live_line(&source).expect("行を作れない");
            live.program.performance.loop_seconds =
                events.last().map_or(0.0, |event| event.seconds);
            live.program.performance.events = events;
            live
        })
        .collect::<Vec<_>>();
    let patch: LivePatch = lines[0].patch.clone();
    setup
        .record_session(
            label,
            &patch,
            super::capture_support::LIVE_CAPTURE_SECONDS,
            |sender| {
                let first = send_line(sender, &lines[0].patch, &lines[0].program);
                sleep_until(first.started + OVERWRITE_AFTER);
                let second = send_line(sender, &lines[1].patch, &lines[1].program);
                sleep_until(second.started + Duration::from_millis(STEP_MS));
                vec![first, second]
            },
        )
        .into_iter()
        .map(|segment| segment.with_head_onset(HEAD_ONSET_SECONDS))
        .collect()
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn a_line_overwritten_by_its_off_line_before_the_last_note_off_still_sounds() {
    let Some(setup) = CaptureSetup::from_env("column-cc-reset-capture") else {
        return;
    };
    let off = phrase(&[]);
    let recorded = [
        ("vibrato", Rule::Vibrato),
        ("power-chord", Rule::PowerChord),
    ]
    .into_iter()
    .map(|(label, rule)| {
        (
            label,
            overwrite(&setup, label, phrase(&[rule]), off.clone()),
        )
    })
    .collect::<Vec<_>>();
    drop(setup);

    let mut failures = Vec::new();
    for (label, segments) in &recorded {
        for index in CHECKED_NOTES {
            let (at, seconds) = NOTES[index];
            let peak = peak(segments[1].window(at, seconds));
            eprintln!(
                "column-cc-reset-capture: {label} second-line note={} peak={peak:.4}",
                index + 1
            );
            if peak <= SILENCE_PEAK {
                failures.push(format!(
                    "{label}: 上書きした OFF の行の {} 音目が鳴っていない（peak={peak:.4}）",
                    index + 1
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
