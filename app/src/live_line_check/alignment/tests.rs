use super::*;

const RATE: u32 = 48_000;

/// 立ち上がり 30 ms・減衰 200 ms の音を `at_ms` から置いた、2 秒のステレオ波形。
///
/// 立ち上がりの無い（指数減衰だけの）包絡では頭の欠けがずれに出ないので、立ち上がりを付ける。
fn note_at(at_ms: f64) -> Vec<f32> {
    let frames = 2 * RATE as usize;
    let start = (at_ms * f64::from(RATE) / 1000.0) as usize;
    let mut samples = vec![0.0; frames * 2];
    for frame in start..frames {
        let t = (frame - start) as f64 / f64::from(RATE);
        let envelope = (t / 0.03).min(1.0) * (-t / 0.2).exp();
        let value = (envelope * (t * 440.0 * std::f64::consts::TAU).sin()) as f32;
        samples[frame * 2] = value;
        samples[frame * 2 + 1] = value;
    }
    samples
}

fn frames(ms: f64) -> usize {
    (ms * f64::from(RATE) / 1000.0) as usize
}

#[test]
fn the_same_shape_from_its_own_head_has_no_lag() {
    let reference = note_at(0.0);
    let live = note_at(100.0);
    let lag = envelope_lag_ms(&live, frames(100.0), &reference, 0, RATE, 200.0, 50.0);
    assert_eq!(lag, Some(0.0));
}

/// 頭を 20 ms 欠いた音は、頭と思った点から見ると 20 ms 先へ進んでいる（lag は負）。
#[test]
fn a_cut_head_shows_up_as_a_lag() {
    let reference = note_at(0.0);
    let mut live = note_at(100.0);
    for sample in &mut live[..frames(120.0) * 2] {
        *sample = 0.0;
    }
    let lag = envelope_lag_ms(&live, frames(120.0), &reference, 0, RATE, 200.0, 50.0).unwrap();
    assert!((lag + 20.0).abs() <= 2.0, "lag={lag}");
}

#[test]
fn a_late_start_shows_up_as_a_positive_lag() {
    let reference = note_at(0.0);
    let live = note_at(130.0);
    let lag = envelope_lag_ms(&live, frames(100.0), &reference, 0, RATE, 200.0, 50.0).unwrap();
    assert!((lag - 30.0).abs() <= 1.0, "lag={lag}");
}

#[test]
fn silence_has_no_lag() {
    let reference = note_at(0.0);
    let live = vec![0.0; reference.len()];
    assert_eq!(
        envelope_lag_ms(&live, 0, &reference, 0, RATE, 200.0, 50.0),
        None
    );
}
