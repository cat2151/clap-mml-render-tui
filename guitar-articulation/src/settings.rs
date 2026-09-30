//! 画面の設定と、専用 file（`guitar_articulation_settings.json`）の読み書き。
//!
//! パスは `cmrt-history` が決め（`guitar_articulation_settings_file_path`）、serde はこちらが持つ。

use serde::{Deserialize, Serialize};

use crate::StartupInstrument;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GuitarArticulationSettings {
    #[serde(default)]
    pub startup_instrument: StartupInstrument,
}

/// 保存済みの設定を読む。file が無い・壊れている・保存先が決まらないときは既定値。
pub fn load_settings() -> GuitarArticulationSettings {
    cmrt_history::guitar_articulation_settings_file_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// 設定を保存する。呼び出し側は失敗をログ 1 行に留め、画面は落とさないこと。
pub fn save_settings(settings: &GuitarArticulationSettings) -> anyhow::Result<()> {
    let path = cmrt_history::guitar_articulation_settings_file_path().ok_or_else(|| {
        anyhow::anyhow!("guitar_articulation_settings.json の保存先を決められない")
    })?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

#[cfg(test)]
mod tests;
