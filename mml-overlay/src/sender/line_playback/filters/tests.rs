//! filter は「絶対秒がそのまま位相」。周をまたいでも位相が飛ばないことをここで固定する。

use super::*;

fn note_on(seconds: f64) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x90, 60, 127],
    }
}

fn note_off(seconds: f64) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x80, 60, 0],
    }
}

/// 1 周 1 秒、頭と真ん中に note on、終わりに note off。
fn cycle() -> Vec<TimedMidiEvent> {
    vec![note_on(0.0), note_on(0.5), note_off(1.0)]
}

fn both() -> FilterSettings {
    FilterSettings {
        modulation: true,
        velocity: true,
    }
}

fn modulation() -> FilterSettings {
    FilterSettings {
        modulation: true,
        velocity: false,
    }
}

fn control_changes(events: &[TimedMidiEvent]) -> Vec<(f64, u8)> {
    events
        .iter()
        .filter(|event| event.message[0] == 0xB0)
        .map(|event| (event.seconds, event.message[2]))
        .collect()
}

/// filter を全部 OFF にしたら、周をずらすだけ。CC も velocity も足さない。
#[test]
fn without_any_filter_a_lap_is_just_a_shift() {
    let lap = lap(&cycle(), FilterSettings::default(), 3.0, 1.0);

    assert_eq!(lap, shift(&cycle(), 3.0));
}

/// CC は周の範囲だけに入る。次の周のぶんまで先走って入れない（二重に入る）。
#[test]
fn control_changes_stay_inside_the_lap() {
    let lap = lap(&cycle(), modulation(), 3.0, 1.0);

    let ccs = control_changes(&lap);
    assert!(!ccs.is_empty());
    assert!(ccs.iter().all(|(seconds, _)| (3.0..4.0).contains(seconds)));
    assert!(ccs.windows(2).all(|pair| pair[1].0 > pair[0].0));
    assert!(ccs.iter().all(|(_, value)| *value <= 127));
}

/// **これが継ぎ目の無さの本体**。周 k の頭の CC 値は、絶対秒だけで決まる。
/// ループ長（1 秒）と LFO の周期（4 秒）が別物でも、位相は連続する。
#[test]
fn the_phase_follows_the_absolute_seconds_not_the_lap_number() {
    let lfo = filter_lfo();

    for lap_index in 0..8 {
        let offset = f64::from(lap_index);
        let lap = lap(&cycle(), modulation(), offset, 1.0);

        // 周の頭には必ずその時点の値が 1 つ置かれる（次の変化点まで値が確定しないため）。
        assert_eq!(control_changes(&lap)[0], (offset, lfo.value_at(offset)));
    }
    // 4 秒周期なので 0 秒と 4 秒は同じ値へ戻る。
    assert_eq!(lfo.value_at(0.0), FILTER_MIN);
    assert_eq!(lfo.value_at(4.0), FILTER_MIN);
    assert_eq!(lfo.value_at(2.0), FILTER_MAX);
}

/// 同時刻なら CC が先。後だと、鳴り始めの 1 音に modulation 値が乗らない。
#[test]
fn a_control_change_comes_before_the_note_on_at_the_same_time() {
    let lap = lap(&cycle(), modulation(), 2.0, 1.0);

    assert_eq!(lap[0].message[0], 0xB0);
    assert_eq!(lap[1].message[0], 0x90);
    assert_eq!(lap[0].seconds, lap[1].seconds);
}

/// velocity は MML の指定ではなく、その音自身の時刻の LFO 値になる。
#[test]
fn the_velocity_is_taken_over_by_the_lfo() {
    let lfo = filter_lfo();
    let lap = lap(&cycle(), both(), 1.0, 1.0);

    let velocities: Vec<(f64, u8)> = lap
        .iter()
        .filter(|event| event.message[0] == 0x90)
        .map(|event| (event.seconds, event.message[2]))
        .collect();
    assert_eq!(
        velocities,
        vec![
            (1.0, lfo.value_at(1.0).max(1)),
            (1.5, lfo.value_at(1.5).max(1)),
        ]
    );
    // MML 由来の 127 が残っていない＝乗っ取れている。
    assert!(velocities.iter().all(|(_, value)| *value != 127));
    // note off には触らない。触ると音が切れなくなる。
    assert_eq!(lap.last().unwrap().message, [0x80, 60, 0]);
}

/// 1 回だけ鳴らす行にも filter は掛かる。repeat と CC1 は独立した設定。
#[test]
fn a_line_played_once_still_gets_its_filters() {
    let once = one_shot(&cycle(), modulation(), 1.0);

    let ccs = control_changes(&once);
    assert_eq!(ccs[0], (0.0, FILTER_MIN));
    assert!(ccs.iter().all(|(seconds, _)| (0.0..1.0).contains(seconds)));
}

/// `loop_seconds` が 0 でも、実際のイベントが伸びていればそこまで掛ける。
/// `loop_seconds` は「最後のイベントまで」しか測らないので、両方を見る。
#[test]
fn the_span_of_a_single_shot_covers_the_events_themselves() {
    let once = one_shot(&cycle(), modulation(), 0.0);

    let ccs = control_changes(&once);
    assert!(ccs.len() > 1, "ccs={ccs:?}");
    assert!(ccs.iter().all(|(seconds, _)| (0.0..1.0).contains(seconds)));
}

/// 長さの無い行（全イベントが 0 秒）には掛ける時間が無い。落ちずに素通しする。
#[test]
fn a_line_with_no_length_is_left_alone() {
    let once = one_shot(&[note_on(0.0)], both(), 0.0);

    assert_eq!(control_changes(&once), Vec::new());
    // velocity は時刻に依らず掛かる。0 秒の LFO 値は 0 だが、note off にしない下限 1 へ。
    assert_eq!(once[0].message, [0x90, 60, 1]);
}
