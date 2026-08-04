//! 小節ごとに引く、レーン値の決め方。
//!
//! CC1 と velocity は1回の抽選結果を分け合う。「両方まとめて完全ランダム」と
//! 「上り下りの組み合わせ4通り」の計5通りを均等に引くので、小節単位で
//! クレッシェンド／デクレッシェンドの起伏が付く。

use rand::Rng;

use crate::GRID_STEPS;

/// 1小節ぶんのレーン値の決め方。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::state) enum MeasurePattern {
    /// セル単位で2値から抽選する。
    Random,
    /// 小節頭から末尾へ線形に変化する。`ascending` は `choices[0]` から
    /// `choices[1]` へ上がる向き。
    Ramp { ascending: bool },
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
    pub(in crate::state) fn draw(rng: &mut impl Rng) -> Self {
        Self::from_plan(PLANS[rng.gen_range(0..PLANS.len())])
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
/// step 0 が小節頭の値、step `GRID_STEPS - 1` が末尾の値ちょうどになる。
pub(super) fn ramp_value(choices: [u8; 2], ascending: bool, step: usize) -> u8 {
    let [low, high] = choices.map(f32::from);
    let (start, end) = if ascending { (low, high) } else { (high, low) };
    let ratio = step as f32 / (GRID_STEPS - 1) as f32;
    (start + (end - start) * ratio).round() as u8
}

#[cfg(test)]
mod tests;
