use super::*;

#[test]
fn core_config_from_config_disables_random_patch() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 44100
buffer_size = 512
patches_dirs = ["/tmp/surge-data/patches_factory", "/tmp/surge-data/patches_3rdparty"]
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();
    let core_cfg = core_config_from_config(&cfg);
    assert!(
        !core_cfg.random_patch,
        "Config から生成した CoreConfig は常に random_patch=false にする"
    );
    assert_eq!(core_cfg.patches_dir.as_deref(), Some("/tmp/surge-data"));
    assert_eq!(core_cfg.output_wav, "output.wav");
    assert_eq!(core_cfg.buffer_size, 512);
}

/// `plugin_id` を `CoreConfig` まで運べないと、descriptor を複数持つ CLAP で
/// instance 生成が「どれを使うか決められない」で落ちる。
#[test]
fn core_config_from_config_carries_plugin_id() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Dexed.clap"
plugin_id   = "com.digital-suburban.dexed"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();

    let core_cfg = core_config_from_config(&cfg);

    assert_eq!(
        core_cfg.plugin_id.as_deref(),
        Some("com.digital-suburban.dexed")
    );
}

/// `active_plugin` の 1 行だけを書いた config でも、組み込みプロファイルの
/// `plugin_id` が `CoreConfig` まで届くこと。ユーザーが実際に書く形はこちら。
#[test]
fn core_config_from_config_carries_plugin_id_resolved_from_active_plugin() {
    let mut cfg: Config = toml::from_str(
        r#"
active_plugin = "Dexed"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#,
    )
    .unwrap();
    cmrt_runtime::apply_active_plugin_profile(&mut cfg).unwrap();

    let core_cfg = core_config_from_config(&cfg);

    assert_eq!(
        core_cfg.plugin_id.as_deref(),
        Some("com.digital-suburban.dexed")
    );
}

/// 従来どおり `active_plugin` を書かない config では `None` のまま。
/// ここが `Some` になると、descriptor が 1 件しかない CLAP でも ID 不一致で落ちうる。
#[test]
fn core_config_from_config_leaves_plugin_id_unset_when_config_omits_it() {
    let toml_str = r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#;
    let cfg: Config = toml::from_str(toml_str).unwrap();

    assert_eq!(core_config_from_config(&cfg).plugin_id, None);
}
