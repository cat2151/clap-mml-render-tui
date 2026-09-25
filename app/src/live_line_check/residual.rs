//! 切り替えの後に残る前の行の音を、前の行だけを鳴らした録音と比べる。
//!
//! 2 行目は高い正弦波で書き、その周波数を notch で除く。残りは前の行の音（音色の release と
//! effect の余韻）なので、同じ時刻の前の行だけの録音（同じく notch 済み）の RMS と比べる。
//! 差が大きいほど、前の行の音が切り替えで消えている。
//!
//! 2 行目が鳴り始めると、その頭の過渡が notch を漏れる。2 行目の頭（notch で除いた帯域だけの
//! 波形の立ち上がり）より後の窓は、前の行の残りではなく漏れの上限として印を付ける。

use anyhow::Result;

use super::ResidualRequest;
use crate::live_chord_check::{read_capture, Capture};

/// notch の鋭さ。帯域幅は `notch_hz / NOTCH_Q`。
const NOTCH_Q: f64 = 15.0;
/// 切り替えの直後は fadeout の途中なので、最初の窓だけこの長さにする。
const FIRST_WINDOW_MS: f64 = 60.0;
/// 2 つ目以降の窓の長さ。
const WINDOW_MS: f64 = 50.0;
/// 「前の行の残りが消えた」時刻を探す刻み。
const FINE_WINDOW_MS: f64 = 5.0;
/// 前の行の残りが消えたとみなす減衰。
const GONE_DB: f64 = 40.0;
/// 2 行目の帯域の 1 ms peak がこれを超えたら、2 行目が鳴り始めたとみなす。
const NEXT_LINE_BAND_PEAK: f64 = 1.0e-2;
/// 停止の段差を窓に入れないよう、次の送信（停止）のこの手前で窓を終える。
const STOP_MARGIN_MS: f64 = 10.0;
/// RMS が 0 のときの dB。
const SILENT_DB: f64 = -200.0;

/// 切り替えからの 1 窓。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ResidualWindow {
    /// 切り替えからの窓の始まり。
    pub(super) start_ms: f64,
    pub(super) end_ms: f64,
    /// 切り替えありの録音の、notch 後の RMS。
    pub(super) capture_db: f64,
    /// 前の行だけの録音の、notch 後の RMS。
    pub(super) reference_db: f64,
    /// 2 行目の頭より後にかかる窓（2 行目の漏れを含む）。
    pub(super) next_line_leak: bool,
}

impl ResidualWindow {
    /// 前の行だけの録音より何 dB 低いか。
    pub(super) fn attenuation_db(&self) -> f64 {
        self.reference_db - self.capture_db
    }
}

/// 解析の結果。時刻はどれも切り替え（2 行目の送信）から。
#[derive(Debug)]
pub(super) struct Residual {
    pub(super) windows: Vec<ResidualWindow>,
    /// 2 行目の頭。
    pub(super) next_line_onset_ms: Option<f64>,
    /// ここから 2 行目の頭まで、[`FINE_WINDOW_MS`] 刻みのどの窓も [`GONE_DB`] 以上低い。
    pub(super) gone_from_ms: Option<f64>,
}

/// 12 平均律（A4 = 440 Hz）の周波数。
pub(super) fn note_hz(note: u8) -> f64 {
    440.0 * 2f64.powf((f64::from(note) - 69.0) / 12.0)
}

/// インターリーブステレオを mono にし、`notch_hz` を除く（RBJ の notch biquad を順方向に 1 回）。
pub(super) fn notched_mono(samples: &[f32], sample_rate: u32, notch_hz: f64) -> Vec<f64> {
    let w0 = 2.0 * std::f64::consts::PI * notch_hz / f64::from(sample_rate);
    let alpha = w0.sin() / (2.0 * NOTCH_Q);
    let a0 = 1.0 + alpha;
    let (b0, b1, b2) = (1.0 / a0, -2.0 * w0.cos() / a0, 1.0 / a0);
    let (a1, a2) = (-2.0 * w0.cos() / a0, (1.0 - alpha) / a0);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    mono(samples)
        .map(|x| {
            let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            y
        })
        .collect()
}

/// notch で除かれる帯域だけの波形（mono を左右へ複製したインターリーブステレオ）。
pub(super) fn band_stereo(samples: &[f32], sample_rate: u32, notch_hz: f64) -> Vec<f32> {
    mono(samples)
        .zip(notched_mono(samples, sample_rate, notch_hz))
        .flat_map(|(x, notched)| {
            let band = (x - notched) as f32;
            [band, band]
        })
        .collect()
}

fn mono(samples: &[f32]) -> impl Iterator<Item = f64> + '_ {
    samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[left, right]| (f64::from(*left) + f64::from(*right)) / 2.0)
}

/// `switch_frame` から `until_ms` までを窓に分け、両方の録音の notch 後の RMS を並べる。
/// 窓は最初が [`FIRST_WINDOW_MS`]、以後 [`WINDOW_MS`]。どちらかの録音が尽きたら終わる。
pub(super) fn residual(
    capture: &[f32],
    reference: &[f32],
    sample_rate: u32,
    switch_frame: usize,
    notch_hz: f64,
    until_ms: f64,
) -> Residual {
    let frames_per_ms = f64::from(sample_rate) / 1000.0;
    let next_line_onset_ms = next_line_onset_ms(
        &band_stereo(capture, sample_rate, notch_hz),
        switch_frame,
        frames_per_ms,
    );
    let capture = notched_mono(capture, sample_rate, notch_hz);
    let reference = notched_mono(reference, sample_rate, notch_hz);
    let rms = |samples: &[f64], start_ms: f64, end_ms: f64| {
        let start = switch_frame + (start_ms * frames_per_ms).round() as usize;
        let end = switch_frame + (end_ms * frames_per_ms).round() as usize;
        (end <= samples.len() && start < end).then(|| rms_db(&samples[start..end]))
    };

    let mut windows = Vec::new();
    let mut start_ms = 0.0;
    while start_ms < until_ms {
        let len_ms = if windows.is_empty() {
            FIRST_WINDOW_MS
        } else {
            WINDOW_MS
        };
        let end_ms = f64::min(start_ms + len_ms, until_ms);
        let (Some(capture_db), Some(reference_db)) = (
            rms(&capture, start_ms, end_ms),
            rms(&reference, start_ms, end_ms),
        ) else {
            break;
        };
        windows.push(ResidualWindow {
            start_ms,
            end_ms,
            capture_db,
            reference_db,
            next_line_leak: next_line_onset_ms.is_some_and(|onset| end_ms > onset),
        });
        start_ms = end_ms;
    }

    // 頭の検出は立ち上がりの途中で閾値を越えるので、その手前の 1 窓も漏れを含みうる。
    let fine_until = next_line_onset_ms
        .map_or(until_ms, |onset| onset - FINE_WINDOW_MS)
        .min(until_ms);
    let mut gone_from_ms = None;
    let mut start_ms = 0.0;
    while start_ms + FINE_WINDOW_MS <= fine_until {
        let end_ms = start_ms + FINE_WINDOW_MS;
        let gone = rms(&capture, start_ms, end_ms)
            .zip(rms(&reference, start_ms, end_ms))
            .is_some_and(|(capture_db, reference_db)| reference_db - capture_db >= GONE_DB);
        match (gone, gone_from_ms) {
            (true, None) => gone_from_ms = Some(start_ms),
            (false, _) => gone_from_ms = None,
            (true, Some(_)) => {}
        }
        start_ms = end_ms;
    }
    Residual {
        windows,
        next_line_onset_ms,
        gone_from_ms,
    }
}

/// 帯域だけの波形で、`switch_frame` の後に 2 行目が鳴り始めた時刻（ms）。
fn next_line_onset_ms(band: &[f32], switch_frame: usize, frames_per_ms: f64) -> Option<f64> {
    let block = frames_per_ms.round().max(1.0) as usize;
    let frames = band.as_chunks::<2>().0.get(switch_frame..).unwrap_or(&[]);
    frames
        .chunks(block)
        .position(|frames| {
            frames
                .iter()
                .any(|[left, _]| f64::from(left.abs()) >= NEXT_LINE_BAND_PEAK)
        })
        .map(|index| (index * block) as f64 / frames_per_ms)
}

fn rms_db(samples: &[f64]) -> f64 {
    let mean_square =
        samples.iter().map(|sample| sample * sample).sum::<f64>() / samples.len() as f64;
    if mean_square > 0.0 {
        10.0 * mean_square.log10()
    } else {
        SILENT_DB
    }
}

/// `[residual]` 節を出す。`live_sends[1]`（2 行目の送信）から次の送信の手前までを見る。
pub(super) fn report(
    request: &ResidualRequest,
    capture: &Capture,
    live_sends: &[usize],
    step_ms: u64,
) -> Result<()> {
    let Some(&switch_frame) = live_sends.get(1) else {
        anyhow::bail!("--residual-reference には行を 2 つ以上指定してください");
    };
    let reference = read_capture(&request.reference)?;
    if reference.channels != 2 || reference.sample_rate != capture.sample_rate {
        anyhow::bail!(
            "reference の形式が録音と違います: channels={} sample_rate={}",
            reference.channels,
            reference.sample_rate
        );
    }
    let notch_hz = note_hz(request.notch_note);
    let result = residual(
        &capture.samples,
        &reference.samples,
        capture.sample_rate,
        switch_frame,
        notch_hz,
        step_ms as f64 - STOP_MARGIN_MS,
    );
    print(&result, notch_hz);
    Ok(())
}

fn print(result: &Residual, notch_hz: f64) {
    let optional_ms =
        |value: Option<f64>| value.map_or_else(|| "-".to_string(), |value| format!("{value:.0}"));
    println!();
    println!(
        "[residual] 2 行目の送信からの、前の行の残り（notch {notch_hz:.1} Hz Q={NOTCH_Q}。reference = 前の行だけの録音）"
    );
    for window in &result.windows {
        println!(
            "  {:>6.0}..{:<6.0} ms capture_db={:>7.1} reference_db={:>7.1} attenuation_db={:>6.1}{}",
            window.start_ms,
            window.end_ms,
            window.capture_db,
            window.reference_db,
            window.attenuation_db(),
            if window.next_line_leak {
                " (2 行目の漏れを含む)"
            } else {
                ""
            }
        );
    }
    let before_next_line = result
        .windows
        .iter()
        .filter(|window| window.start_ms >= FIRST_WINDOW_MS && !window.next_line_leak)
        .map(ResidualWindow::attenuation_db)
        .reduce(f64::min);
    println!(
        "  next_line_onset_ms={} gone_from_ms={} (以後 2 行目の頭まで {FINE_WINDOW_MS:.0} ms 刻みで {GONE_DB:.0} dB 以上低い)",
        optional_ms(result.next_line_onset_ms),
        optional_ms(result.gone_from_ms),
    );
    println!(
        "  min_attenuation_from_{FIRST_WINDOW_MS:.0}ms_before_next_line_db={}",
        before_next_line.map_or_else(|| "-".to_string(), |value| format!("{value:.1}"))
    );
}

#[cfg(test)]
mod tests;
