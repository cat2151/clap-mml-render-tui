//! プロファイルの**焼き込み**（[`Config`] のトップレベルフィールドへの反映）のテスト。
//!
//! 解決規則そのもの（組み込みプロファイルの中身・名前の表記ゆれ吸収・config 側での
//! 上書き・エラーメッセージ）は `cmrt_server_config` 側のテストが持つ。ここで重ねない。

use super::*;
use crate::configured_patch_dirs;

/// `toml::from_str` は profile を解決しないので、`Config::load()` と同じく
/// `apply_active_plugin_profile()` を明示的に通す。
fn load_from_toml(toml_str: &str) -> anyhow::Result<Config> {
    let mut cfg: Config = toml::from_str(toml_str)?;
    apply_active_plugin_profile(&mut cfg)?;
    Ok(cfg)
}

const MINIMAL_CONFIG: &str = r#"
input_midi  = "input.mid"
output_midi = "output.mid"
output_wav  = "output.wav"
sample_rate = 48000
buffer_size = 512
"#;

#[test]
fn a_config_without_active_plugin_keeps_its_top_level_settings() {
    let cfg = load_from_toml(&format!(
        "plugin_path = '/usr/lib/clap/Surge XT.clap'\n\
         patches_dirs = ['/surge/patches_factory']\n{MINIMAL_CONFIG}"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/usr/lib/clap/Surge XT.clap");
    assert_eq!(
        cfg.patches_dirs.as_deref(),
        Some(["/surge/patches_factory".to_string()].as_slice())
    );
    assert_eq!(cfg.plugin_id, None);
}

/// 解決したプロファイルの 3 項目がトップレベルへ入ること。読み手（app・各画面）は
/// プロファイルを知らずに `cfg.plugin_path` / `cfg.patches_dirs` だけを見る。
#[test]
fn an_active_profile_is_baked_into_the_top_level_fields() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'my_synth'\nplugin_path = '/clap/Surge XT.clap'\n\
         patches_dirs = ['/surge/patches_factory']\n{MINIMAL_CONFIG}\n\
         [plugins.my_synth]\nplugin_path = '/clap/MySynth.clap'\n\
         plugin_id = 'com.example.my-synth'\npatches_dirs = ['/my/patches', '/my/more']\n"
    ))
    .unwrap();

    assert_eq!(cfg.plugin_path, "/clap/MySynth.clap");
    assert_eq!(cfg.plugin_id.as_deref(), Some("com.example.my-synth"));
    assert_eq!(
        configured_patch_dirs(&cfg),
        ["/my/patches".to_string(), "/my/more".to_string()]
    );
}

/// 解決エラーが呼び出し側まで伝わること（`Config::load` はこれを config.toml の
/// パスつきで包み直す）。メッセージの中身は共有 crate 側のテストが見る。
#[test]
fn a_resolution_error_reaches_the_caller() {
    let error = load_from_toml(&format!("active_plugin = 'nope'\n{MINIMAL_CONFIG}")).unwrap_err();

    assert!(error.to_string().contains("nope"));
}

/// 用途別絞り込みは焼き込まれない。解決した値を持つのは [`crate::catalog_plugins`]。
fn resolved_roles(cfg: &Config) -> crate::PatchRoles {
    crate::catalog_plugins(cfg).remove(0).patch_roles
}

/// Dexed の cartridge には Surge のようなカテゴリ階層が無いので、組み込みプロファイルは
/// 用途別の絞り込みを全て外す。ここが効かないと chord / bass / drum 行の候補が 0 件になる。
#[test]
fn the_builtin_dexed_profile_does_not_narrow_the_patch_roles() {
    let cfg = load_from_toml(&format!("active_plugin = 'Dexed'\n{MINIMAL_CONFIG}")).unwrap();

    assert_eq!(resolved_roles(&cfg), crate::PatchRoles::default());
}

/// トップレベルの値は「プロファイルが書いていない項目の土台」なので、絞らない
/// プロファイルを選んでも消えない。混在カタログで Surge の音色へ当て直すための土台。
#[test]
fn the_top_level_values_survive_a_profile_that_narrows_nothing() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'Dexed'\nchord_patch_categories = ['MyPads']\n{MINIMAL_CONFIG}"
    ))
    .unwrap();

    assert_eq!(
        cfg.top_level_patch_roles.chord_patch_categories,
        Some(vec!["MyPads".to_string()])
    );
    assert!(resolved_roles(&cfg).chord_patch_categories.is_empty());
}

/// Surge のプロファイルは絞り込みを書かないので、トップレベルの設定がそのまま残る。
/// 既存 config を持つ Surge ユーザーの挙動が変わらないことの担保。
#[test]
fn the_surge_profile_keeps_the_top_level_patch_categories() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'Surge XT'\nchord_patch_categories = ['MyPads']\n\
         kick_patch_keywords = ['thump']\n{MINIMAL_CONFIG}"
    ))
    .unwrap();

    let roles = resolved_roles(&cfg);
    assert_eq!(roles.chord_patch_categories, ["MyPads".to_string()]);
    assert_eq!(roles.kick_patch_keywords, ["thump".to_string()]);
    assert_eq!(
        roles.bass_patch_categories,
        crate::PatchRoles::builtin_for(Some(crate::SURGE_XT_PLUGIN_ID), "").bass_patch_categories
    );
}

/// プロファイル側にカテゴリを書けば、そのプラグインだけ絞り込める。
/// トップレベル（＝ Surge 用）とプラグイン用を同じ config に共存させるための経路。
#[test]
fn a_profile_can_narrow_the_patch_roles_by_itself() {
    let cfg = load_from_toml(&format!(
        "active_plugin = 'Dexed'\nchord_patch_categories = ['Pads']\n{MINIMAL_CONFIG}\n\
         [plugins.Dexed]\nchord_patch_categories = ['SynprezFM']\n"
    ))
    .unwrap();

    let roles = resolved_roles(&cfg);
    assert_eq!(roles.chord_patch_categories, ["SynprezFM".to_string()]);
    // 書かなかった項目は組み込み Dexed プロファイルの「絞らない」が残る。
    assert!(roles.bass_patch_categories.is_empty());
}
