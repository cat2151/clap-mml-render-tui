//! init セルの JSON にある effect chain（`"effects after instrument"`）の読み書き。
//!
//! chain の中身は解釈しない（キーの意味は play server の catalog が持つ）。
//! DAW は配列の要素を値のまま出し入れするだけ。

use cmrt_core::EFFECT_CHAIN_JSON_KEY;
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

#[cfg(test)]
mod tests;
