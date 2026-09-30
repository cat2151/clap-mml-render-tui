//! Guitar Articulation のピックスクレイプの列ルールを ON にすると、行の LIVE 演奏経路で
//! その列が擦る音（`Pick_Scratch`）に変わるかを、live mix の出力で確かめる。
//!
//! 同じ 3 音を「全ルール OFF」と「2 音目の列にだけピックスクレイプ」で [`convert`] して鳴らし、
//! 音ごとの波形の相関と、2 音目の RMS を比べる。
//! 2 音目は sample の範囲（30〜42）の外の音高なので、畳まずに KS だけ替えると無音になる。
//! RMS が基準の音以上なら、畳んで鳴らせている。
//! 条件ごとに server を起こし直す。環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib pick_scratch_capture -- --ignored --nocapture
//! ```

use cmrt_guitar_articulation::{convert, Rule, RuleTable};

use super::capture_support::{rms, CaptureSetup, DIFFERENT, SAME};

/// E2 → C4（60。ピックスクレイプでは 36 へ畳む）→ A2。
const MML: &str = "o3 l4 e o5 c o3 a";
const COLUMN: usize = 1;
const NOTE_SECONDS: f64 = 0.5;
/// 比べる窓を次の音の頭より手前で切る幅（onset は音の頭より数 ms 遅れて見つかる）。
const NEXT_NOTE_GUARD_SECONDS: f64 = 0.020;
/// 擦る音が鳴っているとみなす、基準の音に対する 2 音目の RMS の比の下限。
/// `Pick_Scratch` は velocity が効かず、Sus より大きく鳴る。
const LOUDER_THAN_PLAIN: f64 = 1.0;

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn pick_scratch_rule_turns_its_column_into_a_scrape() {
    let Some(setup) = CaptureSetup::from_env("pick-scratch-capture") else {
        return;
    };
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let mut rules = RuleTable::default();
    let plain = setup.record("plain", &[convert(&raw, &rules)]);
    rules.toggle(COLUMN, Rule::PickScratch);
    let scratch = setup.record("scratch", &[convert(&raw, &rules)]);
    drop(setup);

    let (plain, scratch) = (&plain[0], &scratch[0]);
    let seconds = NOTE_SECONDS - NEXT_NOTE_GUARD_SECONDS;
    let ncc: Vec<f64> = (0..3)
        .map(|index| plain.correlation(scratch, index as f64 * NOTE_SECONDS, seconds))
        .collect();
    let at = COLUMN as f64 * NOTE_SECONDS;
    let (plain_rms, scratch_rms) = (
        rms(plain.window(at, seconds)),
        rms(scratch.window(at, seconds)),
    );
    eprintln!("pick-scratch-capture: ncc={ncc:.4?} rms={plain_rms:.4}/{scratch_rms:.4}");

    let mut failures = Vec::new();
    if ncc[0] <= SAME {
        failures.push(format!("1 音目が基準と違う（ncc={:.4}）", ncc[0]));
    }
    if ncc[1] >= DIFFERENT {
        failures.push(format!("2 音目が基準と同じ音（ncc={:.4}）", ncc[1]));
    }
    if scratch_rms < plain_rms * LOUDER_THAN_PLAIN {
        failures.push(format!(
            "2 音目が擦る音の大きさで鳴っていない（rms={plain_rms:.4}/{scratch_rms:.4}）"
        ));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
