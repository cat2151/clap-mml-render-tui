//! 三角波 LFO。**固定グリッドで刻まず、値が変わる点だけを列挙する。**
//!
//! 固定グリッドだと、細かくすれば同じ値の連投が wire を埋め、粗くすれば折り返しを
//! 取りこぼす。整数値が切り替わる時刻を直接求めれば、どちらも起きない。

use crate::Span;

#[cfg(test)]
mod tests;

/// 時刻比較の許容誤差。値が切り替わる「ちょうどその時刻」を、丸め誤差で 1 つ手前の値に
/// 落とさないための幅。秒単位で 1ns 相当なので可聴域には影響しない。
const EPS: f64 = 1e-9;

/// `min → max → min` を `period_seconds` で 1 周する三角波。
///
/// 位相の原点は絶対秒の 0。つまり周期はフレーズやループ長と無関係に回り続ける
/// （4 秒周期なら、何周目のどこであっても絶対秒で 4 秒ごとに折り返す）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleLfo {
    pub period_seconds: f64,
    pub min: u8,
    pub max: u8,
}

impl TriangleLfo {
    pub fn new(period_seconds: f64, min: u8, max: u8) -> Self {
        Self {
            period_seconds,
            min,
            max,
        }
    }

    /// 動く LFO として使えるか。周期が非正、または振幅が無いなら常に `min`。
    fn is_usable(&self) -> bool {
        self.period_seconds > 0.0 && self.period_seconds.is_finite() && self.max > self.min
    }

    /// 振幅（整数の段数）。
    fn steps(&self) -> f64 {
        f64::from(self.max - self.min)
    }

    /// その時刻の値。`0 秒 = min`、`period/2 = max`、`period = min`。
    pub fn value_at(&self, seconds: f64) -> u8 {
        if !self.is_usable() || !seconds.is_finite() {
            return self.min;
        }
        let steps = self.steps();
        let phase = (seconds / self.period_seconds).rem_euclid(1.0);
        // 上りは「その値に達した瞬間から」、下りは「次の値に落ちる直前まで」その値。
        // こうすると各値の滞在時間が等しくなり、上り下りで同じ時刻表になる。
        let stepped = if phase <= 0.5 {
            (steps * phase * 2.0 + EPS).floor()
        } else {
            (steps * (2.0 - phase * 2.0) - EPS).ceil()
        };
        self.min + stepped.clamp(0.0, steps) as u8
    }

    /// `span` 内で値が変わる点。時刻は狭義単調増加で、連続する 2 点の値は必ず異なる。
    ///
    /// 先頭は必ず `span.start_seconds` そのもの（＝その時点の値）。CC として送るとき、
    /// span の頭で現在値を 1 つ置いておかないと、次の折り返しまで値が確定しないため。
    ///
    /// 4 秒周期 `0→127→0` を `[0, 4)` で取ると **254 点**（約 15.7ms 間隔）。
    ///
    /// 点の総数は `span の長さ / period * 2 * (max - min)` に比例する。極端に短い周期を
    /// 長い span に掛けると点が爆発するので、span は呼び出し側が先読み幅で切ること。
    pub fn change_points(&self, span: Span) -> Vec<(f64, u8)> {
        let mut points = Vec::new();
        if !span.start_seconds.is_finite()
            || !span.end_seconds.is_finite()
            || span.end_seconds <= span.start_seconds
        {
            return points;
        }
        points.push((span.start_seconds, self.value_at(span.start_seconds)));
        if !self.is_usable() {
            return points;
        }

        let period = self.period_seconds;
        let half = period / 2.0;
        let steps = self.steps();
        let mut cycle = (span.start_seconds / period).floor();
        loop {
            let base = cycle * period;
            if base >= span.end_seconds {
                break;
            }
            for i in 1..=(self.max - self.min) {
                let ratio = f64::from(i) / steps;
                // 上り: min+i に達する時刻。i = max-min のとき base + half（＝ max）。
                push_inside(&mut points, span, base + ratio * half, self.min + i);
                // 下り: max-i に落ちる時刻。i = max-min のとき base + period（＝ 次の周の頭）。
                push_inside(&mut points, span, base + half + ratio * half, self.max - i);
            }
            cycle += 1.0;
        }
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        points
    }
}

/// 先頭の点と重ならないよう start ちょうどは捨てる（値は既に入っている）。
/// 終端は半開区間なので end ちょうども捨てる。
fn push_inside(points: &mut Vec<(f64, u8)>, span: Span, seconds: f64, value: u8) {
    if seconds > span.start_seconds + EPS && seconds + EPS < span.end_seconds {
        points.push((seconds, value));
    }
}
