use super::*;

const RATE: u32 = 48_000;

/// 440 Hz・振幅 0.5 の sine を左右同じ値で `seconds` 秒。
fn sine(seconds: f64) -> Vec<f32> {
    let frames = (seconds * f64::from(RATE)) as usize;
    (0..frames)
        .flat_map(|n| {
            let v = 0.5 * (2.0 * std::f64::consts::PI * 440.0 * n as f64 / f64::from(RATE)).sin();
            [v as f32, v as f32]
        })
        .collect()
}

fn frames_of_ms(ms: usize) -> usize {
    RATE as usize * ms / 1000
}

#[test]
fn a_clean_sine_has_no_click_and_no_dropout() {
    let samples = sine(1.0);
    assert_eq!(detect_clicks(&samples, RATE, DEFAULT_CLICK_K), vec![]);
    assert_eq!(detect_dropouts(&samples, RATE), vec![]);
}

#[test]
fn an_inserted_step_is_found_as_one_click_at_its_frame() {
    let mut samples = sine(1.0);
    let at = frames_of_ms(500);
    // 波形を途中で 0.4 だけずらす（切り替えで前の音が途切れたときの段差と同じ形）。
    for sample in &mut samples[at * 2..] {
        *sample += 0.4;
    }

    let clicks = detect_clicks(&samples, RATE, DEFAULT_CLICK_K);

    assert_eq!(clicks.len(), 1, "{clicks:?}");
    assert_eq!(clicks[0].frame, at);
    assert!(clicks[0].magnitude > 0.35);
    assert_eq!(detect_dropouts(&samples, RATE), vec![]);
}

#[test]
fn an_inserted_20ms_gap_is_found_as_one_dropout() {
    let mut samples = sine(1.0);
    let at = frames_of_ms(400);
    let len = frames_of_ms(20);
    samples[at * 2..(at + len) * 2].fill(0.0);

    let dropouts = detect_dropouts(&samples, RATE);

    // 0 の連続は sine の零点をまたいで 1 frame ずれうる。
    assert_eq!(dropouts.len(), 1, "{dropouts:?}");
    assert!(dropouts[0].frame.abs_diff(at) <= 1);
    assert!(dropouts[0].frames.abs_diff(len) <= 2);
}

#[test]
fn leading_and_trailing_silence_is_not_a_dropout() {
    let mut samples = vec![0.0; frames_of_ms(100) * 2];
    samples.extend(sine(0.2));
    samples.extend(vec![0.0; frames_of_ms(100) * 2]);
    assert_eq!(detect_dropouts(&samples, RATE), vec![]);
}

#[test]
fn heads_are_measured_from_each_send_frame() {
    // 0〜100 ms 無音、100 ms から鳴る。2 行目は 300 ms に送って 350 ms から鳴る。
    let mut samples = vec![0.0; frames_of_ms(100) * 2];
    samples.extend(sine(0.2));
    samples.extend(vec![0.0; frames_of_ms(50) * 2]);
    samples.extend(sine(0.2));

    let heads = line_heads(&samples, RATE, &[0, frames_of_ms(300)]);

    assert_eq!(heads, vec![Some(frames_of_ms(100)), Some(frames_of_ms(50))]);
}

#[test]
fn a_line_that_sounds_right_at_its_send_frame_has_a_zero_head() {
    let mut samples = vec![0.0; frames_of_ms(10) * 2];
    samples.extend(sine(0.1));
    assert_eq!(
        line_heads(&samples, RATE, &[frames_of_ms(10)]),
        vec![Some(0)]
    );
}

#[test]
fn a_silent_line_has_no_head() {
    let samples = vec![0.0; frames_of_ms(100) * 2];
    assert_eq!(line_heads(&samples, RATE, &[0]), vec![None]);
}
