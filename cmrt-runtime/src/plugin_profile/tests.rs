use super::*;
use crate::configured_patch_dirs;

/// `toml::from_str` は profile を解決しないので、`Config::load()` と同じく
/// `apply_active_plugin_profile()` を明示的に通す。
fn load_from_toml(toml_str: &str) -> anyhow::Result<Config> {
    let mut cfg: Config = toml::from_str(toml_str)?;
    apply_active_plugin_profile(&mut cfg)?;
    Ok(cfg)
}

const SURGE_AND_DEXED_PROFILES: &str = r#"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512

[plugins.surge_xt]
plugin_path = "/clap/Surge XT.clap"
plugin_id   = "org.surge-synth-team.surge-xt"
patches_dirs = ["/surge/patches_factory", "/surge/patches_3rdparty"]

[plugins.dexed]
plugin_path = "/clap/Dexed.clap"
plugin_id   = "com.digital-suburban.dexed"
"#;

#[test]
fn a_config_without_active_plugin_keeps_its_top_level_settings() {
    let cfg = load_from_toml(
        r#"
plugin_path = "/usr/lib/clap/Surge XT.clap"
patches_dirs = ["/surge/patches_factory"]
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#,
    )
    .unwrap();

    assert_eq!(cfg.plugin_path, "/usr/lib/clap/Surge XT.clap");
    assert_eq!(
        cfg.patches_dirs.as_deref(),
        Some(["/surge/patches_factory".to_string()].as_slice())
    );
    assert_eq!(cfg.plugin_id, None);
}

#[test]
fn an_active_profile_is_baked_into_the_top_level_fields() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'dexed'\n{SURGE_AND_DEXED_PROFILES}"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/clap/Dexed.clap");
    assert_eq!(cfg.plugin_id.as_deref(), Some("com.digital-suburban.dexed"));
}

/// プロファイルに `patches_dirs` を書かなければ「音色置き場は無い」。
/// 他プロファイルや旧トップレベルの Surge 用ディレクトリを流用してはいけない。
#[test]
fn a_profile_without_patches_dirs_has_no_patch_directories() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'dexed'\nplugin_path = '/clap/Surge XT.clap'\n\
         patches_dirs = ['/surge/patches_factory']\n{SURGE_AND_DEXED_PROFILES}"
    ))
    .unwrap();

    assert_eq!(cfg.patches_dirs, None);
    assert!(configured_patch_dirs(&cfg).is_empty());
}

#[test]
fn switching_the_active_profile_switches_the_patch_directories() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'surge_xt'\n{SURGE_AND_DEXED_PROFILES}"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/clap/Surge XT.clap");
    assert_eq!(configured_patch_dirs(&cfg).len(), 2);
}

/// 移行の途中で必ず引っかかるので、併記は conflict error にしない。
#[test]
fn a_profile_wins_over_a_top_level_plugin_path_without_erroring() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'dexed'\nplugin_path = '/clap/Surge XT.clap'\n{SURGE_AND_DEXED_PROFILES}"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/clap/Dexed.clap");
}

#[test]
fn an_active_plugin_without_a_matching_profile_lists_the_available_names() {
    let error = load_from_toml(&format!(
        "active_plugin = 'dxd'\n{SURGE_AND_DEXED_PROFILES}"
    ))
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("dxd"));
    assert!(message.contains("dexed"));
    assert!(message.contains("surge_xt"));
}

#[test]
fn an_active_profile_with_an_empty_plugin_path_is_an_error() {
    let error = load_from_toml(
        r#"
active_plugin = 'broken'
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512

[plugins.broken]
plugin_path = "   "
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("plugin_path"));
}
