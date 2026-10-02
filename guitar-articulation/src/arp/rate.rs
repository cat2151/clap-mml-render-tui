//! chord 素材のアルペジオの音価。MML 素材は MML の `t` と `l` が刻みを決めるので使わない。

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 1 step の音価。`8t` は 8 分 3 連（1 拍を 3 等分）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArpRate {
    Eighth,
    EighthTriplet,
    #[default]
    Sixteenth,
    SixteenthTriplet,
    ThirtySecond,
}

impl ArpRate {
    /// overlay の `r` / `R` で巡る並び。
    pub const ALL: [ArpRate; 5] = [
        ArpRate::Eighth,
        ArpRate::EighthTriplet,
        ArpRate::Sixteenth,
        ArpRate::SixteenthTriplet,
        ArpRate::ThirtySecond,
    ];

    /// 設定・見出しの綴り（`8` / `8t` / `16` / `16t` / `32`）。
    pub fn label(self) -> &'static str {
        match self {
            ArpRate::Eighth => "8",
            ArpRate::EighthTriplet => "8t",
            ArpRate::Sixteenth => "16",
            ArpRate::SixteenthTriplet => "16t",
            ArpRate::ThirtySecond => "32",
        }
    }

    /// overlay の設定行の綴り。
    pub fn name(self) -> &'static str {
        match self {
            ArpRate::Eighth => "8分",
            ArpRate::EighthTriplet => "8分3連",
            ArpRate::Sixteenth => "16分",
            ArpRate::SixteenthTriplet => "16分3連",
            ArpRate::ThirtySecond => "32分",
        }
    }

    /// 全音符を何等分した長さか。
    fn divisions(self) -> f64 {
        match self {
            ArpRate::Eighth => 8.0,
            ArpRate::EighthTriplet => 12.0,
            ArpRate::Sixteenth => 16.0,
            ArpRate::SixteenthTriplet => 24.0,
            ArpRate::ThirtySecond => 32.0,
        }
    }

    /// `bpm` の 4 分音符を 1 拍として、この音価 1 つの秒。
    pub fn seconds(self, bpm: u16) -> f64 {
        60.0 / f64::from(bpm) * 4.0 / self.divisions()
    }

    /// [`Self::ALL`] の並びで `delta` だけ動かし、端で止める。
    pub fn stepped(self, delta: isize) -> ArpRate {
        let index = Self::ALL.iter().position(|rate| *rate == self).unwrap_or(0);
        let moved = index.saturating_add_signed(delta).min(Self::ALL.len() - 1);
        Self::ALL[moved]
    }
}

impl Serialize for ArpRate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.label())
    }
}

/// 知らない綴りは既定（16 分）。
impl<'de> Deserialize<'de> for ArpRate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let label = String::deserialize(deserializer)?;
        Ok(Self::ALL
            .into_iter()
            .find(|rate| rate.label() == label)
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests;
