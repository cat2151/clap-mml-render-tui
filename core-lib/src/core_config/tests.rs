use super::*;

fn primary_config(extra: &str) -> Config {
    let mut cfg: Config = toml::from_str(&format!(
        r#"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
{extra}
"#
    ))
    .unwrap();
    cmrt_runtime::apply_primary_plugin_profile(&mut cfg).unwrap();
    cfg
}

#[test]
fn core_config_from_config_disables_random_patch() {
    let cfg = primary_config(
        r#"
[plugins."Surge XT"]
patches_dirs = ["/tmp/surge-data/patches_factory", "/tmp/surge-data/patches_3rdparty"]
"#,
    );

    let core_cfg = core_config_from_config(&cfg);

    assert!(
        !core_cfg.random_patch,
        "Config から生成した CoreConfig は常に random_patch=false にする"
    );
    assert_eq!(core_cfg.patches_dir.as_deref(), Some("/tmp/surge-data"));
    assert_eq!(core_cfg.output_wav, "output.wav");
    assert_eq!(core_cfg.buffer_size, 512);
}

/// 固定 Surge XT の `plugin_id` が `CoreConfig` まで届くこと。
#[test]
fn core_config_from_config_carries_the_primary_plugin_id() {
    let cfg = primary_config("");

    let core_cfg = core_config_from_config(&cfg);

    assert_eq!(
        core_cfg.plugin_id.as_deref(),
        Some(cmrt_runtime::SURGE_XT_PLUGIN_ID)
    );
}

/// 混在カタログでは、選んだ plugin 自身の ID と base を運ぶ。
#[test]
fn core_config_for_plugin_carries_that_plugins_identity() {
    let cfg = primary_config("");
    let dexed = CatalogPlugin {
        name: "Dexed".to_string(),
        plugin_path: "/usr/lib/clap/Dexed.clap".to_string(),
        plugin_id: Some(cmrt_runtime::DEXED_PLUGIN_ID.to_string()),
        base: Some("/dexed/cartridges".to_string()),
        dirs: vec!["/dexed/cartridges".to_string()],
        resolved_patches: None,
        source_notices: Vec::new(),
    };

    let core_cfg = core_config_for_plugin(&cfg, &dexed);

    assert_eq!(
        core_cfg.plugin_id.as_deref(),
        Some(cmrt_runtime::DEXED_PLUGIN_ID)
    );
    assert_eq!(core_cfg.patches_dir.as_deref(), Some("/dexed/cartridges"));
}
