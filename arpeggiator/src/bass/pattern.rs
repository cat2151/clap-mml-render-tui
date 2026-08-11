//! ベースラインの型。1拍の刻み方と、octave 上を混ぜるかどうかだけを持つ。
//!
//! 1 step = 16分音符という前提はこの enum の外（呼び出し側の grid）にある。ここは
//! 「1拍の中に何 step の音を並べるか」しか知らない。

/// ベースラインの型。
///
/// 複数オクターブ・休符入り・シャッフルはまだ持たない。増やすならこの enum へ
/// variant を足し、[`Self::beat_rhythm`] と [`Self::uses_octave`] を実装する。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BassPattern {
    /// 全音符1個を小節いっぱい伸ばす。root のみ。
    #[default]
    Whole,
    /// 八分音符で埋める。root のみ。
    Eighth,
    /// 八分音符で埋め、1音ごとに root と octave 上を往復する。
    EighthOctave,
    /// 1拍を「八分・16分・16分」で刻む（grid 表記で `#-##`）。root のみ。
    EighthTwoSixteenths,
    /// `#-##` を刻み、八分音符ごとに root と octave 上を往復する。
    /// 拍あたり root(八分) → octave(16分) → octave(16分) になる。
    EighthTwoSixteenthsOctave,
    /// 16分音符で埋め、八分音符ごとに root と octave 上を往復する（grid 表記で `####`）。
    /// 拍あたり root・root・octave・octave になる。
    ///
    /// root だけで 16分を刻む型は持たない（八分と聴き分けにくく、送りが1段無駄になる）。
    SixteenthOctave,
}

impl BassPattern {
    /// wheel の種別送りで巡回する順序。
    pub const ALL: [BassPattern; 6] = [
        Self::Whole,
        Self::Eighth,
        Self::EighthOctave,
        Self::EighthTwoSixteenths,
        Self::EighthTwoSixteenthsOctave,
        Self::SixteenthOctave,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Whole => "Whole",
            Self::Eighth => "8th",
            Self::EighthOctave => "8th+Oct",
            Self::EighthTwoSixteenths => "#-##",
            Self::EighthTwoSixteenthsOctave => "#-##+Oct",
            Self::SixteenthOctave => "####+Oct",
        }
    }

    /// [`Self::ALL`] の次の型。末尾からは先頭へ戻る。
    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    /// [`Self::ALL`] の前の型。先頭からは末尾へ戻る。
    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// 1拍（[`super::BEAT_STEPS`] step）の中に並べる音の長さ列。合計は必ず1拍ぶん。
    /// 小節いっぱい伸ばす [`Self::Whole`] だけ `None`。
    pub(super) fn beat_rhythm(self) -> Option<&'static [usize]> {
        match self {
            Self::Whole => None,
            Self::Eighth | Self::EighthOctave => Some(&[2, 2]),
            Self::EighthTwoSixteenths | Self::EighthTwoSixteenthsOctave => Some(&[2, 1, 1]),
            Self::SixteenthOctave => Some(&[1, 1, 1, 1]),
        }
    }

    /// octave 上を混ぜるか。
    pub(super) fn uses_octave(self) -> bool {
        matches!(
            self,
            Self::EighthOctave | Self::EighthTwoSixteenthsOctave | Self::SixteenthOctave
        )
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|pattern| *pattern == self)
            .expect("ALL contains every variant")
    }
}

#[cfg(test)]
mod tests;
