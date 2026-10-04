//! フレーズ共通のビブラートの待機・立ち上がり・最終深さ。速度は既存の CC21 にだけ保存する。

use serde::{Deserialize, Serialize};

use crate::{RuleTable, VIBRATO_DEPTH};

/// 待機と立ち上がりの上限（ms）。
pub const VIBRATO_TIME_MAX_MS: u16 = 5000;

/// 列の vibrato ON/OFF と独立した、フレーズ共通の深さ変化の設定。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VibratoSettings {
    pub delay_ms: u16,
    pub rise_ms: u16,
    pub depth: u8,
}

impl Default for VibratoSettings {
    fn default() -> Self {
        Self {
            delay_ms: 300,
            rise_ms: 400,
            depth: VIBRATO_DEPTH,
        }
    }
}

impl VibratoSettings {
    pub(crate) fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub(crate) fn bounded(self) -> Self {
        Self {
            delay_ms: self.delay_ms.min(VIBRATO_TIME_MAX_MS),
            rise_ms: self.rise_ms.min(VIBRATO_TIME_MAX_MS),
            depth: self.depth.min(127),
        }
    }
}

impl RuleTable {
    pub fn vibrato_settings(&self) -> VibratoSettings {
        self.vibrato
    }

    /// 値を入力範囲内に収めて置く。設定が変わったら `true`。速度は [`Self::param`] の CC21 を使う。
    pub fn set_vibrato_settings(&mut self, settings: VibratoSettings) -> bool {
        let settings = settings.bounded();
        let changed = self.vibrato != settings;
        self.vibrato = settings;
        changed
    }
}

#[cfg(test)]
mod tests;
