//! Guitar Articulation の列ルール（ミュート・PH・スライド・チョーキング・ビブラート・奏法リストのルール）を ON にすると、
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
/// [`MML`] の 2 オクターブ上。ユニゾンチョーキング（音域 C4〜C6）の 2 音目 G4 が音域に入る。
const HIGH_MML: &str = "o5 l4 e g2 a";
/// スライドインの幅 1（F#2 → G2）と幅 7（C2 → G2）。音の長さは [`MML`] と同じ。
const SLIDE_IN_NARROW_MML: &str = "o3 l4 f+ g2 a";
const SLIDE_IN_WIDE_MML: &str = "o3 l4 c g2 a";
const COLUMN: usize = 1;
const PITCH_BEND: u8 = 0xE0;
/// 各音の頭の秒と長さ（`MML` のテンポ 120 の値）。
const NOTES: [(f64, f64); 3] = [(0.0, 0.5), (0.5, 1.0), (1.5, 0.5)];
/// 比べる窓を次の音の頭より手前で切る幅。onset は音の頭より数 ms 遅れて見つかるので、
/// 音の長さいっぱいに取ると次の音（レガートのスライドは頭から大きい）の頭が混ざる。
const NEXT_NOTE_GUARD_SECONDS: f64 = 0.020;
/// onset を探し直すときに最大振幅を取る、1 音目の頭からの長さ（[`Segment::with_head_onset`]）。
/// 2 音目（0.5 秒）の頭を含まない長さ。トリルは 2 音目が 1 音目の数倍大きい。
const HEAD_ONSET_SECONDS: f64 = 0.3;
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

/// (label, 2 音目の列に ON にするルール, 鳴らす MML)。基準は同じ MML の全ルール OFF。
const CONDITIONS: [(&str, Rule, &str); 17] = [
    ("palm-mute", Rule::PalmMute, MML),
    ("pinch-harmonic", Rule::PinchHarmonic, MML),
    ("slide", Rule::Slide, MML),
    ("choke", Rule::Choke, MML),
    ("vibrato", Rule::Vibrato, MML),
    ("natural-harmonics", Rule::NaturalHarmonics, MML),
    ("brushing", Rule::Brushing, MML),
    ("fret-mute", Rule::FretMute, MML),
    ("slide-out", Rule::SlideOut, MML),
    ("pseudo-legato", Rule::PseudoLegato, MML),
    ("portamento", Rule::Portamento, MML),
    ("slide-in", Rule::SlideIn, MML),
    ("trill-half", Rule::TrillHalf, MML),
    ("trill-whole", Rule::TrillWhole, MML),
    ("trill-min3", Rule::TrillMinorThird, MML),
    ("trill-maj3", Rule::TrillMajorThird, MML),
    ("unison-bend-auto", Rule::UnisonBendAuto, HIGH_MML),
];

/// server を起こし直して `events` を 1 回鳴らし、onset を 1 音目の大きさで探し直した録音。
fn record(setup: &CaptureSetup, label: &str, events: Vec<cmrt_chord::TimedMidiEvent>) -> Segment {
    let mut segments = setup.record(label, &[events]);
    segments.remove(0).with_head_onset(HEAD_ONSET_SECONDS)
}

/// `mml` の 2 音目の列にだけ `rule` を ON にして変換する（`None` は全ルール OFF）。
fn phrase(mml: &str, rule: Option<Rule>) -> Vec<cmrt_chord::TimedMidiEvent> {
    let raw = cmrt_chord::timed_performance(mml)
        .expect("MML を解釈できない")
        .events;
    let mut rules = RuleTable::default();
    if let Some(rule) = rule {
        rules.toggle(COLUMN, rule);
    }
    convert(&raw, &rules)
}

#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn each_articulation_rule_changes_the_sound_of_its_column() {
    let Some(setup) = CaptureSetup::from_env("articulation-capture") else {
        return;
    };
    let plain = record(&setup, "plain", phrase(MML, None));
    let high_plain = record(&setup, "plain-high", phrase(HIGH_MML, None));
    let recorded = CONDITIONS
        .iter()
        .map(|(label, rule, mml)| {
            let segment = record(&setup, label, phrase(mml, Some(*rule)));
            (*label, *rule, *mml, segment)
        })
        .collect::<Vec<_>>();
    drop(setup);

    let (plain_spread, plain_mean) = period_stats("plain", &plain);
    let mut failures = Vec::new();
    for (label, rule, mml, segment) in &recorded {
        let base = if *mml == HIGH_MML {
            &high_plain
        } else {
            &plain
        };
        let ncc = compare(label, base, segment);
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
            let (spread, mean) = period_stats(label, segment);
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

/// スライドインの幅（CC27）で滑り込む音が変わるか。同じ G2 へ幅 1 と幅 7 で滑り込み、2 音目を比べる。
#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn slide_in_width_changes_the_sound() {
    let Some(setup) = CaptureSetup::from_env("articulation-capture") else {
        return;
    };
    let narrow = record(
        &setup,
        "slide-in-width1",
        phrase(SLIDE_IN_NARROW_MML, Some(Rule::SlideIn)),
    );
    let wide = record(
        &setup,
        "slide-in-width7",
        phrase(SLIDE_IN_WIDE_MML, Some(Rule::SlideIn)),
    );
    drop(setup);
    let ncc = compare("slide-in width1 vs width7", &narrow, &wide);
    assert!(
        ncc[1] < DIFFERENT,
        "幅 1 と幅 7 の 2 音目が同じ音（ncc={:.4}）",
        ncc[1]
    );
}

/// ユニゾンチョーキング（手動）の pitch bend で音が変わるか。同じ変換結果から pitch bend だけを抜いた列と、2 音目を比べる。
#[test]
#[ignore = "実機の sampler 音色が要る（CMRT_TEST_KEYSWITCH_PATCH）"]
fn unison_bend_manual_pitch_bend_changes_the_sound() {
    let Some(setup) = CaptureSetup::from_env("articulation-capture") else {
        return;
    };
    let bent = phrase(HIGH_MML, Some(Rule::UnisonBendManual));
    assert!(
        bent.iter()
            .any(|event| event.message[0] & 0xF0 == PITCH_BEND),
        "pitch bend が無い"
    );
    let unbent = bent
        .iter()
        .copied()
        .filter(|event| event.message[0] & 0xF0 != PITCH_BEND)
        .collect();
    let without = record(&setup, "unison-manual-no-bend", unbent);
    let with = record(&setup, "unison-manual-bend", bent);
    drop(setup);
    let ncc = compare("unison manual no bend vs bend", &without, &with);
    assert!(ncc[0] > SAME, "1 音目が違う（ncc={:.4}）", ncc[0]);
    assert!(
        ncc[1] < DIFFERENT,
        "pitch bend の有無で 2 音目が同じ音（ncc={:.4}）",
        ncc[1]
    );
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
