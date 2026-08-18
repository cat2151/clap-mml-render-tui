use super::*;
use crate::default_dexed_plugin_path;

fn config_with(plugin_id: Option<&str>, plugin_path: &str) -> Config {
    Config {
        plugin_id: plugin_id.map(str::to_string),
        plugin_path: plugin_path.to_string(),
        ..Config::default()
    }
}

#[test]
fn plugin_id_decides_when_it_is_present() {
    assert!(config_with(Some(SURGE_XT_PLUGIN_ID), "").is_surge_xt());
    assert!(!config_with(Some(DEXED_PLUGIN_ID), default_plugin_path()).is_surge_xt());
}

#[test]
fn plugin_path_decides_when_plugin_id_is_absent() {
    // active_plugin が無かった時代の config。Surge 専用だった。
    assert!(config_with(None, default_plugin_path()).is_surge_xt());
    assert!(!config_with(None, default_dexed_plugin_path()).is_surge_xt());
    assert!(!config_with(None, "").is_surge_xt());
}

#[test]
fn file_stem_drops_directory_and_extension() {
    assert_eq!(
        plugin_file_stem(r"C:\Program Files\Common Files\CLAP\Surge Synth Team\Surge XT.clap"),
        "Surge XT"
    );
    assert_eq!(
        plugin_file_stem("/Library/Audio/Plug-Ins/CLAP/Dexed.clap"),
        "Dexed"
    );
    assert_eq!(plugin_file_stem("  "), "");
}
