use super::*;
use crate::LoopPlaybackClip;
use std::path::PathBuf;

fn one_measure_grid() -> LoopPlaybackGrid {
    vec![vec![Some(LoopPlaybackClip {
        path: PathBuf::from("/loops/00.wav"),
        span_measures: 1,
        kind: cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
        bpm: Some(120.0),
        category: None,
        meter_numerator: 4,
        meter_denominator: 4,
    })]]
}

#[test]
fn the_cycle_counter_advances_once_per_lap_and_marks_the_boundary() {
    let grid = one_measure_grid();
    let mut transport = TransportState::default();

    // 1 周目の頭。まだラップしていない。
    let measure = transport.next_measure_to_start(&grid).unwrap();
    transport.started(measure);
    assert_eq!(transport.cycle(), 0);
    // 次に鳴らす measure は先頭へ折り返す＝いまが差し替えてよい境目。
    assert!(transport.wraps_next(&grid));

    let measure = transport.next_measure_to_start(&grid).unwrap();
    transport.started(measure);
    assert_eq!(transport.cycle(), 1);
}

#[test]
fn restarting_and_pausing_count_the_cycles_from_scratch() {
    let grid = one_measure_grid();
    let mut transport = TransportState::default();
    for _ in 0..3 {
        let measure = transport.next_measure_to_start(&grid).unwrap();
        transport.started(measure);
    }
    assert_eq!(transport.cycle(), 2);

    transport.restart_at(0);
    assert_eq!(transport.cycle(), 0);
    // restart 待ちを「周の境目」と誤認して差し替えないこと。
    assert!(!transport.wraps_next(&grid));

    let measure = transport.next_measure_to_start(&grid).unwrap();
    transport.started(measure);
    transport.pause();
    assert_eq!(transport.cycle(), 0);
    assert!(!transport.wraps_next(&grid));
}

#[test]
fn a_multi_measure_grid_is_only_at_the_boundary_on_its_last_measure() {
    let mut grid = one_measure_grid();
    let second = grid[0][0].clone();
    grid[0].push(second);
    let mut transport = TransportState::default();

    // 1 小節目。まだ折り返さないので差し替えてはいけない。
    let measure = transport.next_measure_to_start(&grid).unwrap();
    transport.started(measure);
    assert_eq!(measure, 0);
    assert!(!transport.wraps_next(&grid));

    // 最終小節でだけ境目になる。
    let measure = transport.next_measure_to_start(&grid).unwrap();
    transport.started(measure);
    assert_eq!(measure, 1);
    assert_eq!(transport.cycle(), 0);
    assert!(transport.wraps_next(&grid));
}
