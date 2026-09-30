//! Guitar Articulation の列ルール（ミュート・PH・スライド・チョーキング・ビブラート）を ON にすると、
//! 行の LIVE 演奏経路で実際に音が変わるかを、live mix の出力で確かめる。
//!
//! 同じ 3 音を「全ルール OFF」と「2 音目の列にだけ 1 つのルール」で [`convert`] して鳴らし、
//! 音ごとの波形の相関を比べる。音色の round robin の位置を揃えるため、条件ごとに server を起こし直す。
//! ビブラートは相関に加えて、2 音目の周期（自己相関で測る）の揺れが基準より大きいことも見る。
//! 環境変数は [`super::capture_support`] と同じ。
//!
//! ```text
//! cargo test -p clap-mml-render-tui --lib articulation_capture -- --ignored --nocapture
//! ```

use cmrt_guitar_articulation::{convert, Rule, RuleTable};

use super::capture_support::{peak, rms, CaptureSetup, Segment, DIFFERENT, SAME};

/// E2 → G2（+3 半音。スライドは 3 半音、チョーキングは 1 音半）→ A2。
/// 2 音目はビブラートの揺れが見える長さ（1 秒）にする。
const MML: &str = "o3 l4 e g2 a";
const COLUMN: usize = 1;
/// 各音の頭の秒と長さ（`MML` のテンポ 120 の値）。
const NOTES: [(f64, f64); 3] = [(0.0, 0.5), (0.5, 1.0), (1.5, 0.5)];
/// 比べる窓を次の音の頭より手前で切る幅。onset は音の頭より数 ms 遅れて見つかるので、
/// 音の長さいっぱいに取ると次の音（レガートのスライドは頭から大きい）の頭が混ざる。
const NEXT_NOTE_GUARD_SECONDS: f64 = 0.020;
/// 周期の揺れを測る 2 音目の区間。頭の立ち上がりを外す。
const PITCH_FROM_SECONDS: f64 = 0.15;
const PITCH_TO_SECONDS: f64 = 1.0;
/// 周期を 1 つ測る窓とその送り。
const PITCH_BLOCK_SECONDS: f64 = 0.040;
const PITCH_HOP_SECONDS: f64 = 0.020;
/// 周期を探す基本周波数の範囲。G2（98 Hz）を挟む。
const PITCH_MIN_HZ: f64 = 60.0;
const PITCH_MAX_HZ: f64 = 200.0;
/// 平均周期のずれを報告する閾値（音程が上がっていないか。assert はしない）。
const PITCH_SHIFT_REPORT: f64 = 0.03;

const CONDITIONS: [(&str, Rule); 5] = [
    ("palm-mute", Rule::PalmMute),
    ("pinch-harmonic", Rule::PinchHarmonic),
    ("slide", Rule::Slide),
    ("choke", Rule::Choke),
    ("vibrato", Rule::Vibrato),
];

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn each_articulation_rule_changes_the_sound_of_its_column() {
    let Some(setup) = CaptureSetup::from_env("articulation-capture") else {
        return;
    };
    let raw = cmrt_chord::timed_performance(MML)
        .expect("MML を解釈できない")
        .events;
    let phrase = |rule: Option<Rule>| {
        let mut rules = RuleTable::default();
        if let Some(rule) = rule {
            rules.toggle(COLUMN, rule);
        }
        convert(&raw, &rules)
    };
    let plain = setup.record("plain", &[phrase(None)]);
    let recorded = CONDITIONS
        .iter()
        .map(|(label, rule)| (*label, *rule, setup.record(label, &[phrase(Some(*rule))])))
        .collect::<Vec<_>>();
    drop(setup);

    let (plain_spread, plain_mean) = period_stats("plain", &plain[0]);
    let mut failures = Vec::new();
    for (label, rule, segments) in &recorded {
        let ncc = compare(label, &plain[0], &segments[0]);
        if ncc[0] <= SAME {
            failures.push(format!("{label}: 1 音目が基準と違う（ncc={:.4}）", ncc[0]));
        }
        if ncc[1] >= DIFFERENT {
            failures.push(format!(
                "{label}: 2 音目が基準と同じ音（ncc={:.4}）",
                ncc[1]
            ));
        }
        if *rule == Rule::Vibrato {
            let (spread, mean) = period_stats(label, &segments[0]);
            if spread <= plain_spread {
                failures.push(format!(
                    "{label}: 周期の揺れ {spread:.5} が基準 {plain_spread:.5} 以下"
                ));
            }
            let shift = mean / plain_mean - 1.0;
            let note = if shift.abs() >= PITCH_SHIFT_REPORT {
                "（3% 以上ずれた）"
            } else {
                ""
            };
            eprintln!("articulation-capture: {label} mean_period_shift={shift:+.4}{note}");
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// 音ごとに、ずれを探した最大の正規化相関を出して返す。RMS と頭 30ms の peak も並べる。
fn compare(label: &str, left: &Segment, right: &Segment) -> Vec<f64> {
    NOTES
        .iter()
        .enumerate()
        .map(|(index, &(at, length))| {
            let seconds = length - NEXT_NOTE_GUARD_SECONDS;
            let ncc = left.correlation(right, at, seconds);
            let (l, r) = (left.window(at, seconds), right.window(at, seconds));
            eprintln!(
                "articulation-capture: plain vs {label} note={} ncc={ncc:.4} rms={:.4}/{:.4} head_peak={:.4}/{:.4}",
                index + 1,
                rms(l),
                rms(r),
                peak(left.window(at, 0.030)),
                peak(right.window(at, 0.030)),
            );
            ncc
        })
        .collect()
}

/// 2 音目の周期（秒）を窓ごとに測り、平均で割った標準偏差（揺れ）と平均を返す。
fn period_stats(label: &str, segment: &Segment) -> (f64, f64) {
    let (at, _) = NOTES[COLUMN];
    let samples = segment.window(
        at + PITCH_FROM_SECONDS,
        PITCH_TO_SECONDS - PITCH_FROM_SECONDS,
    );
    let rate = f64::from(segment.sample_rate);
    let block = (PITCH_BLOCK_SECONDS * rate) as usize;
    let hop = (PITCH_HOP_SECONDS * rate) as usize;
    let periods = (0..samples.len().saturating_sub(block))
        .step_by(hop.max(1))
        .filter_map(|start| period(&samples[start..start + block], rate))
        .collect::<Vec<_>>();
    assert!(!periods.is_empty(), "{label}: 周期を 1 つも測れない");
    let mean = periods.iter().sum::<f64>() / periods.len() as f64;
    let variance = periods.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / periods.len() as f64;
    let spread = variance.sqrt() / mean;
    eprintln!(
        "articulation-capture: {label} periods={} mean_hz={:.2} spread={spread:.5}",
        periods.len(),
        1.0 / mean
    );
    (spread, mean)
}

/// 窓の自己相関が最大になる遅れ（放物線で補間）を周期の秒として返す。無音なら `None`。
fn period(block: &[f32], rate: f64) -> Option<f64> {
    let min_lag = (rate / PITCH_MAX_HZ) as usize;
    let max_lag = ((rate / PITCH_MIN_HZ) as usize).min(block.len() / 2);
    let score = |lag: usize| {
        let (head, tail) = (&block[..block.len() - lag], &block[lag..]);
        let (mut dot, mut hh, mut tt) = (0.0_f64, 0.0_f64, 0.0_f64);
        for (h, t) in head.iter().zip(tail) {
            let (h, t) = (f64::from(*h), f64::from(*t));
            dot += h * t;
            hh += h * h;
            tt += t * t;
        }
        if hh == 0.0 || tt == 0.0 {
            0.0
        } else {
            dot / (hh * tt).sqrt()
        }
    };
    let scores = (min_lag..=max_lag).map(score).collect::<Vec<_>>();
    let (best, top) = scores
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    if top <= 0.0 {
        return None;
    }
    let shift = match (best.checked_sub(1), scores.get(best + 1)) {
        (Some(before), Some(after)) => {
            let (l, r) = (scores[before], *after);
            let denominator = l - 2.0 * top + r;
            if denominator == 0.0 {
                0.0
            } else {
                0.5 * (l - r) / denominator
            }
        }
        _ => 0.0,
    };
    Some((min_lag as f64 + best as f64 + shift) / rate)
}
