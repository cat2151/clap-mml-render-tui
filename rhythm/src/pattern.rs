//! リズム型と、1小節の中での Attack 位置。
//!
//! 1 step = 16分音符・16 step = 1小節という前提はこの module の外（呼び出し側の grid）に
//! ある。ここは「何 step ごとに鳴らすか」しか知らない。

use rand::RngExt;

use crate::role::DrumRole;

/// リズム型1つの Attack 位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placement {
    /// 鳴らさない。
    Silent,
    /// 小節頭に1音だけ。
    Once,
    /// `offset` step 目から `every` step ごと。
    Every { every: usize, offset: usize },
}

impl Placement {
    /// `steps` step ぶんの Attack 位置を昇順で返す。
    fn attacks(self, steps: usize) -> Vec<usize> {
        match self {
            Self::Silent => Vec::new(),
            Self::Once if steps == 0 => Vec::new(),
            Self::Once => vec![0],
            Self::Every { every, offset } => (offset..steps).step_by(every.max(1)).collect(),
        }
    }
}

/// kick（bass drum）のリズム型。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KickPattern {
    /// 四分音符で4つ打ち。
    #[default]
    Quarter,
    /// 小節頭に1音だけ。
    Whole,
    Silent,
}

impl KickPattern {
    /// wheel の種別送りで巡回する順序。
    pub const ALL: [KickPattern; 3] = [Self::Quarter, Self::Whole, Self::Silent];

    pub fn label(self) -> &'static str {
        match self {
            Self::Quarter => "4th",
            Self::Whole => "Whole",
            Self::Silent => "Silent",
        }
    }

    fn placement(self) -> Placement {
        match self {
            Self::Quarter => QUARTER,
            Self::Whole => Placement::Once,
            Self::Silent => Placement::Silent,
        }
    }
}

/// snare のリズム型。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SnarePattern {
    /// 2・4拍（16 step なら step 4 と 12）。八分裏ではない。
    #[default]
    Backbeat,
    Silent,
}

impl SnarePattern {
    /// wheel の種別送りで巡回する順序。
    pub const ALL: [SnarePattern; 2] = [Self::Backbeat, Self::Silent];

    pub fn label(self) -> &'static str {
        match self {
            Self::Backbeat => "Backbeat",
            Self::Silent => "Silent",
        }
    }

    fn placement(self) -> Placement {
        match self {
            // 1拍飛ばしの四分音符。offset 4 = 2拍目から。
            Self::Backbeat => Placement::Every {
                every: 8,
                offset: 4,
            },
            Self::Silent => Placement::Silent,
        }
    }
}

/// hi-hat のリズム型。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HatPattern {
    /// 八分音符で埋める。
    #[default]
    Eighth,
    /// 16分音符で埋める。
    Sixteenth,
    Silent,
}

impl HatPattern {
    /// wheel の種別送りで巡回する順序。
    pub const ALL: [HatPattern; 3] = [Self::Eighth, Self::Sixteenth, Self::Silent];

    pub fn label(self) -> &'static str {
        match self {
            Self::Eighth => "8beat",
            Self::Sixteenth => "16beat",
            Self::Silent => "Silent",
        }
    }

    fn placement(self) -> Placement {
        match self {
            Self::Eighth => EIGHTH,
            Self::Sixteenth => SIXTEENTH,
            Self::Silent => Placement::Silent,
        }
    }
}

/// percussion のリズム型。他の役割の型を合併したもの。
///
/// 「kick / snare / hi-hat に取られなかった残り全部」を鳴らす行なので、
/// どの刻みが合うかは当たった patch 次第。だから list も広く取ってある。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PercPattern {
    #[default]
    Quarter,
    Eighth,
    Sixteenth,
    /// 小節頭に1音だけ。
    Whole,
    Silent,
}

impl PercPattern {
    /// wheel の種別送りで巡回する順序。
    pub const ALL: [PercPattern; 5] = [
        Self::Quarter,
        Self::Eighth,
        Self::Sixteenth,
        Self::Whole,
        Self::Silent,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Quarter => "4th",
            Self::Eighth => "8beat",
            Self::Sixteenth => "16beat",
            Self::Whole => "Whole",
            Self::Silent => "Silent",
        }
    }

    fn placement(self) -> Placement {
        match self {
            Self::Quarter => QUARTER,
            Self::Eighth => EIGHTH,
            Self::Sixteenth => SIXTEENTH,
            Self::Whole => Placement::Once,
            Self::Silent => Placement::Silent,
        }
    }
}

/// 四分音符。
const QUARTER: Placement = Placement::Every {
    every: 4,
    offset: 0,
};
/// 八分音符。
const EIGHTH: Placement = Placement::Every {
    every: 2,
    offset: 0,
};
/// 16分音符。
const SIXTEENTH: Placement = Placement::Every {
    every: 1,
    offset: 0,
};

/// 役割つきのリズム型。役割ごとに list が違うので、送りも表示もこの型の上で行う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrumPattern {
    Kick(KickPattern),
    Snare(SnarePattern),
    Hat(HatPattern),
    Perc(PercPattern),
}

impl DrumPattern {
    /// この型が属する役割。
    pub fn role(self) -> DrumRole {
        match self {
            Self::Kick(_) => DrumRole::Kick,
            Self::Snare(_) => DrumRole::Snare,
            Self::Hat(_) => DrumRole::HiHat,
            Self::Perc(_) => DrumRole::Percussion,
        }
    }

    /// 役割の既定の型。wheel を一度も回していない行が鳴らすもの。
    pub fn default_for(role: DrumRole) -> Self {
        match role {
            DrumRole::Kick => Self::Kick(KickPattern::default()),
            DrumRole::Snare => Self::Snare(SnarePattern::default()),
            DrumRole::HiHat => Self::Hat(HatPattern::default()),
            DrumRole::Percussion => Self::Perc(PercPattern::default()),
        }
    }

    /// 譜面の引き直し（`r` / AUTO サイクル）で当てる型。
    ///
    /// kick / snare / hi-hat はビートの土台なので抽選せず既定のままにする（`Silent` を
    /// 引くとリズムそのものが消えてしまう）。percussion だけは list 全体から引いて、
    /// 引き直しごとの表情を変える。刻みを変えたいときは wheel で送る。
    pub fn random_for(role: DrumRole, rng: &mut impl RngExt) -> Self {
        if role != DrumRole::Percussion {
            return Self::default_for(role);
        }
        Self::Perc(PercPattern::ALL[rng.random_range(0..PercPattern::ALL.len())])
    }

    /// 右 pane と log に出す型の名前。
    pub fn label(self) -> &'static str {
        match self {
            Self::Kick(pattern) => pattern.label(),
            Self::Snare(pattern) => pattern.label(),
            Self::Hat(pattern) => pattern.label(),
            Self::Perc(pattern) => pattern.label(),
        }
    }

    /// 役割の list を1つ送った型。末尾からは先頭へ戻る。
    pub fn next(self) -> Self {
        self.cycled(true)
    }

    /// 役割の list を1つ戻した型。先頭からは末尾へ戻る。
    pub fn previous(self) -> Self {
        self.cycled(false)
    }

    /// 役割の list に並ぶ全ての型。右 pane の section へ出す。
    pub fn all_for(role: DrumRole) -> Vec<Self> {
        match role {
            DrumRole::Kick => KickPattern::ALL.iter().copied().map(Self::Kick).collect(),
            DrumRole::Snare => SnarePattern::ALL.iter().copied().map(Self::Snare).collect(),
            DrumRole::HiHat => HatPattern::ALL.iter().copied().map(Self::Hat).collect(),
            DrumRole::Percussion => PercPattern::ALL.iter().copied().map(Self::Perc).collect(),
        }
    }

    fn cycled(self, forward: bool) -> Self {
        match self {
            Self::Kick(pattern) => Self::Kick(cycled(&KickPattern::ALL, pattern, forward)),
            Self::Snare(pattern) => Self::Snare(cycled(&SnarePattern::ALL, pattern, forward)),
            Self::Hat(pattern) => Self::Hat(cycled(&HatPattern::ALL, pattern, forward)),
            Self::Perc(pattern) => Self::Perc(cycled(&PercPattern::ALL, pattern, forward)),
        }
    }

    fn placement(self) -> Placement {
        match self {
            Self::Kick(pattern) => pattern.placement(),
            Self::Snare(pattern) => pattern.placement(),
            Self::Hat(pattern) => pattern.placement(),
            Self::Perc(pattern) => pattern.placement(),
        }
    }
}

/// `all` の中で `current` の隣を返す。端は巻き戻る。
fn cycled<T: Copy + PartialEq>(all: &[T], current: T, forward: bool) -> T {
    let index = all
        .iter()
        .position(|item| *item == current)
        .expect("ALL contains every variant");
    let delta = if forward { 1 } else { all.len() - 1 };
    all[(index + delta) % all.len()]
}

/// ドラム1音。音高は持たない（drum 行は1 instance につき1音しか鳴らさない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrumHit {
    pub step: usize,
    /// 次の音が鳴るまでの長さ。最後の音は小節末まで伸びる。
    pub duration_steps: usize,
}

/// `pattern` を `steps` step ぶん展開する。`steps` が 0 なら空。
///
/// note は次の Attack まで伸ばしっぱなしにする。ドラムの音は patch 側で減衰して消えるので、
/// 長さを別に決めるより素直で、note off の取りこぼしも起きない。
pub fn generate_drum(pattern: DrumPattern, steps: usize) -> Vec<DrumHit> {
    let attacks = pattern.placement().attacks(steps);
    attacks
        .iter()
        .enumerate()
        .map(|(index, step)| DrumHit {
            step: *step,
            duration_steps: attacks.get(index + 1).copied().unwrap_or(steps) - step,
        })
        .collect()
}

#[cfg(test)]
mod tests;
