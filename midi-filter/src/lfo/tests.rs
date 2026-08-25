use super::*;

fn lfo() -> TriangleLfo {
    TriangleLfo::new(4.0, 0, 127)
}

#[test]
fn value_at_hits_min_max_min_over_one_period() {
    let lfo = lfo();
    assert_eq!(lfo.value_at(0.0), 0);
    assert_eq!(lfo.value_at(2.0), 127);
    assert_eq!(lfo.value_at(4.0), 0);
}

#[test]
fn value_at_is_periodic_and_symmetric() {
    let lfo = lfo();
    // 位相の原点は絶対秒の 0 なので、何周目でも同じ値。
    assert_eq!(lfo.value_at(1.0), lfo.value_at(9.0));
    // 上りは floor、下りは ceil で刻む（各値の滞在時間を等しくするため）ので、
    // 折り返しを挟んで対称な 2 点は 1 段までずれる。ここを 0 にすると、上りと下りの
    // どちらかで滞在時間が半端になる。
    let up = i16::from(lfo.value_at(0.4));
    let down = i16::from(lfo.value_at(3.6));
    assert!((up - down).abs() <= 1, "{up} vs {down}");
    // 上りの真ん中はおよそ中間値。
    assert_eq!(lfo.value_at(1.0), 63);
}

#[test]
fn value_at_stays_inside_min_max() {
    let lfo = TriangleLfo::new(4.0, 20, 90);
    for i in 0..4000 {
        let value = lfo.value_at(f64::from(i) * 0.003);
        assert!((20..=90).contains(&value), "{value}");
    }
}

#[test]
fn degenerate_lfo_is_constant() {
    assert_eq!(TriangleLfo::new(0.0, 0, 127).value_at(1.0), 0);
    assert_eq!(TriangleLfo::new(4.0, 64, 64).value_at(1.0), 64);
    assert_eq!(TriangleLfo::new(4.0, 100, 10).value_at(1.0), 100);
}

#[test]
fn change_points_over_one_period_are_254_and_increasing() {
    let points = lfo().change_points(Span::new(0.0, 4.0));

    assert_eq!(points.len(), 254);
    for pair in points.windows(2) {
        assert!(pair[1].0 > pair[0].0, "{:?} -> {:?}", pair[0], pair[1]);
        assert_ne!(pair[1].1, pair[0].1, "同じ値を連投しないこと");
    }
    assert_eq!(points[0], (0.0, 0));
    assert_eq!(points[127], (2.0, 127));
    assert_eq!(points.last().copied().unwrap().1, 1);
}

#[test]
fn change_points_agree_with_value_at() {
    let lfo = lfo();
    for (seconds, value) in lfo.change_points(Span::new(0.0, 4.0)) {
        assert_eq!(lfo.value_at(seconds), value, "at {seconds}");
    }
}

#[test]
fn change_points_start_from_the_span_head_even_mid_cycle() {
    let lfo = lfo();
    let span = Span::new(5.5, 7.0);
    let points = lfo.change_points(span);

    assert_eq!(points[0].0, 5.5);
    assert_eq!(points[0].1, lfo.value_at(5.5));
    assert!(points.iter().all(|p| p.0 >= 5.5 && p.0 < 7.0));
    for pair in points.windows(2) {
        assert!(pair[1].0 > pair[0].0);
        assert_ne!(pair[1].1, pair[0].1);
    }
}

#[test]
fn change_points_cross_the_fold_and_the_cycle_boundary() {
    let lfo = lfo();
    // 折り返し（2.0 秒）を跨ぐと値が上って下る。
    let values: Vec<u8> = lfo
        .change_points(Span::new(1.9, 2.1))
        .into_iter()
        .map(|p| p.1)
        .collect();
    assert_eq!(values.iter().copied().max(), Some(127));
    assert!(values.first() < values.iter().max());
    assert!(values.last() < values.iter().max());

    // 周の境目（4.0 秒）でも折り返す。
    let values: Vec<u8> = lfo
        .change_points(Span::new(3.9, 4.1))
        .into_iter()
        .map(|p| p.1)
        .collect();
    assert_eq!(values.iter().copied().min(), Some(0));
}

#[test]
fn empty_or_backwards_span_has_no_points() {
    assert!(lfo().change_points(Span::new(2.0, 2.0)).is_empty());
    assert!(lfo().change_points(Span::new(3.0, 1.0)).is_empty());
}

#[test]
fn degenerate_lfo_emits_only_the_span_head() {
    assert_eq!(
        TriangleLfo::new(4.0, 64, 64).change_points(Span::new(0.0, 8.0)),
        vec![(0.0, 64)]
    );
}
