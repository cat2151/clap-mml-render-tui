use super::*;

fn triggers_at(steps: &[usize]) -> [bool; GRID_STEPS] {
    std::array::from_fn(|step| steps.contains(&step))
}

/// 中央に2音だけの行でも、その2音が必ず両端の値になる。
#[test]
fn velocity_spans_the_first_and_last_note_on() {
    let span = velocity_span(&triggers_at(&[6, 9]));

    assert_eq!(span, RampSpan { start: 6, end: 9 });
}

/// 音が1つだけなら幅が潰れる。始点の値で一定になる。
#[test]
fn a_single_note_collapses_the_velocity_span() {
    let span = velocity_span(&triggers_at(&[7]));

    assert_eq!(span, RampSpan { start: 7, end: 7 });
}

/// CC1 は最後の音が鳴り終わるところまで伸ばす。
#[test]
fn cc1_extends_the_span_by_the_sustain_of_the_last_note() {
    assert_eq!(
        cc1_span(&triggers_at(&[6, 9]), 1),
        RampSpan { start: 6, end: 10 }
    );
    assert_eq!(
        cc1_span(&triggers_at(&[0]), 4),
        RampSpan { start: 0, end: 4 }
    );
}

/// chord mode の和音行は step 0 の全音符。小節頭が始点、小節末尾が終点になる。
#[test]
fn a_whole_note_at_the_measure_head_spans_the_whole_measure() {
    let span = cc1_span(&triggers_at(&[0]), GRID_STEPS as u8);

    assert_eq!(span, RampSpan::WHOLE_MEASURE);
}

/// 小節末尾をはみ出す音長は末尾でクランプする。
#[test]
fn the_cc1_span_never_runs_past_the_measure() {
    let span = cc1_span(&triggers_at(&[14]), 4);

    assert_eq!(
        span,
        RampSpan {
            start: 14,
            end: GRID_STEPS - 1
        }
    );
}

/// 全休符の行は鳴らないので、区間は小節まるごとへ落とす。
#[test]
fn a_silent_row_falls_back_to_the_whole_measure() {
    assert_eq!(velocity_span(&triggers_at(&[])), RampSpan::WHOLE_MEASURE);
    assert_eq!(cc1_span(&triggers_at(&[]), 1), RampSpan::WHOLE_MEASURE);
}
