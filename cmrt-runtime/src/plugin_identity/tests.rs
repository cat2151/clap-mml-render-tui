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

/// Vaporizer2 の同定。`plugin_id` があればそれで決まる。
#[test]
fn vaporizer2_is_decided_by_its_plugin_id() {
    assert!(is_vaporizer2_plugin(Some(VAPORIZER2_PLUGIN_ID), ""));
    assert!(!is_vaporizer2_plugin(
        Some(SURGE_XT_PLUGIN_ID),
        "VASTvaporizer2.clap"
    ));
    assert!(!is_vaporizer2_plugin(Some(DEXED_PLUGIN_ID), ""));
}

/// `plugin_id` が無ければファイル名で見る。**既定 `plugin_path` との一致ではない。**
/// カタログの 2 つめ以降として現れるプラグインは、config の `plugin_path`
/// （＝既定プラグインのパス）とは別物なので、一致比較にすると当たらない。
#[test]
fn vaporizer2_is_decided_by_its_file_name_when_the_id_is_absent() {
    assert!(is_vaporizer2_plugin(
        None,
        r"C:\Program Files\Common Files\CLAP\VASTvaporizer2.clap"
    ));
    assert!(is_vaporizer2_plugin(None, "/opt/clap/Vaporizer2.clap"));
    assert!(!is_vaporizer2_plugin(None, default_plugin_path()));
    assert!(!is_vaporizer2_plugin(None, default_dexed_plugin_path()));
    assert!(!is_vaporizer2_plugin(None, ""));
}

/// Surge 判定が Vaporizer2 を巻き込まない（両方 `.fxp` ではなく別の形を扱う）。
#[test]
fn the_surge_test_does_not_claim_vaporizer2() {
    assert!(!config_with(Some(VAPORIZER2_PLUGIN_ID), "").is_surge_xt());
    assert!(!config_with(None, "VASTvaporizer2.clap").is_surge_xt());
}
