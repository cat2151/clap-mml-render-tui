//! 小節ごとに引く、レーン値の決め方。
//!
//! CC1 と velocity は1回の抽選結果を分け合う。「両方まとめて完全ランダム」と
//! 「上り下りの組み合わせ4通り」の計5通りを均等に引くので、小節単位で
//! クレッシェンド／デクレッシェンドの起伏が付く。

use rand::RngExt;

use crate::GRID_STEPS;

/// 1小節ぶんのレーン値の決め方。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::state) enum MeasurePattern {
    /// セル単位で2値から抽選する。
    Random,
    /// `RampSpan` の始点から終点へ線形に変化する。`ascending` は `choices[0]` から
    /// `choices[1]` へ上がる向き。
    Ramp { ascending: bool },
}

/// ランプを張る区間（両端を含む step 番号）。行ごとに、実際に鳴る範囲から決まる。
///
/// 区間の外は端の値でクランプする。`end <= start` なら幅が潰れているので
/// 始点の値で一定になる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::state) struct RampSpan {
    pub(in crate::state) start: usize,
    pub(in crate::state) end: usize,
}

impl RampSpan {
    /// 小節まるごと。鳴るセルが分からないときのフォールバック。
    pub(in crate::state) const WHOLE_MEASURE: Self = Self {
        start: 0,
        end: GRID_STEPS - 1,
    };
}

/// 1小節ぶんの、CC1 と velocity をまとめた抽選結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::state) struct MeasurePlan {
    pub(in crate::state) cc1: MeasurePattern,
    pub(in crate::state) velocity: MeasurePattern,
}

/// 均等に引く5通り。`None` は両レーンとも完全ランダム、`Some` は
/// `(velocity の向き, CC1 の向き)`。
const PLANS: [Option<(bool, bool)>; 5] = [
    None,
    Some((true, true)),
    Some((false, false)),
    Some((true, false)),
    Some((false, true)),
];

impl MeasurePlan {
    pub(in crate::state) fn draw(rng: &mut impl RngExt) -> Self {
        Self::from_plan(PLANS[rng.random_range(0..PLANS.len())])
    }

    fn from_plan(plan: Option<(bool, bool)>) -> Self {
        match plan {
            None => Self {
                cc1: MeasurePattern::Random,
                velocity: MeasurePattern::Random,
            },
            Some((velocity_ascending, cc1_ascending)) => Self {
                cc1: MeasurePattern::Ramp {
                    ascending: cc1_ascending,
                },
                velocity: MeasurePattern::Ramp {
                    ascending: velocity_ascending,
                },
            },
        }
    }
}

/// ランプの `step` 番目の値。`choices` は `[低い値, 高い値]` なので、下りは
/// 始点と終点を入れ替えて使う。
///
/// `span.start` が始点の値、`span.end` が終点の値ちょうどになり、区間の外は
/// 端の値でクランプする。幅が潰れている区間では始点の値で一定。
pub(super) fn ramp_value(choices: [u8; 2], ascending: bool, step: usize, span: RampSpan) -> u8 {
    let [low, high] = choices.map(f32::from);
    let (from, to) = if ascending { (low, high) } else { (high, low) };
    let Some(width) = span.end.checked_sub(span.start).filter(|width| *width > 0) else {
        return from.round() as u8;
    };
    let position = step.clamp(span.start, span.end) - span.start;
    let ratio = position as f32 / width as f32;
    (from + (to - from) * ratio).round() as u8
}

#[cfg(test)]
mod tests;
