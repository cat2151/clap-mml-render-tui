//! drum 行の役割。1 role = 1 instance = 1 patch。

/// drum 行が担当する打楽器。
///
/// リズム型（[`super::DrumPattern`]）も patch の抽選条件も、この役割ごとに分かれる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrumRole {
    /// bass drum。
    Kick,
    Snare,
    HiHat,
    /// 上の3つに当てはまらない打楽器全部（clap / cowbell / tom など）。
    Percussion,
}

impl DrumRole {
    /// 画面の下から kick・snare・hi-hat・percussion の順に並べるので、instance index の
    /// 昇順（＝画面の上から）ではこの並びになる。
    pub const ALL: [DrumRole; 4] = [Self::Percussion, Self::HiHat, Self::Snare, Self::Kick];

    /// grid の NOTE 欄へ出す短いラベル。
    ///
    /// drum 行は音高が意味を持たない（[`super::DrumPattern`] が鳴らす音は1つだけ）ので、
    /// note 名の代わりにこれを出す。
    pub fn label(self) -> &'static str {
        match self {
            Self::Kick => "KICK",
            Self::Snare => "SNR",
            Self::HiHat => "HAT",
            Self::Percussion => "PERC",
        }
    }
}

#[cfg(test)]
mod tests;
