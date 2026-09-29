//! chain の 1 段（`{json_key: value}`）と chain 全体の、表示・bypass・送る綴り。

use cmrt_core::{AudioEffectCatalog, EFFECT_STAGE_BYPASS_JSON_KEY};
use serde_json::Value;

/// chain の 1 段が bypass されているか（`EFFECT_STAGE_BYPASS_JSON_KEY` が `true`）。
pub fn stage_is_bypassed(stage: &Value) -> bool {
    stage
        .as_object()
        .and_then(|object| object.get(EFFECT_STAGE_BYPASS_JSON_KEY))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// chain の 1 段に bypass を付け外しした値を返す。plugin を決めるキーは触らない。
/// `bypass` は `false` にするとキーごと消す（付いていないのと同じ形に戻す）。
pub fn stage_with_bypass(stage: &Value, bypass: bool) -> Value {
    let mut object = stage.as_object().cloned().unwrap_or_default();
    if bypass {
        object.insert(EFFECT_STAGE_BYPASS_JSON_KEY.to_string(), Value::Bool(true));
    } else {
        object.remove(EFFECT_STAGE_BYPASS_JSON_KEY);
    }
    Value::Object(object)
}

/// chain の 1 段を出す語。catalog にあれば `display`、無ければ JSON のまま
/// （何が書かれているか見えないと消す判断ができない）。bypass 段は前に `[bypass] ` を付ける。
pub fn stage_label(stage: &Value, catalog: Option<&AudioEffectCatalog>) -> String {
    let known = stage.as_object().and_then(|object| {
        let mut plugin_keys = object
            .iter()
            .filter(|(key, _)| key.as_str() != EFFECT_STAGE_BYPASS_JSON_KEY);
        let (json_key, value) = plugin_keys.next()?;
        if plugin_keys.next().is_some() {
            return None;
        }
        let value = value.as_str()?;
        let preset = catalog?.find(json_key, value).ok()?;
        Some(preset.display.clone())
    });
    let label = known.unwrap_or_else(|| stage.to_string());
    if stage_is_bypassed(stage) {
        format!("[bypass] {label}")
    } else {
        label
    }
}

/// realtime play server へ音色と一緒に送る chain の綴り（JSON 配列）。空の chain は空文字列。
pub fn chain_json(chain: &[Value]) -> String {
    if chain.is_empty() {
        String::new()
    } else {
        Value::Array(chain.to_vec()).to_string()
    }
}

#[cfg(test)]
mod tests;
