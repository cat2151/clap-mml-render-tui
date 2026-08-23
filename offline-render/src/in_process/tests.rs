use super::*;

use cmrt_runtime::{PatchRoles, DEXED_PLUGIN_ID, FLOE_PLUGIN_ID, SURGE_XT_PLUGIN_ID};

/// 2 プラグインぶんのカタログ。開発機のインストール状況に左右されないよう、
/// `catalog_plugins()` を通さずに手で並べる。
fn catalog_entry(name: &str, plugin_id: &str, base: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: Some(base.to_string()),
        dirs: vec![base.to_string()],
        resolved_patches: None,
        source_notices: Vec::new(),
        patch_roles: PatchRoles::default(),
    }
}

fn surge_catalog_entry() -> CatalogPlugin {
    catalog_entry("Surge XT", SURGE_XT_PLUGIN_ID, "/data/Surge XT")
}

fn dexed_catalog_entry() -> CatalogPlugin {
    catalog_entry("Dexed", DEXED_PLUGIN_ID, "/data/Dexed/Cartridges")
}

fn floe_catalog_entry() -> CatalogPlugin {
    catalog_entry("Floe", FLOE_PLUGIN_ID, "/data/Floe/presets")
}

fn config_for_test() -> Config {
    toml::from_str(
        r#"
plugin_path = "/clap/Surge XT.clap"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#,
    )
    .unwrap()
}

fn mml_with_patch(patch: &str) -> String {
    format!("{{\"Surge XT patch\":\"{patch}\"}}cde")
}

fn plugins_from(catalog: Vec<CatalogPlugin>) -> InProcessPlugins {
    InProcessPlugins::from_catalog(&config_for_test(), catalog, &PluginEntries::none())
}

/// cartridge 形式（`.syx` を含む）を指した MML は Dexed 側へ引く。
/// ここを間違えると Surge のインスタンスへ DX7 の SysEx が渡り、
/// Surge は理解できない 163 byte を**黙って無視する**（`docs/adr/0009-offline-entry-map.md`）。
#[test]
fn cartridge_patch_selects_the_cartridge_plugin() {
    let plugins = plugins_from(vec![surge_catalog_entry(), dexed_catalog_entry()]);

    let index = plugins.index_for_mml(&mml_with_patch("Dexed_01.syx/00 Say Again."));

    assert_eq!(index, 1);
}

/// state file 形式（`.fxp`）を指した MML は Surge 側へ引く。既定が Dexed でも変わらない。
#[test]
fn state_file_patch_selects_the_state_file_plugin() {
    let plugins = plugins_from(vec![dexed_catalog_entry(), surge_catalog_entry()]);

    let index = plugins.index_for_mml(&mml_with_patch("patches_factory/Basses/Init Saw.fxp"));

    assert_eq!(index, 1);
}

/// 音色を無指定にした MML は**常に既定プラグイン**（`docs/adr/0004-default-plugin-owns-unspecified-patches.md`）。
/// patch 文字列の形で引くと空文字列が「cartridge ではない」と判定され、既定が Dexed でも
/// Surge 側へ飛んでしまう。cache 名前空間の `OnceLock` が無改修で正しい根拠でもある。
#[test]
fn mml_without_a_patch_selects_the_default_plugin() {
    let plugins = plugins_from(vec![dexed_catalog_entry(), surge_catalog_entry()]);

    assert_eq!(plugins.index_for_mml("cde"), 0);
    assert_eq!(plugins.index_for_mml("{\"bpm\":120}cde"), 0);
}

/// 相対化の基点と `plugin_id` はプラグインごとに違う。別プラグインの base で
/// MML 先頭 JSON のパスを解決すると、存在しないファイルを掴んで音色が当たらない。
#[test]
fn each_plugin_renders_with_its_own_base_and_plugin_id() {
    let plugins = plugins_from(vec![surge_catalog_entry(), dexed_catalog_entry()]);

    assert_eq!(
        plugins.core_cfg(0).patches_dir.as_deref(),
        Some("/data/Surge XT")
    );
    assert_eq!(
        plugins.core_cfg(0).plugin_id.as_deref(),
        Some(SURGE_XT_PLUGIN_ID)
    );
    assert_eq!(
        plugins.core_cfg(1).patches_dir.as_deref(),
        Some("/data/Dexed/Cartridges")
    );
    assert_eq!(
        plugins.core_cfg(1).plugin_id.as_deref(),
        Some(DEXED_PLUGIN_ID)
    );
}

/// prepare 時のカタログとレンダー時のカタログがずれても、範囲外の添字で落ちない。
#[test]
fn out_of_range_plugin_index_falls_back_to_the_default_plugin() {
    let plugins = plugins_from(vec![surge_catalog_entry(), dexed_catalog_entry()]);

    assert_eq!(
        plugins.core_cfg(9).patches_dir,
        plugins.core_cfg(0).patches_dir
    );
}

/// entry が無ければ in-process では鳴らせない。null ポインタを踏まずにエラーで返す。
#[test]
fn a_missing_entry_is_an_error_instead_of_a_null_dereference() {
    let plugins = plugins_from(vec![surge_catalog_entry()]);

    let error = plugins
        .for_mml("cde")
        .err()
        .expect("entry が無い構成では in-process レンダリングできない");

    assert!(error.to_string().contains("PluginEntry"), "{error}");
}

#[test]
fn floe_preset_selects_the_floe_entry_and_base() {
    let plugins = plugins_from(vec![
        surge_catalog_entry(),
        dexed_catalog_entry(),
        floe_catalog_entry(),
    ]);

    let index = plugins.index_for_mml(&mml_with_patch(
        "Celtic Harp Factory Presets/Realistic Celtic Harp.floe-preset",
    ));

    assert_eq!(index, 2);
    assert_eq!(
        plugins.core_cfg(index).plugin_id.as_deref(),
        Some(FLOE_PLUGIN_ID)
    );
    assert_eq!(
        plugins.core_cfg(index).patches_dir.as_deref(),
        Some("/data/Floe/presets")
    );
}
