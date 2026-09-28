//! 試聴先（notepad の行・DAW の track）の effect chain のうち、auto reverb が受け持つ 1 段。
//!
//! auto reverb は chain に書いた段を、同じ JSON の [`AUTO_REVERB_JSON_KEY`] にも控える。
//! chain の中で控えと一致する段だけを auto reverb の段として入れ替え、それ以外の段は位置ごと残す。
//!
//! 次のどちらかなら手動 reverb とみなし、控えを消して [`MANUAL_REVERB_JSON_KEY`] を書く。
//! 手動 reverb の chain には auto reverb は触らない。
//! - 控えがあるのに、控えと一致する段が chain に無い（reverb を差し替えた・bypass した・消した）
//! - 控えが無いのに、chain に reverb がある

use cmrt_core::{
    AudioEffectCatalog, AudioEffectPreset, EFFECT_CHAIN_JSON_KEY, EFFECT_STAGE_BYPASS_JSON_KEY,
};
use serde_json::{Map, Value};

use super::REVERB_KIND;

/// auto reverb が chain に書いた段を控える key。値は chain に書いた段そのもの。
pub const AUTO_REVERB_JSON_KEY: &str = "auto reverb";

/// chain の reverb が手動だと示す key。値は `true`。
pub const MANUAL_REVERB_JSON_KEY: &str = "manual reverb";

/// 試聴先の chain を、auto reverb が入れ替えない段と、auto reverb の扱いに分けたもの。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HostChain {
    /// auto reverb の段を除いた chain。
    kept_stages: Vec<Value>,
    reverb: ReverbOwner,
}

#[derive(Clone, Debug, PartialEq)]
enum ReverbOwner {
    /// auto reverb が受け持つ。`at` は auto reverb の段があった位置（無ければ末尾へ入れる）。
    Auto { at: Option<usize> },
    /// 手動 reverb。`newly_detected` は JSON にまだ印が無い（host が書く）。
    Manual { newly_detected: bool },
}

impl Default for ReverbOwner {
    fn default() -> Self {
        Self::Auto { at: None }
    }
}

impl HostChain {
    /// 行頭 JSON（DAW は init セルの JSON）から組む。`catalog` は chain の段が reverb かを見るのに使う。
    pub fn from_json(json: Option<&Value>, catalog: Option<&AudioEffectCatalog>) -> Self {
        let chain = json
            .and_then(|json| json.get(EFFECT_CHAIN_JSON_KEY))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let marked_manual = json
            .and_then(|json| json.get(MANUAL_REVERB_JSON_KEY))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if marked_manual {
            return Self::manual(chain, false);
        }
        match json.and_then(|json| json.get(AUTO_REVERB_JSON_KEY)) {
            Some(recorded) => match chain.iter().position(|stage| stage == recorded) {
                Some(at) => {
                    let mut kept_stages = chain;
                    kept_stages.remove(at);
                    Self {
                        kept_stages,
                        reverb: ReverbOwner::Auto { at: Some(at) },
                    }
                }
                None => Self::manual(chain, true),
            },
            None if catalog.is_some_and(|catalog| {
                chain
                    .iter()
                    .any(|stage| reverb_preset(catalog, stage).is_some())
            }) =>
            {
                Self::manual(chain, true)
            }
            None => Self {
                kept_stages: chain,
                reverb: ReverbOwner::Auto { at: None },
            },
        }
    }

    fn manual(chain: Vec<Value>, newly_detected: bool) -> Self {
        Self {
            kept_stages: chain,
            reverb: ReverbOwner::Manual { newly_detected },
        }
    }

    /// 手動 reverb か。手動なら auto reverb は掛けない。
    pub fn is_manual_reverb(&self) -> bool {
        matches!(self.reverb, ReverbOwner::Manual { .. })
    }

    /// 手動 reverb を今回見つけ、JSON にまだ印が無い。host は [`Self::write_into`] で印を書く。
    pub fn newly_detected_manual_reverb(&self) -> bool {
        self.reverb
            == ReverbOwner::Manual {
                newly_detected: true,
            }
    }

    /// auto reverb が入れ替えない段のうち、reverb の段の表示名。bypass 中の段には ` (bypass)` を付ける。
    pub fn reverb_labels(&self, catalog: Option<&AudioEffectCatalog>) -> Vec<String> {
        let Some(catalog) = catalog else {
            return Vec::new();
        };
        self.kept_stages
            .iter()
            .filter_map(|stage| {
                let preset = reverb_preset(catalog, stage)?;
                let bypassed = stage
                    .get(EFFECT_STAGE_BYPASS_JSON_KEY)
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Some(if bypassed {
                    format!("{} (bypass)", preset.display)
                } else {
                    preset.display.clone()
                })
            })
            .collect()
    }

    /// `auto_reverb` を auto reverb の段として入れた chain。元の位置、無ければ末尾へ入れる。
    /// `None` か手動 reverb なら、入れ替えない段だけ。
    pub fn with_auto_reverb(&self, auto_reverb: Option<&Value>) -> Vec<Value> {
        let mut chain = self.kept_stages.clone();
        if let (ReverbOwner::Auto { at }, Some(stage)) = (&self.reverb, auto_reverb) {
            let index = at.unwrap_or(chain.len()).min(chain.len());
            chain.insert(index, stage.clone());
        }
        chain
    }

    /// JSON の object へ chain と印を書く。空の chain は key ごと消す。
    /// 手動 reverb なら控えを消して手動の印を書き、そうでなければ控えを `auto_reverb` にする。
    pub fn write_into(&self, object: &mut Map<String, Value>, auto_reverb: Option<&Value>) {
        let chain = self.with_auto_reverb(auto_reverb);
        set_or_remove(
            object,
            EFFECT_CHAIN_JSON_KEY,
            (!chain.is_empty()).then_some(Value::Array(chain)),
        );
        if self.is_manual_reverb() {
            object.remove(AUTO_REVERB_JSON_KEY);
            object.insert(MANUAL_REVERB_JSON_KEY.to_string(), Value::Bool(true));
        } else {
            set_or_remove(object, AUTO_REVERB_JSON_KEY, auto_reverb.cloned());
        }
    }
}

fn set_or_remove(object: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    match value {
        Some(value) => {
            object.insert(key.to_string(), value);
        }
        None => {
            object.remove(key);
        }
    }
}

/// 段のどれかの key が catalog の reverb を指すなら、その preset。
fn reverb_preset<'a>(
    catalog: &'a AudioEffectCatalog,
    stage: &Value,
) -> Option<&'a AudioEffectPreset> {
    stage.as_object()?.iter().find_map(|(json_key, value)| {
        catalog
            .find(json_key, value.as_str()?)
            .ok()
            .filter(|preset| preset.kind == REVERB_KIND)
    })
}

#[cfg(test)]
mod tests;
