//! effect を持たない音色の試聴に自動で掛ける reverb（auto reverb）の、ルールと解決。
//!
//! ルールは「Role の builtin preset ごと」と「Role ごとのその他」の行に、reverb を 1 つ
//! （または dry）を紐付けたもの。音色がどの行に当たるかは [`PatchRoleIndex`] の分類をそのまま使う。
//! ここは純粋な解決だけを持ち、保存（history）と送信（host）は持たない。

use std::collections::BTreeMap;

use cmrt_core::{AudioEffectCatalog, AudioEffectPreset};
use cmrt_patches::{builtin_role_presets, PatchRole, PatchRoleIndex};
use serde_json::Value;

/// effect 追加の selector から外す preset の値の前方一致。catalog そのものからは外さない。
const EXCLUDED_VALUE_PREFIXES: [&str; 1] = ["Reverb 1/"];

/// ルールで選べる effect の分類。
const REVERB_KIND: &str = "Reverb";

const DRUM_ROOM_JSON_KEY: &str = "Dragonfly Room Reverb preset";
const DRUM_ROOM_VALUE: &str = "Small Drum Room";
const HALL_JSON_KEY: &str = "Dragonfly Hall Reverb preset";
const HALL_VALUE: &str = "Medium Clear Hall";

/// 既定で dry にする builtin preset の label。
const DRY_BY_DEFAULT: [&str; 2] = ["bass|bs", "kick|bass drum"];

/// effect を選ぶ selector に出してよい preset か。既存 chain の段の扱いには使わない。
pub fn is_selectable_effect_preset(preset: &AudioEffectPreset) -> bool {
    !EXCLUDED_VALUE_PREFIXES
        .iter()
        .any(|prefix| preset.value.starts_with(prefix))
}

/// ルールに選べる reverb の一覧（catalog 登録順）。
pub fn reverb_candidates(catalog: &AudioEffectCatalog) -> Vec<&AudioEffectPreset> {
    catalog
        .presets()
        .iter()
        .filter(|preset| preset.kind == REVERB_KIND && is_selectable_effect_preset(preset))
        .collect()
}

/// ルールの 1 行。`label` が `None` なら、その Role のどの builtin preset にも当たらない音色の行。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoReverbRow {
    role: PatchRole,
    label: Option<&'static str>,
}

impl AutoReverbRow {
    pub fn role(&self) -> PatchRole {
        self.role
    }

    /// 表示名であり、保存時のキーでもある。
    pub fn name(&self) -> String {
        match self.label {
            Some(label) => label.to_string(),
            None => format!("{} その他", self.role.key()),
        }
    }

    fn default_effect(&self) -> Option<Value> {
        match self.label {
            Some(label) if DRY_BY_DEFAULT.contains(&label) => None,
            _ if self.role == PatchRole::Drum => Some(stage(DRUM_ROOM_JSON_KEY, DRUM_ROOM_VALUE)),
            _ => Some(stage(HALL_JSON_KEY, HALL_VALUE)),
        }
    }
}

fn stage(json_key: &str, value: &str) -> Value {
    serde_json::json!({ json_key: value })
}

/// 全行。Role は selector の並び順、各 Role の中は builtin preset の順で、末尾にその他。
pub fn auto_reverb_rows() -> Vec<AutoReverbRow> {
    PatchRole::ALL
        .into_iter()
        .flat_map(|role| {
            builtin_role_presets()
                .iter()
                .filter(move |preset| preset.role == role)
                .map(move |preset| AutoReverbRow {
                    role,
                    label: Some(preset.label),
                })
                .chain(std::iter::once(AutoReverbRow { role, label: None }))
        })
        .collect()
}

/// 音色が当たる行。索引に無い音色は Etc のその他。
fn row_of(display: &str, role_index: &PatchRoleIndex) -> AutoReverbRow {
    AutoReverbRow {
        role: role_index.role_of(display).unwrap_or(PatchRole::Etc),
        label: role_index.preset_label_of(display),
    }
}

/// auto reverb の on/off と、各行の effect（`None` は dry）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoReverbRules {
    enabled: bool,
    rows: Vec<(AutoReverbRow, Option<Value>)>,
}

impl Default for AutoReverbRules {
    fn default() -> Self {
        Self {
            enabled: true,
            rows: auto_reverb_rows()
                .into_iter()
                .map(|row| {
                    let effect = row.default_effect();
                    (row, effect)
                })
                .collect(),
        }
    }
}

impl AutoReverbRules {
    /// 保存値から組む。保存に無い行は既定値。
    ///
    /// `catalog` があるときは、そこに reverb として無い値を dry にして log に 1 行ずつ出す。
    /// `catalog` が無いときは保存値をそのまま持つ（effect が使えない環境で設定を消さない）。
    pub fn from_saved(
        enabled: bool,
        saved: &BTreeMap<String, Value>,
        catalog: Option<&AudioEffectCatalog>,
    ) -> Self {
        let mut rules = Self {
            enabled,
            ..Self::default()
        };
        for (row, effect) in &mut rules.rows {
            let Some(value) = saved.get(&row.name()) else {
                continue;
            };
            *effect = match value {
                Value::Null => None,
                value if catalog.is_none_or(|catalog| find_reverb(catalog, value).is_some()) => {
                    Some(value.clone())
                }
                value => {
                    crate::log_line(format!(
                        "auto-reverb: 保存値が catalog の reverb に無いので dry にする row='{}' effect={value}",
                        row.name()
                    ));
                    None
                }
            };
        }
        rules
    }

    /// 保存する形。dry の行は `null`。
    pub fn to_saved(&self) -> BTreeMap<String, Value> {
        self.rows
            .iter()
            .map(|(row, effect)| (row.name(), effect.clone().unwrap_or(Value::Null)))
            .collect()
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 行と、その行の effect（`None` は dry）。並びは [`auto_reverb_rows`] と同じ。
    pub fn rows(&self) -> &[(AutoReverbRow, Option<Value>)] {
        &self.rows
    }

    /// `index` 行目の effect を差し替える。範囲外なら何もしない。
    pub fn set_effect(&mut self, index: usize, effect: Option<Value>) {
        if let Some((_, slot)) = self.rows.get_mut(index) {
            *slot = effect;
        }
    }

    fn effect_of(&self, row: AutoReverbRow) -> Option<&Value> {
        self.rows
            .iter()
            .find(|(candidate, _)| *candidate == row)
            .and_then(|(_, effect)| effect.as_ref())
    }
}

/// `{json_key: value}` の形の値を catalog の reverb 候補から引く。
fn find_reverb<'a>(
    catalog: &'a AudioEffectCatalog,
    value: &Value,
) -> Option<&'a AudioEffectPreset> {
    let object = value.as_object().filter(|object| object.len() == 1)?;
    let (json_key, preset_value) = object.iter().next()?;
    catalog
        .find(json_key, preset_value.as_str()?)
        .ok()
        .filter(|preset| preset.kind == REVERB_KIND && is_selectable_effect_preset(preset))
}

/// 音色 1 つに対する auto reverb の結果。
#[derive(Clone, Debug, PartialEq)]
pub enum AutoReverb {
    /// `stage` を chain の 1 段として掛ける。`effect_name` は preset 名、`row` は当たった行の名前。
    Apply {
        stage: Value,
        effect_name: String,
        row: String,
    },
    /// 当たった行が dry（または保存値が catalog の reverb に無い）。
    Dry { row: String },
    /// 音色が effect を内蔵している。
    Builtin,
    /// auto reverb が off。
    Off,
    /// effect catalog が使えない。
    NoCatalog,
}

/// 音色 1 つに掛ける auto reverb を決める。
pub fn resolve(
    display: &str,
    has_builtin_effects: bool,
    role_index: &PatchRoleIndex,
    catalog: Option<&AudioEffectCatalog>,
    rules: &AutoReverbRules,
) -> AutoReverb {
    if !rules.enabled() {
        return AutoReverb::Off;
    }
    if has_builtin_effects {
        return AutoReverb::Builtin;
    }
    let Some(catalog) = catalog else {
        return AutoReverb::NoCatalog;
    };
    let row = row_of(display, role_index);
    match rules
        .effect_of(row)
        .and_then(|value| find_reverb(catalog, value))
    {
        Some(preset) => AutoReverb::Apply {
            stage: preset.json_element(),
            effect_name: preset.name.clone(),
            row: row.name(),
        },
        None => AutoReverb::Dry { row: row.name() },
    }
}

#[cfg(test)]
pub(crate) mod tests;
