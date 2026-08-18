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

/// プロファイルに `patches_dirs` を書かなければ組み込みの値が残る。
/// 他プロファイルや旧トップレベルの Surge 用ディレクトリを流用してはいけない。
#[test]
fn a_profile_without_patches_dirs_falls_back_to_the_builtin_ones() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'dexed'\nplugin_path = '/clap/Surge XT.clap'\n\
         patches_dirs = ['/surge/patches_factory']\n{SURGE_AND_DEXED_PROFILES}"
    ))
    .unwrap();

    assert_eq!(
        configured_patch_dirs(&cfg),
        crate::default_dexed_cartridge_dirs()
    );
    assert!(!configured_patch_dirs(&cfg)
        .iter()
        .any(|dir| dir.contains("surge")));
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
    // 組み込みの名前も config の名前も、両方を示す。
    assert!(message.contains("Dexed"));
    assert!(message.contains("Surge XT"));
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

const MINIMAL_CONFIG: &str = r#"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#;

/// ユーザーが最初に書く形。`[plugins.*]` が 1 つも無くても動くことがこの機能の要。
#[test]
fn a_builtin_name_alone_needs_no_plugins_table() {
    let cfg = load_from_toml(&format!("active_plugin = 'Dexed'\n{MINIMAL_CONFIG}")).unwrap();

    assert_eq!(cfg.plugin_path, crate::default_dexed_plugin_path());
    assert_eq!(cfg.plugin_id.as_deref(), Some("com.digital-suburban.dexed"));
    // Dexed が factory cartridge を展開する場所が組み込みで入る。
    assert_eq!(
        configured_patch_dirs(&cfg),
        crate::default_dexed_cartridge_dirs()
    );
}

#[test]
fn the_builtin_surge_profile_brings_its_patch_directories() {
    let cfg = load_from_toml(&format!("active_plugin = 'Surge XT'\n{MINIMAL_CONFIG}")).unwrap();

    assert_eq!(cfg.plugin_path, crate::default_plugin_path());
    assert_eq!(
        cfg.plugin_id.as_deref(),
        Some("org.surge-synth-team.surge-xt")
    );
    assert_eq!(configured_patch_dirs(&cfg), crate::default_patches_dirs());
}

/// 大文字小文字・空白・アンダースコアの違いで起動できなくなるのは事故のもと。
#[test]
fn builtin_names_ignore_case_spaces_and_underscores() {
    for name in ["dexed", "DEXED", "De xed"] {
        let cfg = load_from_toml(&format!("active_plugin = '{name}'\n{MINIMAL_CONFIG}")).unwrap();
        assert_eq!(cfg.plugin_id.as_deref(), Some("com.digital-suburban.dexed"));
    }
    for name in ["surge_xt", "surge xt", "SurgeXT"] {
        let cfg = load_from_toml(&format!("active_plugin = '{name}'\n{MINIMAL_CONFIG}")).unwrap();
        assert_eq!(
            cfg.plugin_id.as_deref(),
            Some("org.surge-synth-team.surge-xt")
        );
    }
}

/// 標準以外の場所に入れている人は plugin_path だけ書けばよく、
/// plugin_id や patches_dirs を書き写す必要はない。
#[test]
fn a_configured_profile_overrides_only_the_fields_it_writes() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'Surge XT'\n{MINIMAL_CONFIG}\n\
         [plugins.\"Surge XT\"]\nplugin_path = '/opt/clap/Surge XT.clap'\n"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/opt/clap/Surge XT.clap");
    assert_eq!(
        cfg.plugin_id.as_deref(),
        Some("org.surge-synth-team.surge-xt")
    );
    assert_eq!(configured_patch_dirs(&cfg), crate::default_patches_dirs());
}

/// 組み込みの `patches_dirs` を消したいときは、明示的に空配列を書く。
#[test]
fn an_empty_patches_dirs_clears_the_builtin_ones() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'Surge XT'\n{MINIMAL_CONFIG}\n\
         [plugins.\"Surge XT\"]\npatches_dirs = []\n"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, crate::default_plugin_path());
    assert!(configured_patch_dirs(&cfg).is_empty());
}

/// 組み込みと同名の profile を config に書いても、既存の書き方（全項目を書く）は壊れない。
#[test]
fn a_fully_written_profile_still_wins_over_the_builtin() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'Dexed'\n{MINIMAL_CONFIG}\n\
         [plugins.Dexed]\nplugin_path = '/opt/clap/Dexed.clap'\nplugin_id = 'custom.dexed'\n"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/opt/clap/Dexed.clap");
    assert_eq!(cfg.plugin_id.as_deref(), Some("custom.dexed"));
}
