//! patch selector の auto reverb の on/off と行ごとの effect の永続化。
//!
//! 行の意味と catalog との突き合わせは `cmrt_patch_select::auto_reverb` が持つ。
//! ここは行名 → effect（`{json_key: value}`、dry は `null`）の対応をそのまま読み書きする。

use std::collections::BTreeMap;

use anyhow::Result;

/// 保存された auto reverb の設定。
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutoReverbSettings {
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// 行名 → effect。無い行は既定値、`null` は dry。
    #[serde(default)]
    pub rules: BTreeMap<String, serde_json::Value>,
}

fn enabled_by_default() -> bool {
    true
}

/// 保存が無い・読めないときは `None`（既定のルールを使う）。
pub fn load_auto_reverb_settings() -> Option<AutoReverbSettings> {
    let path = super::paths::auto_reverb_rules_path()?;
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn save_auto_reverb_settings(settings: &AutoReverbSettings) -> Result<()> {
    let Some(path) = super::paths::auto_reverb_rules_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

#[cfg(test)]
mod tests;
