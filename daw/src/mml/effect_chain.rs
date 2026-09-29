//! init セルの JSON にある effect chain（`"effects after instrument"`）の読み書き。
//!
//! chain の中身は解釈しない（キーの意味は play server の catalog が持つ）。
//! DAW は配列の要素を値のまま出し入れするだけ。

use cmrt_core::{AudioEffectCatalog, EFFECT_CHAIN_JSON_KEY};
use cmrt_patch_select::auto_reverb::{HostChain, AUTO_REVERB_JSON_KEY, MANUAL_REVERB_JSON_KEY};
use serde_json::{Map, Value};

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

/// init セルの chain と、auto reverb の扱い。
pub(crate) fn init_cell_host_chain(
    init_cell: &str,
    catalog: Option<&AudioEffectCatalog>,
) -> HostChain {
    HostChain::from_json(split_mml_fragment(init_cell).json.as_ref(), catalog)
}

/// init セルの chain と auto/manual reverb の印を `chain` と `auto_reverb` から書き直す。他のキーは残す。
/// 3 つの key がどれも変わらなければ、init セルを書式ごとそのまま返す。
pub(crate) fn init_cell_with_host_chain(
    init_cell: &str,
    chain: &HostChain,
    auto_reverb: Option<&Value>,
) -> String {
    let mut effects = Map::new();
    chain.write_into(&mut effects, auto_reverb);
    let entries = [
        EFFECT_CHAIN_JSON_KEY,
        AUTO_REVERB_JSON_KEY,
        MANUAL_REVERB_JSON_KEY,
    ]
    .map(|key| (key, effects.get(key).cloned()));
    let current = split_mml_fragment(init_cell).json;
    if entries
        .iter()
        .all(|(key, value)| current.as_ref().and_then(|json| json.get(*key)) == value.as_ref())
    {
        return init_cell.to_string();
    }
    init_cell_with_json_values(init_cell, &entries)
}

#[cfg(test)]
mod tests;
