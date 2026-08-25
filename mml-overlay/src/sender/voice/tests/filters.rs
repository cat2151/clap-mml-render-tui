//! 演奏設定の CC1 / velocity が、**wire に載る 3 バイトとして**どう出るか。
//!
//! filter そのものの正しさは `cmrt-midi-filter` と `line_playback::filters` で見ている。
//! ここで見るのは「設定が実際にサーバーへ届くか」だけ。ここが繋がっていないと、
//! `Ctrl+L` は飾りのまま音は何も変わらない。

use super::*;

/// CC の status。channel 0 固定で送る。
const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION: u8 = 1;

/// LFO の 1 周。実装側の定数（`line_playback::filters`）と同じ値をここにも書く。
/// 4 秒はユーザーが決めた仕様なので、実装が黙って変わったらここで落ちてよい。
const PERIOD_SECONDS: f64 = 4.0;

fn program(repeat: bool, modulation: bool, velocity: bool) -> LineProgram {
    let mut program = line(4);
    program.repeat = repeat;
    program.filters = FilterSettings {
        modulation,
        velocity,
    };
    program
}

/// 送った CC を `(timeline 上の秒, 値)` で拾う。
fn control_changes(sink: &FakeSink) -> Vec<(f64, u8)> {
    sink.timeline_events()
        .iter()
        .filter(|event| event.message[0] == CONTROL_CHANGE)
        .map(|event| (event.timeline_seconds, event.message[2]))
        .collect()
}

fn note_velocities(sink: &FakeSink) -> Vec<u8> {
    sink.timeline_events()
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[2])
        .collect()
}

/// CC1 が 4 秒周期で 0 → 127 → 0 を描く。
#[test]
fn the_modulation_sweeps_zero_to_max_and_back_every_four_seconds() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &program(true, true, false));
    // 8 秒ぶん（＝ちょうど 2 周期）が積まれるまで進める。
    for step in 1..=8 {
        voice.pump_repeat(&sink, origin + Duration::from_secs(step));
    }

    let ccs = control_changes(&sink);
    assert!(!ccs.is_empty());
    assert!(ccs.iter().all(|(_, value)| *value <= 127));
    // 折り返しの山と谷が、周期のちょうど半分ずつずれて現れる。
    let peaks: Vec<f64> = ccs
        .iter()
        .filter(|(_, value)| *value == 127)
        .map(|(seconds, _)| *seconds)
        .collect();
    assert!(peaks.len() >= 2, "peaks={peaks:?}");
    for pair in peaks.windows(2) {
        assert!(
            (pair[1] - pair[0] - PERIOD_SECONDS).abs() < 1e-6,
            "peaks={peaks:?}"
        );
    }
    let first_peak = peaks[0];
    let valleys: Vec<f64> = ccs
        .iter()
        .filter(|(_, value)| *value == 0)
        .map(|(seconds, _)| *seconds)
        .collect();
    assert!(
        valleys
            .iter()
            .any(|seconds| (seconds - (first_peak - PERIOD_SECONDS / 2.0)).abs() < 1e-6),
        "valleys={valleys:?} first_peak={first_peak}"
    );
}

/// 同時刻なら CC が note on より前。後だと鳴り始めの 1 音に modulation が乗らない。
#[test]
fn the_modulation_is_sent_before_the_note_on_at_the_same_time() {
    let sink = FakeSink::default();
    let mut voice = voice();

    voice.play_line(&sink, &program(true, true, false));

    let events = sink.timeline_events();
    assert_eq!(events[0].message, [CONTROL_CHANGE, MODULATION, 0]);
    assert_eq!(events[1].message[0], NOTE_ON);
    assert_eq!(events[0].timeline_seconds, events[1].timeline_seconds);
}

/// 周をまたいでも位相は飛ばない。継ぎ足しの境目で値が跳ねたらここが落ちる。
#[test]
fn the_sweep_does_not_jump_where_the_laps_are_stitched() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &program(true, true, false));
    for step in 1..=4 {
        voice.pump_repeat(&sink, origin + Duration::from_secs(step));
    }

    // 1 周 1 秒に対し LFO は 4 秒周期。周の頭で値が戻らず、隣り合う CC の差は高々 1 段。
    let ccs = control_changes(&sink);
    let first = ccs[0].0;
    for pair in ccs.windows(2) {
        assert!(pair[1].0 >= pair[0].0, "ccs={ccs:?}");
        let step = i16::from(pair[1].1) - i16::from(pair[0].1);
        assert!(step.abs() <= 1, "at {} ccs={ccs:?}", pair[1].0);
        if step != 0 {
            continue;
        }
        // **周の頭だけは同じ値がもう一度出る。** 各周は自分の span の先頭に
        // 「その時点の値」を必ず 1 つ置くので、前の周の最後と重なる。値は同じなので
        // 跳ねはしない（1 秒に 1 つ増えるだけ）。周の頭以外で止まったらそれは異常。
        let phase = (pair[1].0 - first).rem_euclid(1.0);
        assert!(
            !(1e-6..=1.0 - 1e-6).contains(&phase),
            "値が動かない点が周の頭以外にある: {} ccs={ccs:?}",
            pair[1].0
        );
    }
}

/// velocity は MML の指定（ここでは 127）ではなく LFO の値になる。1..=127 に収まる。
#[test]
fn the_velocity_is_taken_over_by_the_lfo() {
    let sink = FakeSink::default();
    let mut voice = voice();

    voice.play_line(&sink, &program(false, false, true));

    // 0 / 0.25 / 0.5 / 0.75 秒の 4 音。0 秒は LFO 値 0 だが、note off にしないため 1 へ。
    assert_eq!(note_velocities(&sink), vec![1, 15, 31, 47]);
}

/// **repeat OFF でも filter は効く。** repeat と CC1 は独立した設定。
#[test]
fn a_line_played_once_still_gets_its_modulation() {
    let sink = FakeSink::default();
    let mut voice = voice();

    voice.play_line(&sink, &program(false, true, false));

    let ccs = control_changes(&sink);
    assert!(!ccs.is_empty(), "repeat OFF でも CC1 は載ること");
    assert_eq!(ccs[0].1, 0);
    assert_eq!(sink.count(&Sent::BeginTimeline), 1);
}

/// 両方 OFF なら Stage 5 までと 1 バイトも変わらない。
#[test]
fn with_every_filter_off_the_wire_is_unchanged() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &program(true, false, false));
    for step in 1..=4 {
        voice.pump_repeat(&sink, origin + Duration::from_secs(step));
    }

    assert_eq!(control_changes(&sink), Vec::new());
    let velocities = note_velocities(&sink);
    assert!(!velocities.is_empty());
    assert!(velocities.iter().all(|velocity| *velocity == 127));
}
