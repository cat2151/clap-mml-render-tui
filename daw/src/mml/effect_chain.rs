//! init セルの JSON にある effect chain（`"effects after instrument"`）の読み書き。
//!
//! chain の中身は解釈しない（キーの意味は play server の catalog が持つ）。
//! DAW は配列の要素を値のまま出し入れするだけ。

use cmrt_core::{EFFECT_CHAIN_JSON_KEY, EFFECT_STAGE_BYPASS_JSON_KEY};
use serde_json::Value;

use super::{init_cell_with_json_values, split_mml_fragment};

/// init セルの chain。キーが無い・配列でないときは空。
pub(crate) fn init_cell_effect_chain(init_cell: &str) -> Vec<Value> {
    split_mml_fragment(init_cell)
        .json
        .as_ref()
        .and_then(|json| json.get(EFFECT_CHAIN_JSON_KEY))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// init セルの chain を書き換える。他のキーは残す。空の chain はキーごと消す。
pub(crate) fn init_cell_with_effect_chain(init_cell: &str, chain: &[Value]) -> String {
    let value = (!chain.is_empty()).then(|| Value::Array(chain.to_vec()));
    init_cell_with_json_values(init_cell, &[(EFFECT_CHAIN_JSON_KEY, value)])
}

/// chain の 1 段が bypass されているか（`EFFECT_STAGE_BYPASS_JSON_KEY` が `true`）。
pub(crate) fn stage_is_bypassed(stage: &Value) -> bool {
    stage
        .as_object()
        .and_then(|object| object.get(EFFECT_STAGE_BYPASS_JSON_KEY))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// chain の 1 段に bypass を付け外しした値を返す。plugin を決めるキーは触らない。
/// `bypass` は `false` にするとキーごと消す（付いていないのと同じ形に戻す）。
pub(crate) fn stage_with_bypass(stage: &Value, bypass: bool) -> Value {
    let mut object = stage.as_object().cloned().unwrap_or_default();
    if bypass {
        object.insert(EFFECT_STAGE_BYPASS_JSON_KEY.to_string(), Value::Bool(true));
    } else {
        object.remove(EFFECT_STAGE_BYPASS_JSON_KEY);
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests;
