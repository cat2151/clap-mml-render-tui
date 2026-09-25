use super::*;

const RATE: u32 = 48_000;
const PREVIOUS_HZ: f64 = 220.0;
const NEXT_NOTE: u8 = 84;
const SWITCH_MS: f64 = 500.0;
const FADE_MS: f64 = 50.0;
/// 次の行の音が鳴り始める、切り替えからの時刻。
const NEXT_ONSET_MS: f64 = 200.0;

fn frames(ms: f64) -> usize {
    (ms * f64::from(RATE) / 1000.0).round() as usize
}

fn sine(hz: f64, frame: usize) -> f64 {
    (2.0 * std::f64::consts::PI * hz * frame as f64 / f64::from(RATE)).sin()
}

/// 前の行（ゆっくり減衰する低い正弦波）だけの録音。
fn previous_line(total_ms: f64) -> Vec<f64> {
    (0..frames(total_ms))
        .map(|frame| 0.3 * (-(frame as f64) / f64::from(RATE)).exp() * sine(PREVIOUS_HZ, frame))
        .collect()
}

/// 切り替えありの録音。`fade` なら前の行を切り替えから `FADE_MS` で 0 へ絞る。
fn switched(total_ms: f64, fade: bool) -> Vec<f64> {
    let switch = frames(SWITCH_MS);
    let fade_frames = frames(FADE_MS) as f64;
    let onset = switch + frames(NEXT_ONSET_MS);
    previous_line(total_ms)
        .into_iter()
        .enumerate()
        .map(|(frame, previous)| {
            let gain = if fade && frame >= switch {
                (1.0 - (frame - switch) as f64 / fade_frames).max(0.0)
            } else {
                1.0
            };
            let next = if frame >= onset {
                0.3 * sine(note_hz(NEXT_NOTE), frame - onset)
            } else {
                0.0
            };
            previous * gain + next
        })
        .collect()
}

fn stereo(mono: &[f64]) -> Vec<f32> {
    mono.iter()
        .flat_map(|&sample| [sample as f32, sample as f32])
        .collect()
}

fn analyse(fade: bool) -> Residual {
    residual(
        &stereo(&switched(1500.0, fade)),
        &stereo(&previous_line(1500.0)),
        RATE,
        frames(SWITCH_MS),
        note_hz(NEXT_NOTE),
        700.0,
    )
}

fn clean_windows(result: &Residual) -> Vec<&ResidualWindow> {
    result
        .windows
        .iter()
        .filter(|window| window.start_ms >= FIRST_WINDOW_MS && !window.next_line_leak)
        .collect()
}

#[test]
fn a_faded_previous_line_is_gone_after_the_fade() {
    let result = analyse(true);
    assert_eq!(
        result.windows.first().map(|window| window.end_ms),
        Some(60.0)
    );
    assert_eq!(
        result.windows.last().map(|window| window.end_ms),
        Some(700.0)
    );
    assert_eq!(result.next_line_onset_ms, Some(NEXT_ONSET_MS));
    assert_eq!(result.gone_from_ms, Some(FADE_MS));
    let clean = clean_windows(&result);
    assert_eq!(clean.len(), 2, "{:?}", result.windows);
    for window in clean {
        assert!(
            window.attenuation_db() >= 40.0,
            "fadeout したのに残っている: {window:?}"
        );
    }
}

#[test]
fn an_unfaded_previous_line_matches_the_reference() {
    let result = analyse(false);
    assert_eq!(result.gone_from_ms, None);
    for window in clean_windows(&result) {
        assert!(
            window.attenuation_db().abs() <= 1.0,
            "fadeout していないのに差がある: {window:?}"
        );
    }
}

#[test]
fn the_notch_removes_only_its_frequency() {
    let tone = |hz: f64| {
        stereo(
            &(0..frames(500.0))
                .map(|frame| sine(hz, frame))
                .collect::<Vec<_>>(),
        )
    };
    let tail = frames(400.0)..frames(500.0);
    let next_hz = note_hz(NEXT_NOTE);
    let removed = rms_db(&notched_mono(&tone(next_hz), RATE, next_hz)[tail.clone()]);
    let kept = rms_db(&notched_mono(&tone(PREVIOUS_HZ), RATE, next_hz)[tail]);
    assert!(removed < -80.0, "removed={removed}");
    assert!(kept > -4.0, "kept={kept}");
}
