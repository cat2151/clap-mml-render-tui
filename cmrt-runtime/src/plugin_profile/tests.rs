//! 固定の Surge XT profile を [`Config`] の runtime field へ焼き込むテスト。

use super::*;
use crate::{configured_patch_dirs, SURGE_XT_PLUGIN_ID};

const MINIMAL_CONFIG: &str = r#"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#;

fn load_from_toml(toml_str: &str) -> anyhow::Result<Config> {
    Config::from_toml_str(toml_str)
}

#[test]
fn the_builtin_surge_profile_is_baked_into_runtime_fields() {
    let cfg = load_from_toml(MINIMAL_CONFIG).unwrap();

    assert_eq!(cfg.plugin_path, crate::default_plugin_path());
    assert_eq!(cfg.plugin_id.as_deref(), Some(SURGE_XT_PLUGIN_ID));
    assert_eq!(configured_patch_dirs(&cfg), crate::default_patches_dirs());
}

#[test]
fn a_surge_profile_overrides_the_runtime_fields() {
    let cfg = load_from_toml(&format!(
        r#"{MINIMAL_CONFIG}
[plugins."Surge XT"]
plugin_path = "/clap/Surge XT.clap"
patches_dirs = ["/surge/patches_factory", "/surge/patches_3rdparty"]
"#
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/clap/Surge XT.clap");
    assert_eq!(cfg.plugin_id.as_deref(), Some(SURGE_XT_PLUGIN_ID));
    assert_eq!(
        configured_patch_dirs(&cfg),
        [
            "/surge/patches_factory".to_string(),
            "/surge/patches_3rdparty".to_string()
        ]
    );
}

#[test]
fn active_plugin_is_rejected_even_when_it_names_surge_xt() {
    let error =
        load_from_toml(&format!("active_plugin = 'Surge XT'\n{MINIMAL_CONFIG}")).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("active_plugin"), "{message}");
    assert!(message.contains(r#"[plugins."Surge XT"]"#), "{message}");
}

#[test]
fn top_level_plugin_settings_are_rejected() {
    let error = load_from_toml(&format!(
        "plugin_path = '/clap/Surge XT.clap'\npatches_dirs = ['/surge']\n{MINIMAL_CONFIG}"
    ))
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("plugin_path"), "{message}");
    assert!(message.contains("patches_dirs"), "{message}");
}

#[test]
fn another_profile_does_not_replace_the_primary_plugin() {
    let cfg = load_from_toml(&format!(
        r#"{MINIMAL_CONFIG}
[plugins.Dexed]
plugin_path = "/clap/Dexed.clap"
plugin_id = "com.digital-suburban.dexed"
patches_dirs = ["/dexed/cartridges"]
"#
    ))
    .unwrap();

    assert_eq!(cfg.plugin_id.as_deref(), Some(SURGE_XT_PLUGIN_ID));
    assert_eq!(cfg.plugin_path, crate::default_plugin_path());
}

#[test]
fn retired_patch_role_overrides_are_rejected() {
    let error = load_from_toml(&format!(
        r#"{MINIMAL_CONFIG}
[plugins."Surge XT"]
chord_patch_categories = ["MyPads"]
kick_patch_keywords = ["thump"]
"#
    ))
    .unwrap_err();

    assert!(format!("{error:#}").contains("chord_patch_categories"));
}
