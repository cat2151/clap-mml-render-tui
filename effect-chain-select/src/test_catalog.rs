//! マシンに依存しない、手で並べた catalog。
//!
//! 候補は catalog 登録順で `Delay/Echo`（`Space / Imaging` / `Delay`）と
//! `Clean`（`Distortion / Saturation` / `Amp Simulator`）。

use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset};

pub(crate) fn catalog() -> AudioEffectCatalog {
    let fx = AudioEffectPluginInfo::new(
        "Test FX",
        "/clap/does-not-exist.clap",
        "org.example.fx",
        "/presets/does-not-exist",
    );
    let amp = AudioEffectPluginInfo::new(
        "Test Amp",
        "/clap/does-not-exist-amp.clap",
        "org.example.amp",
        "/presets/does-not-exist-amp",
    );
    let preset = |plugin: &AudioEffectPluginInfo, value: &str, category: &str, kind: &str| {
        AudioEffectPreset {
            plugin: plugin.key.clone(),
            json_key: plugin.json_key.clone(),
            value: value.to_string(),
            display: format!("{}: {value}", plugin.name),
            name: value.to_string(),
            category: category.to_string(),
            kind: kind.to_string(),
            path: std::path::PathBuf::from(format!("/presets/{value}")),
        }
    };
    let presets = vec![
        preset(&fx, "Delay/Echo", "Space / Imaging", "Delay"),
        preset(&amp, "Clean", "Distortion / Saturation", "Amp Simulator"),
    ];
    AudioEffectCatalog::with_entries(vec![fx, amp], presets)
}
