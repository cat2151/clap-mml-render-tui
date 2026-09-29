//! effect chain overlay のテストで使う、マシンに依存しない catalog。

use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, EffectPlugins};
use serde_json::Value;

/// ギターアンプ 1 つだけの catalog。
pub(crate) fn amp_plugins() -> EffectPlugins {
    let amp = AudioEffectPluginInfo::new(
        "Test Amp",
        "/clap/does-not-exist-amp.clap",
        "org.example.amp",
        "/presets/does-not-exist-amp",
    );
    let clean = AudioEffectPreset {
        plugin: amp.key.clone(),
        json_key: amp.json_key.clone(),
        value: "Clean".to_string(),
        display: "Test Amp: Clean".to_string(),
        name: "Clean".to_string(),
        category: "Distortion / Saturation".to_string(),
        kind: "Amp Simulator".to_string(),
        path: std::path::PathBuf::from("/presets/does-not-exist-amp/Clean"),
    };
    EffectPlugins::with_catalog(AudioEffectCatalog::with_entries(vec![amp], vec![clean]))
}

pub(crate) fn amp_stage() -> Value {
    serde_json::json!({"Test Amp preset": "Clean"})
}
